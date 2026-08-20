use rusqlite::Connection;
use std::{fs, path::PathBuf, sync::Mutex};

/// Owns the application's single SQLite connection.
///
/// Access stays inside the Rust command layer so the frontend never interacts
/// with SQLite directly.
pub struct Database(Mutex<Connection>);

impl Database {
    pub fn open(data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
        let connection = Connection::open(data_dir.join("report-manager.db"))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .map_err(|error| error.to_string())?;
        Ok(Self(Mutex::new(connection)))
    }

    /// Confirms that the managed connection can be acquired and queried.
    pub fn ensure_available(&self) -> Result<(), String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        connection
            .execute_batch("SELECT 1;")
            .map_err(|error| error.to_string())
    }
}
