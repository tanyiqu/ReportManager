use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordType {
    Daily,
    Weekly,
    Meeting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Draft,
    Saved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub record_type: RecordType,
    pub record_date: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub status: RecordStatus,
    pub created_at: String,
    pub updated_at: String,
    pub menu_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordQuery {
    pub menu_id: Option<String>,
    pub record_type: Option<RecordType>,
    pub keyword: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A user-visible navigation entry. `is_system` protects the fixed Home and
/// Settings entries from changes that would make the application unusable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationMenu {
    pub id: String,
    pub label: String,
    pub icon_svg: String,
    pub sort_order: i64,
    pub is_system: bool,
    pub is_hidden: bool,
    pub report_period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub sidebar_collapsed: bool,
    pub default_page_id: String,
    pub week_start: String,
    pub export_directory: String,
    pub minimize_to_tray: bool,
    pub default_report_load_count: u32,
    pub refresh_report_load_count: u32,
    /// Font scale used by all Vditor report editors (0.8 through 1.5).
    pub editor_font_scale: f64,
    /// Vditor display mode used by every report editor.
    /// Valid values are `wysiwyg`, `ir`, and `sv`.
    pub editor_mode: String,
    /// Display order for the per-menu action buttons in Menu Management.
    /// Stored independently from navigation menu ordering.
    pub menu_action_order: Vec<String>,
    pub menus: Vec<NavigationMenu>,
}
