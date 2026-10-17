//! Private, consent-controlled listening contracts. No credentials or file URLs.
use crate::Track;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListeningPreferences {
    pub enabled: bool,
    pub retention_days: u16,
    pub revision: i64,
    pub device_id: String,
    pub device_enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateListeningPreferences {
    pub revision: i64,
    pub enabled: bool,
    pub retention_days: u16,
    pub device_id: String,
    pub device_enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListeningProgress {
    pub session_id: String,
    pub sequence: u32,
    pub track_id: String,
    pub listened_ms: i64,
    pub position_ms: i64,
    pub started_at: i64,
    pub ended: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListeningBatch {
    pub device_id: String,
    pub revision: i64,
    pub events: Vec<ListeningProgress>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningReceipt {
    pub accepted: u32,
    pub duplicates: u32,
    pub revision: i64,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListeningHistoryQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningEntry {
    pub session_id: String,
    pub track: Track,
    pub listened_ms: i64,
    pub position_ms: i64,
    pub started_at: i64,
    pub updated_at: i64,
    pub ended: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningHistoryPage {
    pub items: Vec<ListeningEntry>,
    pub next_cursor: Option<String>,
    pub total_listens: i64,
    pub total_listened_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningExportEntry {
    pub device_id: String,
    pub progress: ListeningProgress,
    pub updated_at: i64,
    pub qualified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningExport {
    pub format_version: u8,
    pub exported_at: i64,
    pub enabled: bool,
    pub retention_days: u16,
    pub events: Vec<ListeningExportEntry>,
}
