pub mod metrics;
pub mod models;
pub mod s3_sync;
pub mod settings;
pub mod store;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use metrics::MetricCollector;
use models::{DeviceInfo, HistoryQuery, LocalDataStats, MetricSample, S3SyncReport};
use settings::AppSettings;
use store::Store;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};

const HISTORY_RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1000;

pub struct AppState {
    store: Mutex<Store>,
    collector: Mutex<MetricCollector>,
    device: Mutex<DeviceInfo>,
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
fn get_device_info(state: State<'_, AppState>) -> DeviceInfo {
    state
        .device
        .lock()
        .map(|device| device.clone())
        .unwrap_or_else(|_| DeviceInfo::current())
}

#[tauri::command]
fn get_latest_metrics(state: State<'_, AppState>) -> CommandResult<MetricSample> {
    let mut collector = state
        .collector
        .lock()
        .map_err(|_| "指标采集器锁定失败".to_string())?;

    collector
        .sample()
        .map_err(|error| format!("指标采集失败: {error}"))
}

#[tauri::command]
fn save_metric_sample(state: State<'_, AppState>, sample: MetricSample) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;

    store
        .save_metric_sample(&sample)
        .map_err(|error| format!("保存历史采样失败: {error}"))?;

    let cutoff = history_retention_cutoff(sample.ts);
    store
        .prune_metric_samples_before(cutoff)
        .map_err(|error| format!("清理历史采样失败: {error}"))?;

    Ok(())
}

fn history_retention_cutoff(ts: i64) -> i64 {
    ts - HISTORY_RETENTION_MS
}

#[tauri::command]
fn get_metric_history(
    state: State<'_, AppState>,
    query: HistoryQuery,
) -> CommandResult<Vec<MetricSample>> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let device_id = match query.device_id {
        Some(device_id) => device_id,
        None => state
            .device
            .lock()
            .map_err(|_| "设备信息锁定失败".to_string())?
            .id
            .clone(),
    };

    store
        .metric_history(&device_id, query.start_ts, query.end_ts)
        .map_err(|error| format!("读取历史采样失败: {error}"))
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> CommandResult<AppSettings> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;

    store
        .get_settings()
        .map_err(|error| format!("读取设置失败: {error}"))
}

#[tauri::command]
fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> CommandResult<AppSettings> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;

    let saved = store
        .update_settings(&settings)
        .map_err(|error| format!("保存设置失败: {error}"))?;

    let next_device = DeviceInfo::current_with_machine_name(&saved.machine_name);
    store
        .upsert_device(&next_device, now_ms())
        .map_err(|error| format!("保存设备信息失败: {error}"))?;

    state
        .collector
        .lock()
        .map_err(|_| "指标采集器锁定失败".to_string())?
        .set_device_id(next_device.id.clone());
    *state
        .device
        .lock()
        .map_err(|_| "设备信息锁定失败".to_string())? = next_device;

    Ok(saved)
}

#[tauri::command]
fn list_devices(state: State<'_, AppState>) -> CommandResult<Vec<DeviceInfo>> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;

    store
        .devices()
        .map_err(|error| format!("读取设备列表失败: {error}"))
}

#[tauri::command]
async fn test_s3_connection(state: State<'_, AppState>) -> CommandResult<()> {
    let settings = {
        let store = state
            .store
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        store
            .get_settings()
            .map_err(|error| format!("读取设置失败: {error}"))?
            .s3_sync
    };

    s3_sync::test_connection(&settings)
        .await
        .map_err(|error| format!("S3 连接测试失败: {error}"))
}

#[tauri::command]
async fn sync_s3_now(state: State<'_, AppState>) -> CommandResult<S3SyncReport> {
    let now = now_ms();
    let cutoff = history_retention_cutoff(now);
    let (settings, device, local_samples) = {
        let store = state
            .store
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let settings = store
            .get_settings()
            .map_err(|error| format!("读取设置失败: {error}"))?
            .s3_sync;
        let device = state
            .device
            .lock()
            .map_err(|_| "设备信息锁定失败".to_string())?
            .clone();
        let local_samples = store
            .metric_history(&device.id, cutoff, now)
            .map_err(|error| format!("读取本机历史采样失败: {error}"))?;
        (settings, device, local_samples)
    };

    let (uploaded_days, downloaded) =
        s3_sync::upload_and_download(&settings, &device, &local_samples, now)
            .await
            .map_err(|error| format!("S3 同步失败: {error}"))?;

    let downloaded_devices = downloaded.len();
    let mut imported_samples = 0;
    {
        let store = state
            .store
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        for item in downloaded {
            store
                .upsert_device(&item.device, now)
                .map_err(|error| format!("保存远端设备失败: {error}"))?;
            imported_samples += store
                .import_metric_samples(&item.samples)
                .map_err(|error| format!("导入远端历史失败: {error}"))?;
        }
    }

    Ok(S3SyncReport {
        uploaded_days,
        downloaded_devices,
        imported_samples,
    })
}

