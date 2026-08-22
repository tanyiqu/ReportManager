mod database;
mod models;

use database::Database;
use models::{AppPreferences, NavigationMenu, Record, RecordQuery};
use std::{
    env,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WebviewWindow, WindowEvent,
};

const TRAY_SHOW_WINDOW_ID: &str = "show-main-window";
const TRAY_QUIT_ID: &str = "quit-application";

/// Runtime copy of the close preference, updated whenever settings are saved.
/// Keeping it in memory lets the window event apply changes without locking SQLite.
struct CloseBehavior(AtomicBool);

/// Restores the existing window instead of creating another application instance.
fn reveal_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
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
            TRAY_QUIT_ID => app.exit(0),
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

/// Shows the main window only after the React application has rendered.
/// This prevents the system from displaying the WebView's initial blank page.
#[tauri::command]
fn show_main_window(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
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
            let database = Database::open(data_dir).map_err(std::io::Error::other)?;
            let minimize_to_tray = database
                .preferences()
                .map_err(std::io::Error::other)?
                .minimize_to_tray;
            app.manage(database);
            app.manage(CloseBehavior(AtomicBool::new(minimize_to_tray)));
            create_system_tray(app.handle())?;
            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| {
                reveal_main_window(app);
            },
        ))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
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
        })
        .invoke_handler(tauri::generate_handler![
            show_main_window,
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
