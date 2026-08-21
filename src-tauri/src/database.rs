use crate::models::{AppPreferences, NavigationMenu};
use rusqlite::{params, Connection, OptionalExtension};
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
        // `ALTER TABLE ... ADD COLUMN` is the only non-idempotent statement in
        // 0002. Ignore its expected duplicate-column error on later launches,
        // but keep reporting every other migration failure.
        connection
            .execute_batch(include_str!(
                "../migrations/0002_navigation_preferences.sql"
            ))
            .or_else(|error| {
                if error.to_string().contains("duplicate column name") {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|error| error.to_string())?;
        Self::seed_navigation(&connection)?;
        Ok(Self(Mutex::new(connection)))
    }

    fn seed_navigation(connection: &Connection) -> Result<(), String> {
        let defaults = [
            ("home", "首页", "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><path d=\"m3 10 9-7 9 7v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1Z\"/><path d=\"M9 21v-6h6v6\"/></svg>", 0_i64, 1_i64),
            ("daily", "日报", "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><path d=\"M6 3h9l3 3v15H6z\"/><path d=\"M9 12h6M9 16h6M9 8h3\"/></svg>", 1, 0),
            ("weekly", "周报", "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"17\" rx=\"2\"/><path d=\"M8 2v4M16 2v4M3 10h18M8 14h3M8 17h7\"/></svg>", 2, 0),
            ("meeting", "例会记录", "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><path d=\"M4 5h16v11H8l-4 4Z\"/><path d=\"M8 9h8M8 12h5\"/></svg>", 3, 0),
            ("settings", "设置", "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.4 2.4-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.03 1.56v.1h-3.4v-.1A1.7 1.7 0 0 0 10 18.94a1.7 1.7 0 0 0-1.88.34l-.06.06-2.4-2.4.06-.06A1.7 1.7 0 0 0 6.06 15 1.7 1.7 0 0 0 4.5 13.97h-.1v-3.4h.1A1.7 1.7 0 0 0 6.06 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.4-2.4.06.06A1.7 1.7 0 0 0 10 5.06a1.7 1.7 0 0 0 1.03-1.56v-.1h3.4v.1A1.7 1.7 0 0 0 15.46 5a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.4 2.4-.06.06A1.7 1.7 0 0 0 19.4 9a1.7 1.7 0 0 0 1.56 1.03h.1v3.4h-.1A1.7 1.7 0 0 0 19.4 15Z\"/></svg>", 4, 1),
        ];
        for (id, label, icon_svg, order, is_system) in defaults {
            connection.execute(
                "INSERT OR IGNORE INTO navigation_menus (id, label, icon_svg, sort_order, is_system) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, label, icon_svg, order, is_system],
            ).map_err(|error| error.to_string())?;
        }
        for (key, value) in [
            ("sidebar_collapsed", "false"),
            ("default_page_id", "home"),
            ("week_start", "monday"),
            ("export_directory", ""),
        ] {
            connection
                .execute(
                    "INSERT OR IGNORE INTO app_settings (key, value) VALUES (?1, ?2)",
                    params![key, value],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Confirms that the managed connection can be acquired and queried.
    pub fn ensure_available(&self) -> Result<(), String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        connection
            .execute_batch("SELECT 1;")
            .map_err(|error| error.to_string())
    }

    pub fn preferences(&self) -> Result<AppPreferences, String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        let setting = |key: &str, fallback: &str| -> Result<String, String> {
            connection
                .query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    [key],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())
                .map(|value| value.unwrap_or_else(|| fallback.to_string()))
        };
        let mut statement = connection.prepare("SELECT id, label, icon_svg, sort_order, is_system FROM navigation_menus ORDER BY sort_order")
            .map_err(|error| error.to_string())?;
        let menus = statement
            .query_map([], |row| {
                Ok(NavigationMenu {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    icon_svg: row.get(2)?,
                    sort_order: row.get(3)?,
                    is_system: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(AppPreferences {
            sidebar_collapsed: setting("sidebar_collapsed", "false")? == "true",
            default_page_id: setting("default_page_id", "home")?,
            week_start: setting("week_start", "monday")?,
            export_directory: setting("export_directory", "")?,
            menus,
        })
    }

    pub fn save_preferences(&self, preferences: AppPreferences) -> Result<AppPreferences, String> {
        let mut connection = self.0.lock().map_err(|error| error.to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        for (key, value) in [
            (
                "sidebar_collapsed",
                preferences.sidebar_collapsed.to_string(),
            ),
            ("default_page_id", preferences.default_page_id.clone()),
            ("week_start", preferences.week_start.clone()),
            ("export_directory", preferences.export_directory.clone()),
        ] {
            transaction.execute("INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value]).map_err(|error| error.to_string())?;
        }
        // First move mutable entries out of the unique sort-order range, then
        // write their new order. This makes swaps (for example 日报 and 周报)
        // safe under the unique index.
        transaction
            .execute(
                "UPDATE navigation_menus SET sort_order = -sort_order - 1 WHERE is_system = 0",
                [],
            )
            .map_err(|error| error.to_string())?;
        let mut order = 1_i64;
        for menu in &preferences.menus {
            if menu.id == "home" {
                transaction.execute("UPDATE navigation_menus SET label = ?1, icon_svg = ?2, sort_order = 0 WHERE id = 'home'", params![menu.label, menu.icon_svg]).map_err(|error| error.to_string())?;
            } else if menu.id == "settings" {
                continue;
            } else {
                transaction.execute("UPDATE navigation_menus SET label = ?1, icon_svg = ?2, sort_order = ?3 WHERE id = ?4", params![menu.label, menu.icon_svg, order, menu.id]).map_err(|error| error.to_string())?;
                order += 1;
            }
        }
        let settings = preferences.menus.iter().find(|menu| menu.id == "settings");
        if let Some(menu) = settings {
            transaction.execute("UPDATE navigation_menus SET label = ?1, icon_svg = ?2, sort_order = ?3 WHERE id = 'settings'", params![menu.label, menu.icon_svg, order]).map_err(|error| error.to_string())?;
        } else {
            transaction
                .execute(
                    "UPDATE navigation_menus SET sort_order = ?1 WHERE id = 'settings'",
                    [order],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())?;

        // `preferences` acquires the same mutex. Release the guard before
        // reading the saved values; retaining it here would block the Tauri
        // command indefinitely whenever any preference is changed.
        drop(connection);
        self.preferences()
    }

    pub fn create_menu(&self, menu: NavigationMenu) -> Result<AppPreferences, String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        let order: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) FROM navigation_menus WHERE id <> 'settings'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?
            + 1;
        connection
            .execute(
                "UPDATE navigation_menus SET sort_order = sort_order + 1 WHERE id = 'settings'",
                [],
            )
            .map_err(|error| error.to_string())?;
        connection.execute("INSERT INTO navigation_menus (id, label, icon_svg, sort_order, is_system) VALUES (?1, ?2, ?3, ?4, 0)", params![menu.id, menu.label, menu.icon_svg, order]).map_err(|error| error.to_string())?;
        drop(connection);
        self.preferences()
    }

    pub fn delete_menu(&self, id: String) -> Result<AppPreferences, String> {
        if id == "home" || id == "settings" {
            return Err("首页和设置不能删除。".to_string());
        }
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        let reports: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM records WHERE menu_id = ?1",
                [&id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if reports > 0 {
            return Err("该菜单中仍有报告，无法删除。".to_string());
        }
        connection
            .execute(
                "DELETE FROM navigation_menus WHERE id = ?1 AND is_system = 0",
                [&id],
            )
            .map_err(|error| error.to_string())?;
        drop(connection);
        self.preferences()
    }
}
