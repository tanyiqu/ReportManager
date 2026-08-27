mod database;
mod models;

use database::Database;
use models::{AppPreferences, NavigationMenu, Record, RecordQuery};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window, WindowEvent,
};

const TRAY_SHOW_WINDOW_ID: &str = "show-main-window";
const TRAY_QUIT_ID: &str = "quit-application";
const WINDOW_CONFIG_FILE: &str = "config.json";
const LEGACY_WINDOW_SIZE_CONFIG_FILE: &str = "window-size.json";
const MIN_WINDOW_WIDTH: f64 = 980.0;
const MIN_WINDOW_HEIGHT: f64 = 650.0;
/// Labels for report-only windows contain the menu id, allowing the frontend to
/// render the same workspace without the main navigation sidebar.
const REPORT_WINDOW_LABEL_PREFIX: &str = "report-window-";

/// Runtime copy of the close preference, updated whenever settings are saved.
/// Keeping it in memory lets the window event apply changes without locking SQLite.
struct CloseBehavior(AtomicBool);

/// Startup-only window configuration kept independently from SQLite.
///
/// Size is logical so it remains appropriate after a display scale-factor
/// change. Position is physical because desktop coordinates are reported by
/// the operating system in physical pixels.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowConfig {
    width: f64,
    height: f64,
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
}

impl WindowConfig {
    fn is_valid(self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && self.width >= MIN_WINDOW_WIDTH
            && self.height >= MIN_WINDOW_HEIGHT
    }
}

/// Stores and migrates the main window configuration beside the portable
/// database. This is available before SQLite and before the frontend starts.
struct WindowConfigStore {
    path: PathBuf,
    legacy_path: PathBuf,
}

impl WindowConfigStore {
    fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(WINDOW_CONFIG_FILE),
            legacy_path: data_dir.join(LEGACY_WINDOW_SIZE_CONFIG_FILE),
        }
    }

    fn read(&self) -> Option<WindowConfig> {
        let raw = fs::read_to_string(&self.path).ok()?;
        let config = serde_json::from_str::<WindowConfig>(&raw).ok()?;
        config.is_valid().then_some(config)
    }

    /// Moves the former size-only file to the unified configuration on first
    /// launch after upgrade. An invalid legacy file is deliberately ignored.
    fn migrate_legacy(&self) -> Option<WindowConfig> {
        let raw = fs::read_to_string(&self.legacy_path).ok()?;
        let config = serde_json::from_str::<WindowConfig>(&raw).ok()?;
        if !config.is_valid() {
            return None;
        }
        if self.save(config) {
            if let Err(error) = fs::remove_file(&self.legacy_path) {
                eprintln!("删除旧窗口配置失败: {error}");
            }
        }
        Some(config)
    }

    fn save(&self, config: WindowConfig) -> bool {
        if !config.is_valid() {
            return false;
        }
        let content = match serde_json::to_vec_pretty(&config) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("序列化窗口配置失败: {error}");
                return false;
            }
        };
        if let Err(error) = fs::write(&self.path, content) {
            eprintln!("保存窗口配置失败: {error}");
            return false;
        }
        true
    }

    fn save_config(&self, logical_size: LogicalSize<f64>, position: Option<PhysicalPosition<i32>>) {
        let _ = self.save(WindowConfig {
            width: logical_size.width,
            height: logical_size.height,
            x: position.map(|value| value.x),
            y: position.map(|value| value.y),
        });
    }

    fn save_current_webview_window(&self, window: &WebviewWindow) {
        if window.is_maximized().unwrap_or(false) {
            return;
        }
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let Ok(size) = window.inner_size() else {
            return;
        };
        let logical_size = size.to_logical::<f64>(scale_factor);
        let position = window.outer_position().ok();
        self.save_config(logical_size, position);
    }

    fn save_current_window(&self, window: &Window) {
        if window.is_maximized().unwrap_or(false) {
            return;
        }
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let Ok(size) = window.inner_size() else {
            return;
        };
        self.save_config(
            size.to_logical::<f64>(scale_factor),
            window.outer_position().ok(),
        );
    }

    fn restore(&self, window: &WebviewWindow) {
        let config = self.read().or_else(|| self.migrate_legacy());
        let config = config.unwrap_or(WindowConfig {
            width: MIN_WINDOW_WIDTH.max(1180.0),
            height: MIN_WINDOW_HEIGHT.max(760.0),
            x: None,
            y: None,
        });

        let _ = window.set_size(LogicalSize::new(config.width, config.height));
        let position = match (config.x, config.y) {
            (Some(x), Some(y)) if self.position_is_visible(window, x, y) => {
                PhysicalPosition::new(x, y)
            }
            _ => self.center_on_primary_monitor(window),
        };
        let _ = window.set_position(position);

        // First launch and legacy migrations both immediately create the new
        // unified file. Persist the actual centered position for next startup.
        self.save_current_webview_window(window);
    }

    fn position_is_visible(&self, window: &WebviewWindow, x: i32, y: i32) -> bool {
        window.available_monitors().map_or(false, |monitors| {
            monitors.iter().any(|monitor| {
                let origin = monitor.position();
                let size = monitor.size();
                x >= origin.x
                    && x < origin.x.saturating_add(size.width as i32)
                    && y >= origin.y
                    && y < origin.y.saturating_add(size.height as i32)
            })
        })
    }

    fn center_on_primary_monitor(&self, window: &WebviewWindow) -> PhysicalPosition<i32> {
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let size = window
            .inner_size()
            .unwrap_or_else(|_| LogicalSize::new(1180.0, 760.0).to_physical(scale_factor));
        let monitor = window.primary_monitor().ok().flatten();
        let Some(monitor) = monitor else {
            return PhysicalPosition::new(0, 0);
        };
        let origin = monitor.position();
        let monitor_size = monitor.size();
        PhysicalPosition::new(
            origin.x + (monitor_size.width.saturating_sub(size.width) / 2) as i32,
            origin.y + (monitor_size.height.saturating_sub(size.height) / 2) as i32,
        )
    }
}

