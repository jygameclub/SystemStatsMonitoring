use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde_json::Value;
use sysinfo::{Components, Disks, Networks, System};

use crate::models::{MetricSample, SensorCategory, SensorReading, SensorUnit};

#[derive(Debug, Clone, Default, PartialEq)]
struct GpuInfo {
    name: Option<String>,
    memory_total: Option<u64>,
}

pub struct MetricCollector {
    system: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    device_id: String,
    gpu: GpuInfo,
    last_disk_io_ts: Option<i64>,
}

impl MetricCollector {
    pub fn new(device_id: String) -> Self {
        Self {
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            device_id,
            gpu: detect_gpu_info(),
            last_disk_io_ts: None,
        }
    }

    pub fn set_device_id(&mut self, device_id: String) {
        self.device_id = device_id;
    }

    pub fn sample(&mut self) -> Result<MetricSample> {
        let sample_ts = now_ms();
        let disk_elapsed_secs = self
            .last_disk_io_ts
            .replace(sample_ts)
            .map(|previous| (sample_ts.saturating_sub(previous) as f64) / 1000.0);

        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh(true);
        self.networks.refresh(true);
        self.components.refresh(true);

        let memory_used = self.system.used_memory();
        let memory_total = self.system.total_memory();
        let (disk_used, disk_total) = disk_usage(&self.disks);
        let (disk_read_bytes, disk_write_bytes) = disk_io_rates(&self.disks, disk_elapsed_secs);
        let (network_rx, network_tx) = network_rates(&self.networks);
        let temperature_readings = component_temperature_readings(&self.components);
        let power_readings = detect_power_sensor_readings();
        let temperature_celsius =
            max_valid_temperature(temperature_readings.iter().map(|reading| {
                if matches!(reading.unit, SensorUnit::Celsius) {
                    Some(reading.value as f32)
                } else {
                    None
                }
            }));
        let power_watts = power_readings
            .iter()
            .find(|reading| reading.id == "system-power-in")
            .or_else(|| {
                power_readings
                    .iter()
                    .find(|reading| reading.id == "battery-power")
            })
            .map(|reading| reading.value);
        let mut sensor_readings =
            Vec::with_capacity(temperature_readings.len() + power_readings.len());
        sensor_readings.extend(temperature_readings);
        sensor_readings.extend(power_readings);

        Ok(MetricSample {
            id: None,
            device_id: self.device_id.clone(),
            ts: sample_ts,
            cpu_usage: Some(self.system.global_cpu_usage() as f64),
            memory_used: Some(memory_used),
            memory_total: Some(memory_total),
            disk_used: Some(disk_used),
            disk_total: Some(disk_total),
            disk_read_bytes: Some(disk_read_bytes),
            disk_write_bytes: Some(disk_write_bytes),
            network_rx: Some(network_rx),
            network_tx: Some(network_tx),
            gpu_usage: None,
            gpu_memory_total: self.gpu.memory_total,
            gpu_name: self.gpu.name.clone(),
            temperature_celsius,
            power_watts,
            sensor_readings,
        })
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn disk_usage(disks: &Disks) -> (u64, u64) {
    disks.iter().fold((0, 0), |(used_acc, total_acc), disk| {
        let total = disk.total_space();
        let available = disk.available_space();
        (
            used_acc + total.saturating_sub(available),
            total_acc + total,
        )
    })
}

fn disk_io_rates(disks: &Disks, elapsed_secs: Option<f64>) -> (f64, f64) {
    let elapsed_secs = match elapsed_secs {
        Some(value) if value.is_finite() && value > 0.0 => value,
        _ => return (0.0, 0.0),
    };
    let (read_bytes, written_bytes) = disks.iter().fold((0_u64, 0_u64), |acc, disk| {
        let usage = disk.usage();
        (
            acc.0.saturating_add(usage.read_bytes),
            acc.1.saturating_add(usage.written_bytes),
        )
    });

    (
        read_bytes as f64 / elapsed_secs,
        written_bytes as f64 / elapsed_secs,
    )
}

fn network_rates(networks: &Networks) -> (f64, f64) {
    networks
        .iter()
        .fold((0.0, 0.0), |(rx_acc, tx_acc), (_, data)| {
            (
                rx_acc + data.received() as f64,
                tx_acc + data.transmitted() as f64,
            )
        })
}

fn max_valid_temperature<I>(temperatures: I) -> Option<f64>
where
    I: IntoIterator<Item = Option<f32>>,
{
    temperatures
        .into_iter()
        .flatten()
        .filter(|value| value.is_finite())
        .map(f64::from)
        .max_by(|left, right| left.total_cmp(right))
}

fn component_temperature_readings(components: &Components) -> Vec<SensorReading> {
    components
        .iter()
        .filter_map(|component| {
            let value = component.temperature()?;
            if !value.is_finite() {
                return None;
            }

            let label = component.label().trim();
            let label = if label.is_empty() {
                "Temperature"
            } else {
                label
            };

            Some(SensorReading {
                id: sensor_id("temperature", label),
                label: label.to_string(),
                category: SensorCategory::Temperature,
                value: f64::from(value),
                unit: SensorUnit::Celsius,
            })
        })
        .collect()
}

fn sensor_id(prefix: &str, label: &str) -> String {
    let normalized = label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if normalized.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}-{normalized}")
    }
}

