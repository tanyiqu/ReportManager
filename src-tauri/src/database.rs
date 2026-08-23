use crate::models::{
    AppPreferences, NavigationMenu, Record, RecordQuery, RecordStatus, RecordType,
};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
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
        connection
            .execute_batch(include_str!("../migrations/0003_close_behavior.sql"))
            .map_err(|error| error.to_string())?;
        // The migration is executed on each startup. Once its columns exist,
        // SQLite reports the expected duplicate-column error; all statements
        // have already completed during the initial successful launch.
        connection
            .execute_batch(include_str!(
                "../migrations/0004_menu_visibility_and_period.sql"
            ))
            .or_else(|error| {
                if error.to_string().contains("duplicate column name") {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(include_str!("../migrations/0005_menu_action_order.sql"))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(include_str!(
                "../migrations/0006_report_workspace_preferences.sql"
            ))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(include_str!("../migrations/0007_editor_font_scale.sql"))
            .map_err(|error| error.to_string())?;
        Self::seed_navigation(&connection)?;
        Self::seed_management_weekly_records(&connection)?;
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
            ("minimize_to_tray", "true"),
            (
                "menu_action_order",
                "[\"visibility\",\"period\",\"rename\",\"icon\",\"delete\"]",
            ),
            ("default_report_load_count", "15"),
            ("refresh_report_load_count", "15"),
            ("editor_font_scale", "1"),
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

    /// Adds the requested twenty rolling weekly samples to the existing
    /// management-weekly menu. The historic `weekly` id is the compatible
    /// fallback when the user has not renamed that menu yet.
    fn seed_management_weekly_records(connection: &Connection) -> Result<(), String> {
        let menu_id = connection
            .query_row(
                "SELECT id FROM navigation_menus WHERE label = '管理层周报' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| "weekly".to_string());
        let current_week_start: String = connection
            .query_row("SELECT date('now', 'localtime', '-' || ((CAST(strftime('%w', 'now', 'localtime') AS INTEGER) + 6) % 7) || ' days')", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        for index in 0..20_i64 {
            let offset = format!("-{} days", index * 7);
            let (start, end, week): (String, String, String) = connection
                .query_row(
                    "SELECT date(?1, ?2), date(?1, ?2, '+6 days'), strftime('%V', date(?1, ?2), '+3 days')",
                    params![current_week_start, offset],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|error| error.to_string())?;
            let format_date = |value: &str| {
                let parts = value.split('-').collect::<Vec<_>>();
                format!(
                    "{}年{}月{}日",
                    parts[0],
                    parts[1].trim_start_matches('0'),
                    parts[2].trim_start_matches('0')
                )
            };
            let week_number = week.parse::<u32>().unwrap_or(0);
            let content = format!(
                "{} - {}，第{}周",
                format_date(&start),
                format_date(&end),
                week_number
            );
            let id = format!("management-weekly-{start}");
            connection.execute(
                "INSERT OR IGNORE INTO records (id, type, record_date, title, content, tags, metadata, status, created_at, updated_at, menu_id) VALUES (?1, 'weekly', ?2, ?3, ?4, '[]', '{}', 'saved', datetime('now', 'localtime'), datetime('now', 'localtime'), ?5)",
                params![id, start, content, content, menu_id],
            ).map_err(|error| error.to_string())?;
        }
        Ok(())
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
        let mut statement = connection.prepare("SELECT id, label, icon_svg, sort_order, is_system, is_hidden, report_period FROM navigation_menus ORDER BY sort_order")
            .map_err(|error| error.to_string())?;
        let menus = statement
            .query_map([], |row| {
                Ok(NavigationMenu {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    icon_svg: row.get(2)?,
                    sort_order: row.get(3)?,
                    is_system: row.get::<_, i64>(4)? != 0,
                    is_hidden: row.get::<_, i64>(5)? != 0,
                    report_period: row.get(6)?,
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
            minimize_to_tray: setting("minimize_to_tray", "true")? == "true",
            default_report_load_count: setting("default_report_load_count", "15")?
                .parse()
                .unwrap_or(15),
            refresh_report_load_count: setting("refresh_report_load_count", "15")?
                .parse()
                .unwrap_or(15),
            editor_font_scale: setting("editor_font_scale", "1")?
                .parse::<f64>()
                .unwrap_or(1.0)
                .clamp(0.8, 1.5),
            menu_action_order: Self::normalize_menu_action_order(&setting(
                "menu_action_order",
                "[\"visibility\",\"period\",\"rename\",\"icon\",\"delete\"]",
            )?),
            menus,
        })
    }

    /// Keeps persisted action button preferences forwards-compatible and safe
    /// when an older database contains malformed or incomplete JSON.
    fn normalize_menu_action_order(value: &str) -> Vec<String> {
        const DEFAULT: [&str; 5] = ["visibility", "period", "rename", "icon", "delete"];
        let saved = serde_json::from_str::<Vec<String>>(value).unwrap_or_default();
        let mut normalized = saved
            .into_iter()
            .filter(|item| DEFAULT.contains(&item.as_str()))
            .fold(Vec::new(), |mut result, item| {
                if !result.contains(&item) {
                    result.push(item);
                }
                result
            });
        for item in DEFAULT {
            if !normalized.iter().any(|saved| saved == item) {
                normalized.push(item.to_string());
            }
        }
        normalized
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
            ("minimize_to_tray", preferences.minimize_to_tray.to_string()),
            (
                "default_report_load_count",
                preferences
                    .default_report_load_count
                    .clamp(1, 100)
                    .to_string(),
            ),
            (
                "refresh_report_load_count",
                preferences
                    .refresh_report_load_count
                    .clamp(1, 100)
                    .to_string(),
            ),
            (
                "editor_font_scale",
                preferences.editor_font_scale.clamp(0.8, 1.5).to_string(),
            ),
            (
                "menu_action_order",
                serde_json::to_string(&Self::normalize_menu_action_order(
                    &serde_json::to_string(&preferences.menu_action_order)
                        .map_err(|error| error.to_string())?,
                ))
                .map_err(|error| error.to_string())?,
            ),
        ] {
            transaction.execute("INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value]).map_err(|error| error.to_string())?;
        }
        // First move mutable entries out of the unique sort-order range, then
        // write their new order. This makes swaps (for example 日报 and 周报)
        // safe under the unique index.
        transaction
            .execute(
                "UPDATE navigation_menus SET sort_order = -sort_order - 1 WHERE id NOT IN ('home', 'settings')",
                [],
            )
            .map_err(|error| error.to_string())?;
        let mut order = 1_i64;
        for menu in &preferences.menus {
            if menu.id == "home" {
                transaction.execute("UPDATE navigation_menus SET label = ?1, icon_svg = ?2, is_hidden = 0, sort_order = 0 WHERE id = 'home'", params![menu.label, menu.icon_svg]).map_err(|error| error.to_string())?;
            } else if menu.id == "settings" {
                continue;
            } else {
                transaction.execute("UPDATE navigation_menus SET label = ?1, icon_svg = ?2, is_hidden = ?3, report_period = ?4, sort_order = ?5 WHERE id = ?6", params![menu.label, menu.icon_svg, menu.is_hidden, menu.report_period, order, menu.id]).map_err(|error| error.to_string())?;
                order += 1;
            }
        }
        let settings = preferences.menus.iter().find(|menu| menu.id == "settings");
        if let Some(menu) = settings {
            transaction.execute("UPDATE navigation_menus SET label = ?1, icon_svg = ?2, is_hidden = 0, sort_order = ?3 WHERE id = 'settings'", params![menu.label, menu.icon_svg, order]).map_err(|error| error.to_string())?;
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
        connection.execute("INSERT INTO navigation_menus (id, label, icon_svg, sort_order, is_system, is_hidden, report_period) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)", params![menu.id, menu.label, menu.icon_svg, order, menu.is_hidden, menu.report_period]).map_err(|error| error.to_string())?;
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

    fn record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
        let record_type = match row.get::<_, String>(1)?.as_str() {
            "weekly" => RecordType::Weekly,
            "meeting" => RecordType::Meeting,
            _ => RecordType::Daily,
        };
        let status = if row.get::<_, String>(7)? == "saved" {
            RecordStatus::Saved
        } else {
            RecordStatus::Draft
        };
        Ok(Record {
            id: row.get(0)?,
            record_type,
            record_date: row.get(2)?,
            title: row.get(3)?,
            content: row.get(4)?,
            tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
            metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
            status,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            menu_id: row.get(10)?,
        })
    }

    pub fn list_records(&self, query: RecordQuery) -> Result<Vec<Record>, String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        let mut sql = "SELECT id, type, record_date, title, content, tags, metadata, status, created_at, updated_at, menu_id FROM records WHERE 1 = 1".to_string();
        let mut values: Vec<Value> = Vec::new();
        if let Some(menu_id) = query.menu_id {
            sql.push_str(" AND menu_id = ?");
            values.push(menu_id.into());
        }
        if let Some(record_type) = query.record_type {
            sql.push_str(" AND type = ?");
            values.push(
                match record_type {
                    RecordType::Daily => "daily",
                    RecordType::Weekly => "weekly",
                    RecordType::Meeting => "meeting",
                }
                .to_string()
                .into(),
            );
        }
        if let Some(keyword) = query.keyword.filter(|value| !value.trim().is_empty()) {
            sql.push_str(" AND (title LIKE ? OR content LIKE ? OR tags LIKE ?)");
            let pattern = format!("%{}%", keyword.trim());
            values.extend([
                pattern.clone().into(),
                pattern.clone().into(),
                pattern.into(),
            ]);
        }
        if let Some(date_from) = query.date_from.filter(|value| !value.is_empty()) {
            sql.push_str(" AND record_date >= ?");
            values.push(date_from.into());
        }
        if let Some(date_to) = query.date_to.filter(|value| !value.is_empty()) {
            sql.push_str(" AND record_date <= ?");
            values.push(date_to.into());
        }
        if let Some(tags) = query.tags {
            for tag in tags.into_iter().filter(|value| !value.trim().is_empty()) {
                sql.push_str(" AND tags LIKE ?");
                values.push(format!("%{}%", tag.trim()).into());
            }
        }
        sql.push_str(" ORDER BY record_date DESC, updated_at DESC LIMIT ? OFFSET ?");
        values.push(i64::from(query.limit.unwrap_or(15).clamp(1, 100)).into());
        values.push(i64::from(query.offset.unwrap_or(0)).into());
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| error.to_string())?;
        let records = statement
            .query_map(params_from_iter(values), Self::record_from_row)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(records)
    }

    pub fn get_record(&self, id: &str) -> Result<Option<Record>, String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        connection.query_row("SELECT id, type, record_date, title, content, tags, metadata, status, created_at, updated_at, menu_id FROM records WHERE id = ?1", [id], Self::record_from_row).optional().map_err(|error| error.to_string())
    }

    pub fn save_record(&self, record: Record) -> Result<Record, String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        let record_type = match record.record_type {
            RecordType::Daily => "daily",
            RecordType::Weekly => "weekly",
            RecordType::Meeting => "meeting",
        };
        let status = match record.status {
            RecordStatus::Draft => "draft",
            RecordStatus::Saved => "saved",
        };
        connection.execute("INSERT INTO records (id, type, record_date, title, content, tags, metadata, status, created_at, updated_at, menu_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) ON CONFLICT(id) DO UPDATE SET type=excluded.type, record_date=excluded.record_date, title=excluded.title, content=excluded.content, tags=excluded.tags, metadata=excluded.metadata, status=excluded.status, updated_at=excluded.updated_at, menu_id=excluded.menu_id", params![record.id, record_type, record.record_date, record.title, record.content, serde_json::to_string(&record.tags).map_err(|error| error.to_string())?, record.metadata.to_string(), status, record.created_at, record.updated_at, record.menu_id]).map_err(|error| error.to_string())?;
        drop(connection);
        self.get_record(&record.id)?
            .ok_or_else(|| "保存后无法读取报告。".to_string())
    }

    /// Permanently removes one report after the UI has obtained confirmation.
    pub fn delete_record(&self, id: &str) -> Result<(), String> {
        let connection = self.0.lock().map_err(|error| error.to_string())?;
        connection
            .execute("DELETE FROM records WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_database() -> Database {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        Database::open(std::env::temp_dir().join(format!("report-manager-test-{suffix}")))
            .expect("open test database")
    }

    fn weekly_query(limit: u32, offset: u32) -> RecordQuery {
        RecordQuery {
            menu_id: Some("weekly".to_string()),
            record_type: None,
            keyword: None,
            date_from: None,
            date_to: None,
            tags: None,
            limit: Some(limit),
            offset: Some(offset),
        }
    }

    #[test]
    fn seeds_and_pages_twenty_weekly_reports() {
        let database = test_database();
        let first_page = database
            .list_records(weekly_query(15, 0))
            .expect("first page");
        let second_page = database
            .list_records(weekly_query(15, 15))
            .expect("second page");
        assert_eq!(first_page.len(), 15);
        assert_eq!(second_page.len(), 5);
        assert!(first_page[0].content.contains("，第"));
        assert!(first_page[0].record_date > first_page[1].record_date);
    }

    #[test]
    fn saves_and_searches_markdown_content() {
        let database = test_database();
        let mut record = database
            .list_records(weekly_query(1, 0))
            .expect("record")
            .remove(0);
        record.content = "## 已完成\n\n- 分页查询".to_string();
        record.updated_at = "2026-08-22T12:00:00Z".to_string();
        database.save_record(record).expect("save record");
        let mut query = weekly_query(15, 0);
        query.keyword = Some("分页查询".to_string());
        let matches = database.list_records(query).expect("search record");
        assert_eq!(matches.len(), 1);
        assert!(matches[0].content.starts_with("## 已完成"));
    }

    #[test]
    fn deletes_a_saved_record() {
        let database = test_database();
        let id = database
            .list_records(weekly_query(1, 0))
            .expect("record")
            .remove(0)
            .id;
        database.delete_record(&id).expect("delete record");
        assert!(database.get_record(&id).expect("get record").is_none());
    }
}
