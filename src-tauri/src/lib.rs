mod database;
mod models;

use database::Database;
use models::{Record, RecordQuery};
use tauri::{Manager, State};

/// These commands are the only boundary used by the React UI.
/// The UI must never access SQLite or construct SQL directly.
#[tauri::command]
fn list_records(_query: RecordQuery, _database: State<'_, Database>) -> Result<Vec<Record>, String> {
    // TODO: parameterized search across title, content, tags and meeting metadata.
    Ok(Vec::new())
}

#[tauri::command]
fn get_record(_id: String, _database: State<'_, Database>) -> Result<Option<Record>, String> { Ok(None) }

#[tauri::command]
fn save_record(record: Record, _database: State<'_, Database>) -> Result<Record, String> {
    // TODO: INSERT ... ON CONFLICT(id) DO UPDATE, preserving created_at.
    Ok(record)
}

#[tauri::command]
fn delete_record(_id: String, _database: State<'_, Database>) -> Result<(), String> { Ok(()) }

#[tauri::command]
fn export_record_markdown(_id: String, _destination: String, _database: State<'_, Database>) -> Result<(), String> {
    // TODO: write only to a user-selected or configured export directory.
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
            app.manage(Database::open(data_dir).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_records, get_record, save_record, delete_record, export_record_markdown])
        .run(tauri::generate_context!())
        .expect("运行 ReportManager 时发生错误");
}