fn detect_power_sensor_readings() -> Vec<SensorReading> {
    detect_platform_power_sensor_readings()
}

#[cfg(target_os = "macos")]
fn detect_platform_power_sensor_readings() -> Vec<SensorReading> {
    let output = std::process::Command::new("ioreg")
        .args(["-r", "-n", "AppleSmartBattery", "-d", "1"])
        .output()
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|raw| parse_macos_power_sensor_readings(&raw))
        .unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
fn detect_platform_power_sensor_readings() -> Vec<SensorReading> {
    Vec::new()
}

fn parse_macos_power_sensor_readings(raw: &str) -> Vec<SensorReading> {
    let mut readings = Vec::new();

    if let Some(power_mw) = parse_ioreg_embedded_number(raw, "SystemPowerIn")
        .or_else(|| parse_ioreg_embedded_number(raw, "BatteryPower"))
        .filter(|value| *value > 0)
    {
        readings.push(SensorReading {
            id: "system-power-in".to_string(),
            label: "System Total".to_string(),
            category: SensorCategory::Power,
            value: power_mw as f64 / 1000.0,
            unit: SensorUnit::Watt,
        });
    }

    if let Some(voltage_mv) = parse_ioreg_embedded_number(raw, "SystemVoltageIn")
        .filter(|value| *value > 0)
        .or_else(|| parse_ioreg_number(raw, "Voltage").filter(|value| *value > 0))
    {
        readings.push(SensorReading {
            id: "system-voltage-in".to_string(),
            label: "DC In".to_string(),
            category: SensorCategory::Voltage,
            value: voltage_mv as f64 / 1000.0,
            unit: SensorUnit::Volt,
        });
    }

    if let Some(current_ma) = parse_ioreg_embedded_number(raw, "SystemCurrentIn")
        .filter(|value| *value > 0)
        .or_else(|| {
            parse_ioreg_number(raw, "InstantAmperage")
                .or_else(|| parse_ioreg_number(raw, "Amperage"))
                .filter(|value| *value != 0)
                .map(i64::abs)
        })
    {
        readings.push(SensorReading {
            id: "system-current-in".to_string(),
            label: "DC In".to_string(),
            category: SensorCategory::Current,
            value: current_ma as f64 / 1000.0,
            unit: SensorUnit::Ampere,
        });
    }

    if !readings
        .iter()
        .any(|reading| reading.id == "system-power-in")
    {
        if let (Some(voltage_mv), Some(amperage_ma)) = (
            parse_ioreg_number(raw, "Voltage").filter(|value| *value > 0),
            parse_ioreg_number(raw, "InstantAmperage")
                .or_else(|| parse_ioreg_number(raw, "Amperage"))
                .filter(|value| *value != 0)
                .map(i64::abs),
        ) {
            let watts = (voltage_mv as f64 * amperage_ma as f64) / 1_000_000.0;
            if watts.is_finite() && watts > 0.0 {
                readings.push(SensorReading {
                    id: "battery-power".to_string(),
                    label: "Battery Power".to_string(),
                    category: SensorCategory::Power,
                    value: watts,
                    unit: SensorUnit::Watt,
                });
            }
        }
    }

    for (key, id, label) in [
        ("SystemLoad", "system-load", "System Load"),
        (
            "AdapterEfficiencyLoss",
            "adapter-efficiency-loss",
            "Adapter Efficiency Loss",
        ),
        (
            "WallEnergyEstimate",
            "wall-energy-estimate",
            "Wall Energy Estimate",
        ),
    ] {
        if let Some(value_milli) = parse_ioreg_embedded_number(raw, key).filter(|value| *value > 0)
        {
            readings.push(SensorReading {
                id: id.to_string(),
                label: label.to_string(),
                category: if key == "WallEnergyEstimate" {
                    SensorCategory::Energy
                } else {
                    SensorCategory::Power
                },
                value: value_milli as f64 / 1000.0,
                unit: if key == "WallEnergyEstimate" {
                    SensorUnit::WattHour
                } else {
                    SensorUnit::Watt
                },
            });
        }
    }

    readings
}