#[tauri::command]
fn get_local_data_stats(state: State<'_, AppState>) -> CommandResult<LocalDataStats> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;

    store
        .local_data_stats()
        .map_err(|error| format!("读取本地数据统计失败: {error}"))
}

#[tauri::command]
fn clear_local_metric_samples(state: State<'_, AppState>) -> CommandResult<usize> {
    let store = state
        .store
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;

    store
        .clear_metric_samples()
        .map_err(|error| format!("清理本地历史数据失败: {error}"))
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "打开面板", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("System Stats Monitoring")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;

    Ok(())
}

fn app_database_path(app: &tauri::App) -> anyhow::Result<PathBuf> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join("system-stats-monitoring.sqlite3"))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = app_database_path(app)
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;
            let store = Store::new(&db_path)
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;
            store
                .init()
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;

            let settings = store
                .get_settings()
                .unwrap_or_else(|_| AppSettings::default());
            let device = DeviceInfo::current_with_machine_name(&settings.machine_name);
            store
                .upsert_device(&device, now_ms())
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;
            let collector = MetricCollector::new(device.id.clone());

            app.manage(AppState {
                store: Mutex::new(store),
                collector: Mutex::new(collector),
                device: Mutex::new(device),
            });

            setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_device_info,
            get_latest_metrics,
            save_metric_sample,
            get_metric_history,
            get_settings,
            update_settings,
            list_devices,
            test_s3_connection,
            sync_s3_now,
            get_local_data_stats,
            clear_local_metric_samples
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(test)]
mod tests {
    use crate::models::{MetricSample, SensorCategory, SensorReading, SensorUnit};
    use crate::s3_sync::{manifest_key, normalize_prefix, samples_key};
    use crate::settings::AppSettings;
    use crate::store::Store;

    fn sample(ts: i64, cpu_usage: Option<f64>) -> MetricSample {
        MetricSample {
            id: None,
            device_id: "test-device".to_string(),
            ts,
            cpu_usage,
            memory_used: Some(4_000),
            memory_total: Some(8_000),
            disk_used: Some(100_000),
            disk_total: Some(200_000),
            network_rx: Some(120.0),
            network_tx: Some(80.0),
            gpu_usage: Some(48.0),
            gpu_memory_total: Some(16_000),
            gpu_name: Some("Apple M4 Pro".to_string()),
            temperature_celsius: Some(63.5),
            power_watts: Some(18.2),
            sensor_readings: vec![
                SensorReading {
                    id: "cpu-core-1".to_string(),
                    label: "CPU performance core 1".to_string(),
                    category: SensorCategory::Temperature,
                    value: 63.5,
                    unit: SensorUnit::Celsius,
                },
                SensorReading {
                    id: "system-power".to_string(),
                    label: "System Total".to_string(),
                    category: SensorCategory::Power,
                    value: 18.2,
                    unit: SensorUnit::Watt,
                },
            ],
        }
    }

