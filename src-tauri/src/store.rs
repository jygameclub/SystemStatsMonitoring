use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{DeviceInfo, LocalDataStats, MetricSample};
use crate::settings::{AppSettings, MetricSettings, S3SyncSettings};

pub struct Store {
    conn: Connection,
    db_path: Option<PathBuf>,
}

impl Store {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create database directory {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("open SQLite database {}", path.display()))?;

        Ok(Self {
            conn,
            db_path: Some(path.to_path_buf()),
        })
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            conn: Connection::open_in_memory().context("open in-memory SQLite database")?,
            db_path: None,
        })
    }

    pub fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              os TEXT NOT NULL,
              arch TEXT NOT NULL,
              agent_version TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              last_seen_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS metric_samples (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              device_id TEXT NOT NULL,
              ts INTEGER NOT NULL,
              cpu_usage REAL,
              memory_used INTEGER,
              memory_total INTEGER,
              disk_used INTEGER,
              disk_total INTEGER,
              network_rx REAL,
              network_tx REAL,
              gpu_usage REAL,
              gpu_memory_total INTEGER,
              gpu_name TEXT,
              temperature_celsius REAL,
              power_watts REAL,
              sensors_json TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_metric_samples_device_ts
            ON metric_samples(device_id, ts);

            CREATE TABLE IF NOT EXISTS app_settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );
            "#,
        )?;

        self.migrate_metric_samples_nullable()?;
        self.add_missing_metric_sample_columns()?;

        Ok(())
    }

    pub fn save_metric_sample(&self, sample: &MetricSample) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO metric_samples (
              device_id,
              ts,
              cpu_usage,
              memory_used,
              memory_total,
              disk_used,
              disk_total,
              network_rx,
              network_tx,
              gpu_usage,
              gpu_memory_total,
              gpu_name,
              temperature_celsius,
              power_watts,
              sensors_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                sample.device_id,
                sample.ts,
                sample.cpu_usage,
                sample.memory_used.map(|value| value as i64),
                sample.memory_total.map(|value| value as i64),
                sample.disk_used.map(|value| value as i64),
                sample.disk_total.map(|value| value as i64),
                sample.network_rx,
                sample.network_tx,
                sample.gpu_usage,
                sample.gpu_memory_total.map(|value| value as i64),
                sample.gpu_name,
                sample.temperature_celsius,
                sample.power_watts,
                serde_json::to_string(&sample.sensor_readings)
                    .context("serialize sensor readings")?
            ],
        )?;

        Ok(())
    }

    pub fn upsert_metric_sample(&self, sample: &MetricSample) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM metric_samples WHERE device_id = ?1 AND ts = ?2",
                params![sample.device_id, sample.ts],
            )
            .context("delete existing metric sample before upsert")?;

        self.save_metric_sample(sample)
    }

    pub fn import_metric_samples(&self, samples: &[MetricSample]) -> Result<usize> {
        for sample in samples {
            self.upsert_metric_sample(sample)?;
        }

        Ok(samples.len())
    }

    pub fn metric_history(
        &self,
        device_id: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<MetricSample>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
              id,
              device_id,
              ts,
              cpu_usage,
              memory_used,
              memory_total,
              disk_used,
              disk_total,
              network_rx,
              network_tx,
              gpu_usage,
              gpu_memory_total,
              gpu_name,
              temperature_celsius,
              power_watts,
              sensors_json
            FROM metric_samples
            WHERE device_id = ?1
              AND ts >= ?2
              AND ts <= ?3
            ORDER BY ts ASC
            "#,
        )?;

        let rows = stmt.query_map(params![device_id, start_ts, end_ts], |row| {
            Ok(MetricSample {
                id: row.get(0)?,
                device_id: row.get(1)?,
                ts: row.get(2)?,
                cpu_usage: row.get(3)?,
                memory_used: row.get::<_, Option<i64>>(4)?.map(|value| value as u64),
                memory_total: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                disk_used: row.get::<_, Option<i64>>(6)?.map(|value| value as u64),
                disk_total: row.get::<_, Option<i64>>(7)?.map(|value| value as u64),
                network_rx: row.get(8)?,
                network_tx: row.get(9)?,
                gpu_usage: row.get(10)?,
                gpu_memory_total: row.get::<_, Option<i64>>(11)?.map(|value| value as u64),
                gpu_name: row.get(12)?,
                temperature_celsius: row.get(13)?,
                power_watts: row.get(14)?,
                sensor_readings: row
                    .get::<_, Option<String>>(15)?
                    .and_then(|raw| serde_json::from_str(&raw).ok())
                    .unwrap_or_default(),
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("collect metric history rows")
    }

    pub fn device_metric_history(
        &self,
        device_id: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<MetricSample>> {
        self.metric_history(device_id, start_ts, end_ts)
    }

    pub fn upsert_device(&self, device: &DeviceInfo, ts: i64) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO devices (
              id,
              name,
              os,
              arch,
              agent_version,
              created_at,
              last_seen_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ON CONFLICT(id) DO UPDATE SET
              name = excluded.name,
              os = excluded.os,
              arch = excluded.arch,
              agent_version = excluded.agent_version,
              last_seen_at = excluded.last_seen_at
            "#,
            params![
                device.id,
                device.name,
                device.os,
                device.arch,
                device.agent_version,
                ts
            ],
        )?;

        Ok(())
    }

    pub fn devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, os, arch, agent_version
            FROM devices
            ORDER BY last_seen_at DESC, name ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(DeviceInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                os: row.get(2)?,
                arch: row.get(3)?,
                agent_version: row.get(4)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("collect device rows")
    }

    pub fn prune_metric_samples_before(&self, cutoff_ts: i64) -> Result<usize> {
        self.conn
            .execute(
                "DELETE FROM metric_samples WHERE ts < ?1",
                params![cutoff_ts],
            )
            .context("delete old metric samples")
    }

    pub fn clear_metric_samples(&self) -> Result<usize> {
        let deleted = self
            .conn
            .execute("DELETE FROM metric_samples", [])
            .context("delete all metric samples")?;

        if self.db_path.is_some() {
            self.conn.execute_batch("VACUUM")?;
        }

        Ok(deleted)
    }

    pub fn local_data_stats(&self) -> Result<LocalDataStats> {
        let sample_count: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM metric_samples", [], |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count as u64)
                })?;

        let (database_path, database_size_bytes) = match &self.db_path {
            Some(path) => {
                let size = std::fs::metadata(path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                (path.display().to_string(), size)
            }
            None => ("In-memory SQLite".to_string(), 0),
        };

        Ok(LocalDataStats {
            database_path,
            database_size_bytes,
            sample_count,
        })
    }

    pub fn get_settings(&self) -> Result<AppSettings> {
        let sample_interval_sec = self
            .get_setting_u64("sample_interval_sec")?
            .unwrap_or(AppSettings::default().sample_interval_sec);
        let local_save_interval_sec = self
            .get_setting_u64("local_save_interval_sec")?
            .unwrap_or(AppSettings::default().local_save_interval_sec);
        let language = self
            .get_setting_string("language")?
            .unwrap_or(AppSettings::default().language);
        let machine_name = self
            .get_setting_string("machine_name")?
            .unwrap_or(AppSettings::default().machine_name);
        let default_metrics = MetricSettings::default();
        let metrics = MetricSettings {
            cpu: self
                .get_setting_bool("metric_cpu")?
                .unwrap_or(default_metrics.cpu),
            memory: self
                .get_setting_bool("metric_memory")?
                .unwrap_or(default_metrics.memory),
            disk: self
                .get_setting_bool("metric_disk")?
                .unwrap_or(default_metrics.disk),
            network: self
                .get_setting_bool("metric_network")?
                .unwrap_or(default_metrics.network),
            gpu: self
                .get_setting_bool("metric_gpu")?
                .unwrap_or(default_metrics.gpu),
            temperature: self
                .get_setting_bool("metric_temperature")?
                .unwrap_or(default_metrics.temperature),
            power: self
                .get_setting_bool("metric_power")?
                .unwrap_or(default_metrics.power),
            battery: self
                .get_setting_bool("metric_battery")?
                .unwrap_or(default_metrics.battery),
        };
        let default_s3 = S3SyncSettings::default();
        let s3_sync = S3SyncSettings {
            enabled: self
                .get_setting_bool("s3_sync_enabled")?
                .unwrap_or(default_s3.enabled),
            endpoint_url: self
                .get_setting_string("s3_endpoint_url")?
                .unwrap_or(default_s3.endpoint_url),
            region: self
                .get_setting_string("s3_region")?
                .unwrap_or(default_s3.region),
            bucket: self
                .get_setting_string("s3_bucket")?
                .unwrap_or(default_s3.bucket),
            access_key_id: self
                .get_setting_string("s3_access_key_id")?
                .unwrap_or(default_s3.access_key_id),
            secret_access_key: self
                .get_setting_string("s3_secret_access_key")?
                .unwrap_or(default_s3.secret_access_key),
            prefix: self
                .get_setting_string("s3_prefix")?
                .unwrap_or(default_s3.prefix),
            sync_interval_min: self
                .get_setting_u64("s3_sync_interval_min")?
                .unwrap_or(default_s3.sync_interval_min),
            path_style: self
                .get_setting_bool("s3_path_style")?
                .unwrap_or(default_s3.path_style),
        };

        let settings = AppSettings {
            sample_interval_sec,
            local_save_interval_sec,
            machine_name,
            language,
            metrics,
            s3_sync,
        };
        settings.validate()?;

        Ok(settings)
    }

    pub fn update_settings(&self, settings: &AppSettings) -> Result<AppSettings> {
        settings.validate()?;
        self.set_setting(
            "sample_interval_sec",
            &settings.sample_interval_sec.to_string(),
        )?;
        self.set_setting(
            "local_save_interval_sec",
            &settings.local_save_interval_sec.to_string(),
        )?;
        self.set_setting("machine_name", settings.machine_name.trim())?;
        self.set_setting("language", &settings.language)?;
        self.set_setting("metric_cpu", bool_to_setting(settings.metrics.cpu))?;
        self.set_setting("metric_memory", bool_to_setting(settings.metrics.memory))?;
        self.set_setting("metric_disk", bool_to_setting(settings.metrics.disk))?;
        self.set_setting("metric_network", bool_to_setting(settings.metrics.network))?;
        self.set_setting("metric_gpu", bool_to_setting(settings.metrics.gpu))?;
        self.set_setting(
            "metric_temperature",
            bool_to_setting(settings.metrics.temperature),
        )?;
        self.set_setting("metric_power", bool_to_setting(settings.metrics.power))?;
        self.set_setting("metric_battery", bool_to_setting(settings.metrics.battery))?;
        self.set_setting("s3_sync_enabled", bool_to_setting(settings.s3_sync.enabled))?;
        self.set_setting("s3_endpoint_url", settings.s3_sync.endpoint_url.trim())?;
        self.set_setting("s3_region", settings.s3_sync.region.trim())?;
        self.set_setting("s3_bucket", settings.s3_sync.bucket.trim())?;
        self.set_setting("s3_access_key_id", settings.s3_sync.access_key_id.trim())?;
        self.set_setting(
            "s3_secret_access_key",
            settings.s3_sync.secret_access_key.trim(),
        )?;
        self.set_setting("s3_prefix", settings.s3_sync.prefix.trim())?;
        self.set_setting(
            "s3_sync_interval_min",
            &settings.s3_sync.sync_interval_min.to_string(),
        )?;
        self.set_setting(
            "s3_path_style",
            bool_to_setting(settings.s3_sync.path_style),
        )?;

        Ok(settings.clone())
    }

    fn get_setting_u64(&self, key: &str) -> Result<Option<u64>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;

        match value {
            Some(value) => value
                .parse::<u64>()
                .map(Some)
                .with_context(|| format!("parse setting {key} as u64")),
            None => Ok(None),
        }
    }

    fn get_setting_string(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context("read string setting")
    }

    fn get_setting_bool(&self, key: &str) -> Result<Option<bool>> {
        let value = self.get_setting_string(key)?;

        match value.as_deref() {
            Some("true") => Ok(Some(true)),
            Some("false") => Ok(Some(false)),
            Some(other) => bail!("setting {key} must be true or false, got {other}"),
            None => Ok(None),
        }
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        if key.trim().is_empty() {
            bail!("setting key cannot be empty");
        }

        self.conn.execute(
            r#"
            INSERT INTO app_settings (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, value],
        )?;

        Ok(())
    }

    fn migrate_metric_samples_nullable(&self) -> Result<()> {
        if !self.metric_samples_need_nullable_migration()? {
            return Ok(());
        }

        self.conn.execute_batch(
            r#"
            DROP INDEX IF EXISTS idx_metric_samples_device_ts;
            ALTER TABLE metric_samples RENAME TO metric_samples_old;

            CREATE TABLE metric_samples (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              device_id TEXT NOT NULL,
              ts INTEGER NOT NULL,
              cpu_usage REAL,
              memory_used INTEGER,
              memory_total INTEGER,
              disk_used INTEGER,
              disk_total INTEGER,
              network_rx REAL,
              network_tx REAL,
              gpu_usage REAL,
              gpu_memory_total INTEGER,
              gpu_name TEXT,
              temperature_celsius REAL,
              power_watts REAL,
              sensors_json TEXT
            );

            INSERT INTO metric_samples (
              id,
              device_id,
              ts,
              cpu_usage,
              memory_used,
              memory_total,
              disk_used,
              disk_total,
              network_rx,
              network_tx
            )
            SELECT
              id,
              device_id,
              ts,
              cpu_usage,
              memory_used,
              memory_total,
              disk_used,
              disk_total,
              network_rx,
              network_tx
            FROM metric_samples_old;

            DROP TABLE metric_samples_old;

            CREATE INDEX IF NOT EXISTS idx_metric_samples_device_ts
            ON metric_samples(device_id, ts);
            "#,
        )?;

        Ok(())
    }

    fn metric_samples_need_nullable_migration(&self) -> Result<bool> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(metric_samples)")?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            let not_null: i64 = row.get(3)?;
            Ok((name, not_null))
        })?;

        for row in rows {
            let (name, not_null) = row?;
            if matches!(
                name.as_str(),
                "cpu_usage"
                    | "memory_used"
                    | "memory_total"
                    | "disk_used"
                    | "disk_total"
                    | "network_rx"
                    | "network_tx"
                    | "gpu_usage"
                    | "gpu_memory_total"
                    | "temperature_celsius"
                    | "power_watts"
            ) && not_null != 0
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
    fn add_missing_metric_sample_columns(&self) -> Result<()> {
        let columns = self.metric_sample_column_names()?;

        for (name, sql_type) in [
            ("gpu_usage", "REAL"),
            ("gpu_memory_total", "INTEGER"),
            ("gpu_name", "TEXT"),
            ("temperature_celsius", "REAL"),
            ("power_watts", "REAL"),
            ("sensors_json", "TEXT"),
        ] {
            if !columns.contains(name) {
                self.conn.execute(
                    &format!("ALTER TABLE metric_samples ADD COLUMN {name} {sql_type}"),
                    [],
                )?;
            }
        }

        Ok(())
    }

    fn metric_sample_column_names(&self) -> Result<HashSet<String>> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(metric_samples)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

        rows.collect::<rusqlite::Result<HashSet<_>>>()
            .context("collect metric sample columns")
    }
}

fn bool_to_setting(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