fn parse_ioreg_number(raw: &str, key: &str) -> Option<i64> {
    raw.lines().find_map(|line| {
        let key_marker = format!("\"{key}\"");
        if !line.contains(&key_marker) {
            return None;
        }

        line.split('=')
            .nth(1)
            .and_then(|value| value.trim().parse::<i64>().ok())
    })
}

fn parse_ioreg_embedded_number(raw: &str, key: &str) -> Option<i64> {
    let key_marker = format!("\"{key}\"=");
    let start = raw.find(&key_marker)? + key_marker.len();
    let value_text: String = raw[start..]
        .chars()
        .skip_while(|ch| ch.is_whitespace())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect();

    value_text.parse::<i64>().ok()
}

fn detect_gpu_info() -> GpuInfo {
    detect_platform_gpu_info().unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn detect_platform_gpu_info() -> Option<GpuInfo> {
    let output = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json", "-detailLevel", "mini"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    parse_macos_system_profiler_gpu(&raw)
}

#[cfg(target_os = "windows")]
fn detect_platform_gpu_info() -> Option<GpuInfo> {
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM | ConvertTo-Json -Compress",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    parse_windows_video_controller_gpu(&raw)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn detect_platform_gpu_info() -> Option<GpuInfo> {
    None
}

fn parse_macos_system_profiler_gpu(raw: &str) -> Option<GpuInfo> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let displays = value.get("SPDisplaysDataType")?.as_array()?;

    displays.iter().find_map(|display| {
        let name = first_nonempty_string(
            display,
            &["sppci_model", "spdisplays_chipset_model", "_name"],
        );
        let memory_total = first_memory_value(
            display,
            &[
                "spdisplays_vram",
                "_spdisplays_vram",
                "spdisplays_vram_shared",
                "_spdisplays_vram_shared",
            ],
        );

        if name.is_some() || memory_total.is_some() {
            Some(GpuInfo { name, memory_total })
        } else {
            None
        }
    })
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_video_controller_gpu(raw: &str) -> Option<GpuInfo> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let controllers: Vec<&Value> = match &value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![&value],
        _ => Vec::new(),
    };

    controllers.into_iter().find_map(|controller| {
        let name = first_nonempty_string(controller, &["Name"]);
        let memory_total = controller.get("AdapterRAM").and_then(value_to_u64);

        if name.is_some() || memory_total.is_some() {
            Some(GpuInfo { name, memory_total })
        } else {
            None
        }
    })
}

fn first_nonempty_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn first_memory_value(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|item| {
            item.as_str()
                .and_then(parse_gpu_memory_bytes)
                .or_else(|| value_to_u64(item))
        })
    })
}

fn value_to_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        .or_else(|| {
            value
                .as_str()
                .and_then(|text| text.trim().parse::<u64>().ok())
        })
}

