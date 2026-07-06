use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorCategory {
    Temperature,
    Voltage,
    Current,
    Power,
    Energy,
    Fan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorUnit {
    Celsius,
    Volt,
    Ampere,
    Watt,
    WattHour,
    Percent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SensorReading {
    pub id: String,
    pub label: String,
    pub category: SensorCategory,
    pub value: f64,
    pub unit: SensorUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricSample {
    pub id: Option<i64>,
    pub device_id: String,
    pub ts: i64,
    pub cpu_usage: Option<f64>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub disk_used: Option<u64>,
    pub disk_total: Option<u64>,
    pub network_rx: Option<f64>,
    pub network_tx: Option<f64>,
    pub gpu_usage: Option<f64>,
    pub gpu_memory_total: Option<u64>,
    pub gpu_name: Option<String>,
    pub temperature_celsius: Option<f64>,
    pub power_watts: Option<f64>,
    pub sensor_readings: Vec<SensorReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalDataStats {
    pub database_path: String,
    pub database_size_bytes: u64,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryQuery {
    pub device_id: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub os: String,
    pub arch: String,
    pub agent_version: String,
}

impl DeviceInfo {
    pub fn current() -> Self {
        Self::current_with_machine_name("")
    }

    pub fn current_with_machine_name(machine_name: &str) -> Self {
        let fallback_name = System::host_name().unwrap_or_else(|| "Local Machine".to_string());
        let name = if machine_name.trim().is_empty() {
            fallback_name
        } else {
            machine_name.trim().to_string()
        };
        let os = System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
        let arch = std::env::consts::ARCH.to_string();
        let id = format!(
            "{}-{}-{}",
            sanitize_device_id(&name),
            sanitize_device_id(&os),
            sanitize_device_id(&arch)
        );

        Self {
            id,
            name,
            os,
            arch,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct S3SyncReport {
    pub uploaded_days: usize,
    pub downloaded_devices: usize,
    pub imported_samples: usize,
}

fn sanitize_device_id(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    sanitized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