    #[test]
    fn validates_settings_ranges() {
        assert!(AppSettings {
            sample_interval_sec: 1,
            local_save_interval_sec: 5,
            machine_name: "Studio Mac".to_string(),
            language: "zh-CN".to_string(),
            metrics: Default::default(),
            s3_sync: Default::default(),
        }
        .validate()
        .is_ok());

        assert!(AppSettings {
            sample_interval_sec: 0,
            local_save_interval_sec: 5,
            machine_name: "Studio Mac".to_string(),
            language: "zh-CN".to_string(),
            metrics: Default::default(),
            s3_sync: Default::default(),
        }
        .validate()
        .is_err());

        assert!(AppSettings {
            sample_interval_sec: 10,
            local_save_interval_sec: 5,
            machine_name: "Studio Mac".to_string(),
            language: "zh-CN".to_string(),
            metrics: Default::default(),
            s3_sync: Default::default(),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn defaults_to_simplified_chinese_and_enabled_metrics() {
        let settings = AppSettings::default();

        assert_eq!(settings.language, "zh-CN");
        assert_eq!(settings.machine_name, "");
        assert!(settings.metrics.cpu);
        assert!(settings.metrics.memory);
        assert!(settings.metrics.disk);
        assert!(settings.metrics.network);
        assert!(settings.metrics.gpu);
        assert!(settings.metrics.temperature);
        assert!(settings.metrics.power);
        assert!(settings.metrics.battery);
        assert!(!settings.s3_sync.enabled);
        assert_eq!(settings.s3_sync.region, "us-east-1");
        assert_eq!(settings.s3_sync.prefix, "system-stats-monitoring");
        assert_eq!(settings.s3_sync.sync_interval_min, 10);
    }

    #[test]
    fn rejects_incomplete_enabled_s3_sync_settings() {
        let mut settings = AppSettings::default();
        settings.s3_sync.enabled = true;

        assert!(settings.validate().is_err());
    }

    #[test]
    fn builds_stable_s3_object_keys() {
        let prefix = normalize_prefix(" /team-a/system-monitor/ ").expect("normalize prefix");

        assert_eq!(prefix, "team-a/system-monitor");
        assert_eq!(
            manifest_key(&prefix, "studio-mac"),
            "team-a/system-monitor/devices/studio-mac/manifest.json"
        );
        assert_eq!(
            samples_key(&prefix, "studio-mac", "2026-07-06"),
            "team-a/system-monitor/devices/studio-mac/samples/2026-07-06.json"
        );
    }

    #[test]
    fn stores_and_queries_metric_samples_by_range() {
        let store = Store::in_memory().expect("create in-memory store");
        store.init().expect("initialize schema");

        store
            .save_metric_sample(&sample(1_000, Some(10.0)))
            .expect("save first sample");
        store
            .save_metric_sample(&sample(2_000, Some(20.0)))
            .expect("save second sample");

        let history = store
            .metric_history("test-device", 1_500, 2_500)
            .expect("query metric history");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].ts, 2_000);
        assert_eq!(history[0].cpu_usage, Some(20.0));
    }

    #[test]
    fn stores_nullable_disabled_metric_fields() {
        let store = Store::in_memory().expect("create in-memory store");
        store.init().expect("initialize schema");

        store
            .save_metric_sample(&sample(1_000, None))
            .expect("save sample with disabled cpu");

        let history = store
            .metric_history("test-device", 0, 2_000)
            .expect("query metric history");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].cpu_usage, None);
        assert_eq!(history[0].gpu_usage, Some(48.0));
        assert_eq!(history[0].gpu_memory_total, Some(16_000));
        assert_eq!(history[0].gpu_name.as_deref(), Some("Apple M4 Pro"));
        assert_eq!(history[0].temperature_celsius, Some(63.5));
        assert_eq!(history[0].power_watts, Some(18.2));
        assert_eq!(history[0].sensor_readings.len(), 2);
        assert_eq!(
            history[0].sensor_readings[0].label,
            "CPU performance core 1"
        );
    }

    #[test]
    fn prunes_old_metric_samples() {
        let store = Store::in_memory().expect("create in-memory store");
        store.init().expect("initialize schema");

        store
            .save_metric_sample(&sample(1_000, Some(10.0)))
            .expect("save old sample");
        store
            .save_metric_sample(&sample(2_000, Some(20.0)))
            .expect("save fresh sample");

        let deleted = store
            .prune_metric_samples_before(1_500)
            .expect("prune old samples");
        let history = store
            .metric_history("test-device", 0, 3_000)
            .expect("query remaining samples");

        assert_eq!(deleted, 1);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].ts, 2_000);
    }

    #[test]
    fn reports_and_clears_local_metric_data() {
        let store = Store::in_memory().expect("create in-memory store");
        store.init().expect("initialize schema");
        store
            .save_metric_sample(&sample(1_000, Some(10.0)))
            .expect("save sample");

        let before = store.local_data_stats().expect("read local data stats");
        assert_eq!(before.sample_count, 1);

        let deleted = store.clear_metric_samples().expect("clear metric samples");
        let after = store
            .local_data_stats()
            .expect("read local data stats again");

        assert_eq!(deleted, 1);
        assert_eq!(after.sample_count, 0);
    }

    #[test]
    fn keeps_thirty_days_for_month_history() {
        let now = 1_783_317_600_000;

        assert_eq!(
            super::history_retention_cutoff(now),
            now - 30 * 24 * 60 * 60 * 1000
        );
    }
}
