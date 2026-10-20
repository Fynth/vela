//! A retryable playlist add is one intent, even if its response is lost. This
//! module owns no transport, authentication, UI or independent playlist state.
use std::collections::HashSet;
use vela_domain::PlaylistDetail;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AddIntent {
    pub playlist_id: String,
    pub track_id: String,
    pub revision: i64,
    pub mutation_id: String,
    original_entries: Vec<String>,
}

impl AddIntent {
    pub fn new(before: &PlaylistDetail, track_id: String) -> Self {
        Self {
            playlist_id: before.playlist.id.clone(),
            track_id,
            revision: before.playlist.revision,
            mutation_id: uuid::Uuid::new_v4().to_string(),
            original_entries: before.items.iter().map(|entry| entry.id.clone()).collect(),
        }
    }

    /// Reuse this exact envelope on uncertain retry. Changing the revision or
    /// mutation id would turn reconciliation into a second add.
    pub fn body(&self) -> serde_json::Value {
        serde_json::json!({
            "revision": self.revision,
            "mutation_id": self.mutation_id,
            "track_id": self.track_id,
        })
    }

    /// The service can return a newer snapshot after an idempotent retry or a
    /// concurrent commit. Only a single-revision, single-entry delta proves the
    /// inverse; the last same-track entry could belong to another collaborator.
    pub fn confirmed_entry(&self, after: &PlaylistDetail) -> Option<String> {
        if after.playlist.id != self.playlist_id
            || self.revision.checked_add(1) != Some(after.playlist.revision)
            || after.items.len() != self.original_entries.len().checked_add(1)?
        {
            return None;
        }
        let original: HashSet<_> = self.original_entries.iter().collect();
        let current: HashSet<_> = after.items.iter().map(|entry| &entry.id).collect();
        if original.len() != self.original_entries.len()
            || current.len() != after.items.len()
            || !original.is_subset(&current)
        {
            return None;
        }
        let mut inserted = after
            .items
            .iter()
            .filter(|entry| !original.contains(&entry.id));
        let entry = inserted.next()?;
        (inserted.next().is_none() && entry.track.id == self.track_id).then(|| entry.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vela_domain::{PlaylistEntry, PlaylistSummary, Track};

    fn detail(revision: i64, entries: &[(&str, &str)]) -> PlaylistDetail {
        PlaylistDetail {
            playlist: PlaylistSummary {
                id: "playlist".into(),
                name: "Real destination".into(),
                owner_id: "owner".into(),
                revision,
                item_count: entries.len() as i64,
                can_edit: true,
                folder_id: None,
                artwork_id: None,
            },
            items: entries
                .iter()
                .enumerate()
                .map(|(at, (id, track))| PlaylistEntry {
                    id: (*id).into(),
                    position: at as i64,
                    can_stream: true,
                    track: Track {
                        id: (*track).into(),
                        title: "Fixture".into(),
                        artist: "".into(),
                        album: "".into(),
                        album_artist: "".into(),
                        genre: "".into(),
                        year: None,
                        track_number: None,
                        disc_number: None,
                        duration_ms: 1000,
                        codec: "wav".into(),
                        bitrate: None,
                        sample_rate: None,
                        bit_depth: None,
                        channels: None,
                        size_bytes: 100,
                        artwork_id: None,
                        added_at: 0,
                        favorite: false,
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn retry_preserves_the_exact_mutation_envelope() {
        let intent = AddIntent::new(&detail(4, &[]), "track".into());
        assert_eq!(intent.body(), intent.clone().body());
        assert_eq!(intent.body()["revision"], 4);
        assert!(uuid::Uuid::parse_str(&intent.mutation_id).is_ok());
    }

    #[test]
    fn duplicate_track_is_identified_by_the_new_entry_not_track_id() {
        let intent = AddIntent::new(&detail(4, &[("old", "track")]), "track".into());
        assert_eq!(
            intent.confirmed_entry(&detail(5, &[("old", "track"), ("mine", "track")])),
            Some("mine".into())
        );
    }

    #[test]
    fn concurrent_snapshot_never_offers_another_actors_entry_for_undo() {
        let intent = AddIntent::new(&detail(4, &[]), "track".into());
        assert_eq!(
            intent.confirmed_entry(&detail(6, &[("mine", "track"), ("theirs", "track")])),
            None
        );
        assert_eq!(
            intent.confirmed_entry(&detail(7, &[("theirs", "track")])),
            None
        );
    }

    #[test]
    fn foreign_replaced_or_malformed_entries_cannot_become_an_inverse() {
        let intent = AddIntent::new(&detail(4, &[("old", "track")]), "track".into());
        assert_eq!(
            intent.confirmed_entry(&detail(5, &[("replacement", "track"), ("mine", "track")])),
            None
        );
        assert_eq!(
            intent.confirmed_entry(&detail(5, &[("old", "track"), ("mine", "other")])),
            None
        );
        assert_eq!(
            intent.confirmed_entry(&detail(5, &[("old", "track"), ("old", "track")])),
            None
        );
        let mut foreign = detail(5, &[("old", "track"), ("mine", "track")]);
        foreign.playlist.id = "foreign".into();
        assert_eq!(intent.confirmed_entry(&foreign), None);
    }
}
