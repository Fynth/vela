//! Framework-independent API contracts. IDs are opaque UUID strings.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: User,
    pub csrf_token: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub needs_setup: bool,
    pub version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration_ms: i64,
    pub codec: String,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
    pub size_bytes: i64,
    pub artwork_id: Option<String>,
    pub added_at: i64,
    pub favorite: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPage {
    pub items: Vec<Track>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub year: Option<i64>,
    pub track_count: i64,
    pub duration_ms: i64,
    pub artwork_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumPage {
    pub items: Vec<AlbumSummary>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub track_count: i64,
    pub album_count: i64,
    pub artwork_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistPage {
    pub items: Vec<ArtistSummary>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub track: Track,
    pub duplicate: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub track: Track,
    pub revision: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateTrackMetadata {
    pub revision: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    #[serde(deserialize_with = "required_nullable")]
    pub year: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub track_number: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub disc_number: Option<i64>,
}
fn required_nullable<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<i64>, D::Error> {
    Option::<i64>::deserialize(deserializer)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub source: String,
    pub url: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size_bytes: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub url: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: String,
    pub url: String,
    pub status: String,
    pub downloaded_bytes: i64,
    pub total_bytes: Option<i64>,
    pub error: Option<String>,
    pub track_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSummary {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub revision: i64,
    pub item_count: i64,
    pub can_edit: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistDetail {
    pub playlist: PlaylistSummary,
    pub items: Vec<PlaylistEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub id: String,
    pub track: Track,
    pub position: i64,
    pub can_stream: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlaylist {
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistMutation {
    pub revision: i64,
    pub mutation_id: String,
    pub track_id: Option<String>,
    pub item_ids: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaboratorRequest {
    pub username: String,
    pub can_edit: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRequest {
    pub expires_in_seconds: i64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ShareResponse {
    pub id: String,
    pub token: String,
    pub expires_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub tracks: i64,
    pub albums: i64,
    pub artists: i64,
    pub size_bytes: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistCollaborator {
    pub user_id: String,
    pub username: String,
    pub can_edit: bool,
}

// Serialization deliberately carries authorized credentials; diagnostics never do.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}
impl std::fmt::Debug for AuthResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthResponse")
            .field("user", &self.user)
            .field("csrf_token", &"[REDACTED]")
            .finish()
    }
}
impl std::fmt::Debug for ImportRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportRequest")
            .field("url", &"[REDACTED]")
            .finish()
    }
}
impl std::fmt::Debug for ImportPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportPreview")
            .field("source", &self.source)
            .field("url", &"[REDACTED]")
            .field("size_bytes", &self.size_bytes)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ImportJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportJob")
            .field("id", &self.id)
            .field("status", &self.status)
            .field("url", &"[REDACTED]")
            .field("downloaded_bytes", &self.downloaded_bytes)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ShareResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShareResponse")
            .field("id", &self.id)
            .field("expires_at", &self.expires_at)
            .field("token", &"[REDACTED]")
            .finish()
    }
}