/// Restores the existing window instead of creating another application instance.
fn reveal_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Saves the current main-window geometry before an explicit application exit.
fn save_main_window_config(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        app.state::<WindowConfigStore>()
            .save_current_webview_window(&window);
    }
}

/// Creates the persistent tray controls used while the main window is hidden.
fn create_system_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_window =
        MenuItem::with_id(app, TRAY_SHOW_WINDOW_ID, "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_QUIT_ID, "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_window, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("应用图标".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TRAY_SHOW_WINDOW_ID => reveal_main_window(app),
            TRAY_QUIT_ID => {
                save_main_window_config(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => reveal_main_window(tray.app_handle()),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Returns the portable data directory used by the application.
///
/// Release builds keep all mutable data beside the executable so the complete
/// application folder can be copied to another computer. During development,
/// keep the same data directory beside the Rust project instead of `target`,
/// because `clean.bat` deliberately removes that build directory.
fn portable_data_dir() -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
    }

    #[cfg(not(debug_assertions))]
    {
        let executable = env::current_exe().map_err(|error| error.to_string())?;
        let executable_dir = executable
            .parent()
            .ok_or_else(|| "无法确定程序所在文件夹".to_string())?;
        Ok(executable_dir.join("data"))
    }
}

/// Shows the invoking window only after the React application has rendered.
/// Both the main window and report-only windows start hidden so users never see
/// WebView's initial blank document.
#[tauri::command]
fn show_main_window(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

/// Opens one reusable report-only window for a navigation menu. Its geometry is
/// intentionally not stored: only the main window controls portable config.
#[tauri::command]
async fn open_report_window(
    menu_id: String,
    source_window: WebviewWindow,
    app: AppHandle,
) -> Result<(), String> {
    let label = format!("{REPORT_WINDOW_LABEL_PREFIX}{menu_id}");
    let scale_factor = source_window
        .scale_factor()
        .map_err(|error| error.to_string())?;
    let source_size = source_window
        .inner_size()
        .map_err(|error| error.to_string())?
        .to_logical::<f64>(scale_factor);
    let source_is_maximized = source_window.is_maximized().unwrap_or(false);

    if let Some(report_window) = app.get_webview_window(&label) {
        report_window
            .set_size(source_size)
            .map_err(|error| error.to_string())?;
        if source_is_maximized {
            report_window
                .maximize()
                .map_err(|error| error.to_string())?;
        } else {
            report_window
                .unmaximize()
                .map_err(|error| error.to_string())?;
        }
        report_window.show().map_err(|error| error.to_string())?;
        report_window
            .unminimize()
            .map_err(|error| error.to_string())?;
        return report_window.set_focus().map_err(|error| error.to_string());
    }

    // This command is async so WebView2 construction does not block Tauri's
    // window event loop. The new window reveals itself after React is ready.
    let report_window =
        WebviewWindowBuilder::new(&app, label, WebviewUrl::App("index.html".into()))
            .title("ReportManager - 独立报告窗口")
            .inner_size(source_size.width, source_size.height)
            .min_inner_size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
            .visible(false)
            .build()
            .map_err(|error| error.to_string())?;
    if source_is_maximized {
        report_window
            .maximize()
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_app_preferences(database: State<'_, Database>) -> Result<AppPreferences, String> {
    database.preferences()
}

#[tauri::command]
fn save_app_preferences(
    preferences: AppPreferences,
    database: State<'_, Database>,
    close_behavior: State<'_, CloseBehavior>,
) -> Result<AppPreferences, String> {
    let saved = database.save_preferences(preferences)?;
    close_behavior
        .0
        .store(saved.minimize_to_tray, Ordering::Relaxed);
    Ok(saved)
}

#[tauri::command]
fn create_navigation_menu(
    menu: NavigationMenu,
    database: State<'_, Database>,
) -> Result<AppPreferences, String> {
    database.create_menu(menu)
}

#[tauri::command]
fn delete_navigation_menu(
    id: String,
    database: State<'_, Database>,
) -> Result<AppPreferences, String> {
    database.delete_menu(id)
}

/// These commands are the only boundary used by the React UI.
/// The UI must never access SQLite or construct SQL directly.
#[tauri::command]
fn list_records(query: RecordQuery, database: State<'_, Database>) -> Result<Vec<Record>, String> {
    database.list_records(query)
}

#[tauri::command]
fn get_record(id: String, database: State<'_, Database>) -> Result<Option<Record>, String> {
    database.get_record(&id)
}

#[tauri::command]
fn save_record(record: Record, database: State<'_, Database>) -> Result<Record, String> {
    database.save_record(record)
}

#[tauri::command]
fn delete_record(id: String, database: State<'_, Database>) -> Result<(), String> {
    database.delete_record(&id)
}

#[tauri::command]
fn export_record_markdown(
    _id: String,
    _destination: String,
    _database: State<'_, Database>,
) -> Result<(), String> {
    // TODO: write only to a user-selected or configured export directory.
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = portable_data_dir().map_err(std::io::Error::other)?;
            let database = Database::open(data_dir.clone()).map_err(std::io::Error::other)?;
            let minimize_to_tray = database
                .preferences()
                .map_err(std::io::Error::other)?
                .minimize_to_tray;
            app.manage(database);
            app.manage(CloseBehavior(AtomicBool::new(minimize_to_tray)));
            app.manage(WindowConfigStore::new(&data_dir));
            if let Some(window) = app.get_webview_window("main") {
                app.state::<WindowConfigStore>().restore(&window);
            }
            create_system_tray(app.handle())?;
            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| {
                reveal_main_window(app);
            },
        ))
        .on_window_event(|window, event| {
            match event {
                WindowEvent::Resized(_) if window.label() == "main" => {
                    // Do not replace the user's normal size with the monitor's
                    // maximized dimensions. The last restored size remains.
                    if window.is_maximized().unwrap_or(false) {
                        return;
                    }
                    window
                        .app_handle()
                        .state::<WindowConfigStore>()
                        .save_current_window(window);
                }
                WindowEvent::CloseRequested { api, .. } => {
                    // Report-only windows are ephemeral. Closing them must not
                    // hide the main window, exit the application, or overwrite
                    // the main window's saved geometry.
                    if window.label() != "main" {
                        return;
                    }
                    // Persist the last visible desktop position before either
                    // hiding to the tray or exiting the application.
                    window
                        .app_handle()
                        .state::<WindowConfigStore>()
                        .save_current_window(window);
                    let minimize_to_tray = window
                        .app_handle()
                        .state::<CloseBehavior>()
                        .0
                        .load(Ordering::Relaxed);
                    if minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        // Closing is an explicit exit when the tray preference is disabled.
                        window.app_handle().exit(0);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            open_report_window,
            get_app_preferences,
            save_app_preferences,
            create_navigation_menu,
            delete_navigation_menu,
            list_records,
            get_record,
            save_record,
            delete_record,
            export_record_markdown
        ])
        .run(tauri::generate_context!())
        .expect("运行 ReportManager 时发生错误");
}
