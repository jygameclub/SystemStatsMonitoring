pub mod metrics;
pub mod models;
pub mod settings;
pub mod store;

use std::path::PathBuf;
use std::sync::Mutex;

use metrics::MetricCollector;
use models::{DeviceInfo, HistoryQuery, MetricSample};
use settings::AppSettings;
use store::Store;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};

pub struct AppState {
    store: Mutex<Store>,
    collector: Mutex<MetricCollector>,
    device: DeviceInfo,
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
fn get_device_info(state: State<'_, AppState>) -> DeviceInfo {
    state.device.clone()
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

    let cutoff = sample.ts - 24 * 60 * 60 * 1000;
    store
        .prune_metric_samples_before(cutoff)
        .map_err(|error| format!("清理历史采样失败: {error}"))?;

    Ok(())
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
    let device_id = query.device_id.unwrap_or_else(|| state.device.id.clone());

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

    store
        .update_settings(&settings)
        .map_err(|error| format!("保存设置失败: {error}"))
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

            let device = DeviceInfo::current();
            let collector = MetricCollector::new(device.id.clone());

            app.manage(AppState {
                store: Mutex::new(store),
                collector: Mutex::new(collector),
                device,
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
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(test)]
mod tests {
    use crate::models::MetricSample;
    use crate::settings::AppSettings;
    use crate::store::Store;

    fn sample(ts: i64, cpu_usage: f64) -> MetricSample {
        MetricSample {
            id: None,
            device_id: "test-device".to_string(),
            ts,
            cpu_usage,
            memory_used: 4_000,
            memory_total: 8_000,
            disk_used: 100_000,
            disk_total: 200_000,
            network_rx: 120.0,
            network_tx: 80.0,
        }
    }

    #[test]
    fn validates_settings_ranges() {
        assert!(AppSettings {
            sample_interval_sec: 1,
            local_save_interval_sec: 5,
        }
        .validate()
        .is_ok());

        assert!(AppSettings {
            sample_interval_sec: 0,
            local_save_interval_sec: 5,
        }
        .validate()
        .is_err());

        assert!(AppSettings {
            sample_interval_sec: 10,
            local_save_interval_sec: 5,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn stores_and_queries_metric_samples_by_range() {
        let store = Store::in_memory().expect("create in-memory store");
        store.init().expect("initialize schema");

        store
            .save_metric_sample(&sample(1_000, 10.0))
            .expect("save first sample");
        store
            .save_metric_sample(&sample(2_000, 20.0))
            .expect("save second sample");

        let history = store
            .metric_history("test-device", 1_500, 2_500)
            .expect("query metric history");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].ts, 2_000);
        assert_eq!(history[0].cpu_usage, 20.0);
    }

    #[test]
    fn prunes_old_metric_samples() {
        let store = Store::in_memory().expect("create in-memory store");
        store.init().expect("initialize schema");

        store
            .save_metric_sample(&sample(1_000, 10.0))
            .expect("save old sample");
        store
            .save_metric_sample(&sample(2_000, 20.0))
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
}
