use std::{fs, path::PathBuf, sync::Mutex};
use rusqlite::Connection;
pub struct Database(pub Mutex<Connection>);
impl Database {
    pub fn open(data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
        let connection = Connection::open(data_dir.join("report-manager.db")).map_err(|error| error.to_string())?;
        connection.execute_batch(include_str!("../migrations/0001_initial.sql")).map_err(|error| error.to_string())?;
        Ok(Self(Mutex::new(connection)))
    }
}