fn parse_gpu_memory_bytes(raw: &str) -> Option<u64> {
    let normalized = raw.trim().to_ascii_uppercase();
    let mut number = String::new();
    let mut started = false;

    for ch in normalized.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            started = true;
            number.push(ch);
        } else if started {
            break;
        }
    }

    let value = number.parse::<f64>().ok()?;
    let multiplier = if normalized.contains("TB") {
        1024_f64.powi(4)
    } else if normalized.contains("GB") {
        1024_f64.powi(3)
    } else if normalized.contains("MB") {
        1024_f64.powi(2)
    } else if normalized.contains("KB") {
        1024_f64
    } else {
        return None;
    };

    Some((value * multiplier).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::{
        max_valid_temperature, parse_gpu_memory_bytes, parse_macos_power_sensor_readings,
        parse_macos_system_profiler_gpu, parse_windows_video_controller_gpu,
    };

    #[test]
    fn disk_usage_saturates_when_available_exceeds_total() {
        let total = 100_u64;
        let available = 120_u64;

        assert_eq!(total.saturating_sub(available), 0);
    }

    #[test]
    fn parses_macos_gpu_name_and_memory() {
        let raw = r#"
        {
          "SPDisplaysDataType": [
            {
              "sppci_model": "Apple M4 Pro",
              "spdisplays_vram": "24 GB"
            }
          ]
        }
        "#;

        let gpu = parse_macos_system_profiler_gpu(raw).expect("parse macOS GPU");

        assert_eq!(gpu.name.as_deref(), Some("Apple M4 Pro"));
        assert_eq!(gpu.memory_total, Some(24 * 1024 * 1024 * 1024));
    }

    #[test]
    fn parses_windows_gpu_name_and_memory() {
        let raw = r#"
        [
          {
            "Name": "NVIDIA GeForce RTX 4070",
            "AdapterRAM": 8589934592
          }
        ]
        "#;

        let gpu = parse_windows_video_controller_gpu(raw).expect("parse Windows GPU");

        assert_eq!(gpu.name.as_deref(), Some("NVIDIA GeForce RTX 4070"));
        assert_eq!(gpu.memory_total, Some(8 * 1024 * 1024 * 1024));
    }

    #[test]
    fn parses_gpu_memory_units() {
        assert_eq!(parse_gpu_memory_bytes("1536 MB"), Some(1536 * 1024 * 1024));
        assert_eq!(parse_gpu_memory_bytes("8 GB"), Some(8 * 1024 * 1024 * 1024));
    }

    #[test]
    fn chooses_highest_valid_temperature() {
        assert_eq!(
            max_valid_temperature([Some(49.5), Some(f32::NAN), Some(63.25), None]),
            Some(63.25)
        );
    }

    #[test]
    fn parses_macos_battery_power_watts() {
        let raw = r#"
        | |           "Voltage" = 11812
        | |           "InstantAmperage" = -1460
        "#;

        let readings = parse_macos_power_sensor_readings(raw);

        assert_eq!(
            readings
                .iter()
                .find(|reading| reading.id == "battery-power")
                .map(|reading| reading.value),
            Some(17.24552)
        );
    }

    #[test]
    fn parses_macos_power_telemetry_watts() {
        let raw = r#"
        | |           "PowerTelemetryData" = {"SystemPowerIn"=21349,"SystemVoltageIn"=12065,"SystemCurrentIn"=1769,"SystemLoad"=21349,"AdapterEfficiencyLoss"=611,"WallEnergyEstimate"=6541}
        "#;

        let readings = parse_macos_power_sensor_readings(raw);

        assert_eq!(
            readings
                .iter()
                .find(|reading| reading.id == "system-power-in")
                .map(|reading| reading.value),
            Some(21.349)
        );
        assert_eq!(
            readings
                .iter()
                .find(|reading| reading.id == "system-voltage-in")
                .map(|reading| reading.value),
            Some(12.065)
        );
        assert_eq!(
            readings
                .iter()
                .find(|reading| reading.id == "system-current-in")
                .map(|reading| reading.value),
            Some(1.769)
        );
    }
}
