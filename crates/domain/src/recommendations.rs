//! Owner-private, on-demand recommendations. No public popularity or rights grant.
use crate::RadioTrack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RecommendationQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendations {
    pub algorithm_version: String,
    pub generated_at: i64,
    pub privacy_revision: i64,
    pub history_enabled: bool,
    pub candidate_count: u32,
    pub anchor_count: u32,
    pub hidden_count: u32,
    pub uses_audio_features: bool,
    pub discovery: Vec<RadioTrack>,
    pub rotation: Vec<RadioTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendationFeedback {
    pub track_id: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationFeedbackReceipt {
    pub hidden_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationExclusions {
    pub format_version: u8,
    pub track_ids: Vec<String>,
}
