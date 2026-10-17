//! Versioned content descriptors and private analysis/radio API contracts.
use crate::Track;
use serde::{Deserialize, Serialize};

/// Content features only: no source path, owner, credentials or listening data.
/// The caller must key durable results by content hash AND extractor_version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioFeatures {
    pub extractor_version: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub analyzed_frames: u64,
    pub analyzed_ms: u64,
    pub complete: bool,
    /// Normalized digital PCM amplitude; this is not LUFS or perceived loudness.
    pub rms: f64,
    pub peak: f64,
    /// None for digital silence, whose decibel value would be negative infinity.
    pub rms_dbfs: Option<f64>,
    pub peak_dbfs: Option<f64>,
    /// Mean per-channel sign-change fraction, in 0..=1, not Hertz.
    pub zero_crossing_rate: f64,
    pub spectral_windows: u64,
    /// Spectral fields are None for silence; no invented zero-Hertz timbre.
    pub spectral_centroid_hz: Option<f64>,
    pub spectral_bandwidth_hz: Option<f64>,
    pub spectral_rolloff_hz: Option<f64>,
    pub spectral_flatness: Option<f64>,
    /// Phase-safe mean-channel power, normalized across the eight fixed-Hz bands.
    pub band_energy: Option<[f64; 8]>,
    /// Not inferred by this extractor; no calibrated tempo/key/genre model yet.
    pub tempo_bpm: Option<f64>,
    pub tempo_confidence: Option<f64>,
    pub chroma: Option<[f64; 12]>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    NotAnalyzed,
    Queued,
    Running,
    Complete,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub track_id: String,
    pub extractor_version: String,
    pub status: AnalysisStatus,
    pub features: Option<AudioFeatures>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnqueueAnalysis {
    pub track_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReceipt {
    pub queued: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisLibraryStatus {
    pub total: i64,
    pub not_analyzed: i64,
    pub queued: i64,
    pub running: i64,
    pub complete: i64,
    pub failed: i64,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RadioQuery {
    pub limit: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioTrack {
    pub track: Track,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackRadio {
    pub seed: Track,
    pub items: Vec<RadioTrack>,
    pub uses_audio_features: bool,
    pub candidate_count: u32,
}
