//! Versioned, account-bound browser bookmarks. No credentials or stream URLs.
use serde::{Deserialize, Serialize};
use vela_domain::Track;

pub const SNAPSHOT_VERSION: u8 = 1;
pub const MAX_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
const MAX_POSITION: f64 = 7.0 * 24.0 * 3600.0;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    Library,
    Favorites,
    Home,
    History,
    Radio,
    Search,
    Album,
    Artist,
    Playlist,
    Queue,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlaybackContext {
    pub kind: ContextKind,
    pub label: String,
}
impl Default for PlaybackContext {
    fn default() -> Self {
        Self::new(ContextKind::Queue, "Playback queue")
    }
}
impl PlaybackContext {
    pub fn new(kind: ContextKind, label: &str) -> Self {
        let label: String = label
            .chars()
            .filter(|c| !c.is_control())
            .scan(0, |len, c| {
                *len += c.len_utf8();
                (*len <= 512).then_some(c)
            })
            .collect();
        Self { kind, label }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSession {
    pub version: u8,
    pub owner_id: String,
    pub queue: Vec<Track>,
    pub cursor: usize,
    pub current_track_id: Option<String>,
    pub position: f64,
    pub context: PlaybackContext,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat: String,
}
fn uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.hyphenated().to_string() == value)
}
impl SavedSession {
    fn valid(&self, owner: &str) -> bool {
        self.version == SNAPSHOT_VERSION
            && self.owner_id == owner
            && uuid(owner)
            && self.queue.len() <= crate::playback::MAX_QUEUE
            && self.position.is_finite()
            && (0.0..=MAX_POSITION).contains(&self.position)
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
            && matches!(self.repeat.as_str(), "off" | "all" | "one")
            && self.context.label.len() <= 512
            && !self.context.label.chars().any(char::is_control)
            && self
                .queue
                .iter()
                .all(|t| uuid(&t.id) && t.artwork_id.as_deref().is_none_or(uuid))
            && if self.queue.is_empty() {
                self.cursor == 0 && self.current_track_id.is_none() && self.position == 0.0
            } else {
                self.queue
                    .get(self.cursor)
                    .is_some_and(|track| Some(&track.id) == self.current_track_id.as_ref())
            }
    }
    pub fn decode(raw: &str, expected_owner: &str) -> Option<Self> {
        if raw.len() > MAX_SNAPSHOT_BYTES {
            return None;
        }
        let value: Self = serde_json::from_str(raw).ok()?;
        value.valid(expected_owner).then_some(value)
    }
    pub fn encode(&self) -> Option<String> {
        if !self.valid(&self.owner_id) {
            return None;
        }
        // A bounded writer also bounds temporary serialized output.
        struct Limited(Vec<u8>);
        impl std::io::Write for Limited {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if self.0.len().saturating_add(buf.len()) > MAX_SNAPSHOT_BYTES {
                    return Err(std::io::Error::other("bookmark too large"));
                }
                self.0.extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut out = Limited(Vec::new());
        serde_json::to_writer(&mut out, self).ok()?;
        String::from_utf8(out.0).ok()
    }
}
pub fn clamp_position(position: f64, duration: f64) -> f64 {
    if !position.is_finite()
        || !(0.0..=MAX_POSITION).contains(&position)
        || (duration.is_finite() && duration > 0.0 && position >= duration)
    {
        0.0
    } else {
        position
    }
}

#[derive(Default)]
pub struct RetryBudget(usize);
impl RetryBudget {
    pub fn next_delay(&mut self) -> Option<u32> {
        let value = [500, 1500, 4000].get(self.0).copied();
        self.0 = self.0.saturating_add(1);
        value
    }
    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn saved() -> SavedSession {
        SavedSession {
            version: 1,
            owner_id: uuid::Uuid::new_v4().to_string(),
            queue: vec![],
            cursor: 0,
            current_track_id: None,
            position: 0.0,
            context: PlaybackContext::default(),
            volume: 0.8,
            shuffle: false,
            repeat: "off".into(),
        }
    }
    #[test]
    fn rejects_foreign_unknown_malformed_and_oversized_bookmarks() {
        let value = saved();
        let raw = value.encode().unwrap();
        assert!(SavedSession::decode(&raw, &value.owner_id).is_some());
        assert!(SavedSession::decode(&raw, &uuid::Uuid::new_v4().to_string()).is_none());
        assert!(
            SavedSession::decode(
                &raw.replace("\"version\":1", "\"version\":2"),
                &value.owner_id
            )
            .is_none()
        );
        assert!(
            SavedSession::decode(&" ".repeat(MAX_SNAPSHOT_BYTES + 1), &value.owner_id).is_none()
        );
        assert!(SavedSession::decode("{", &value.owner_id).is_none());
        let mut invalid = value.clone();
        invalid.position = 1.0;
        assert!(invalid.encode().is_none());
        invalid = value.clone();
        invalid.volume = f64::NAN;
        assert!(invalid.encode().is_none());
        invalid = value.clone();
        invalid.cursor = 1;
        assert!(invalid.encode().is_none());
        invalid = value;
        invalid.context.label = "x".repeat(513);
        assert!(invalid.encode().is_none());
    }
    #[test]
    fn occurrence_binding_and_ids_are_validated() {
        let mut value = saved();
        let track: Track = serde_json::from_value(serde_json::json!({"id":uuid::Uuid::new_v4().to_string(),"title":"Fixture","artist":"Test","album":"Test","album_artist":"Test","genre":"Test","year":null,"track_number":null,"disc_number":null,"duration_ms":30000,"codec":"WAV","bitrate":null,"sample_rate":44100,"bit_depth":16,"channels":1,"size_bytes":2646044,"artwork_id":null,"added_at":0,"favorite":false})).unwrap();
        value.queue = vec![track.clone(), track.clone()];
        value.cursor = 1;
        value.current_track_id = Some(track.id);
        value.position = 8.0;
        assert!(value.encode().is_some());
        value.cursor = 2;
        assert!(value.encode().is_none());
        value.cursor = 1;
        value.current_track_id = Some(uuid::Uuid::new_v4().to_string());
        assert!(value.encode().is_none());
        value.current_track_id = Some(value.queue[1].id.clone());
        value.queue[0].id = "../private".into();
        assert!(value.encode().is_none());
        value.queue[0].id = value.queue[1].id.clone();
        value.queue[0].artwork_id = Some("not-a-uuid".into());
        assert!(value.encode().is_none());
        value.queue[0].artwork_id = None;
        value.queue[0].title = "\"".repeat(MAX_SNAPSHOT_BYTES / 2);
        assert!(
            value.encode().is_none(),
            "JSON escaping must respect the output bound"
        );
        value.queue[0].title = "Fixture".into();
        value.queue = vec![value.queue[0].clone(); 1001];
        assert!(value.encode().is_none());
    }
    #[test]
    fn seeks_and_retry_budget_are_bounded() {
        for p in [f64::NAN, f64::INFINITY, -1.0, MAX_POSITION + 1.0, 30.0] {
            assert_eq!(clamp_position(p, 30.0), 0.0);
        }
        assert_eq!(clamp_position(8.0, 30.0), 8.0);
        assert_eq!(clamp_position(8.0, f64::NAN), 8.0);
        let mut budget = RetryBudget::default();
        assert_eq!(
            (0..5).map(|_| budget.next_delay()).collect::<Vec<_>>(),
            vec![Some(500), Some(1500), Some(4000), None, None]
        );
        budget.reset();
        assert_eq!(budget.next_delay(), Some(500));
        assert!(
            PlaybackContext::new(ContextKind::Album, &"я".repeat(400))
                .label
                .len()
                <= 512
        );
    }
}
