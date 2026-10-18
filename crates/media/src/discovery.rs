//! Deterministic, bounded radio over the caller's actually owned catalog.
//! Derived audio descriptors are read-only here: radio never decodes media,
//! starts analysis or treats playlist metadata as a media access grant.
use std::collections::HashMap;

use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite, sqlite::SqliteRow};
use vela_domain::{AudioFeatures, RadioTrack, Track, TrackRadio};

use crate::{MediaService, TRACK_SELECT, analysis::EXTRACTOR_VERSION, track_from_row};

const MAX_CANDIDATES: i64 = 500;

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Radio size must be 1–50 and the seed must be a valid track UUID")]
    Invalid,
    #[error("Track not found")]
    NotFound,
}

impl MediaService {
    /// Candidate coverage is explicitly limited to the500 newest other owned
    /// tracks. This bounded baseline is not a whole-library similarity index.
    pub async fn track_radio(&self, owner: &str, track_id: &str, limit: u32) -> Result<TrackRadio> {
        if !(1..=50).contains(&limit)
            || !uuid::Uuid::parse_str(track_id).is_ok_and(|id| id.to_string() == track_id)
        {
            return Err(DiscoveryError::Invalid.into());
        }
        let mut tx = self.pool.begin().await?;
        let mut seed_query = query(owner);
        seed_query
            .push(" WHERE t.owner_id=")
            .push_bind(owner)
            .push(" AND t.id=")
            .push_bind(track_id)
            .push(" AND a.size_bytes>0 AND t.duration_ms>0");
        let row = seed_query
            .build()
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(DiscoveryError::NotFound)?;
        let seed = track_from_row(&row)?;
        let seed_features = features(&row);
        let mut candidates_query = query(owner);
        candidates_query
            .push(" WHERE t.owner_id=")
            .push_bind(owner)
            .push(" AND t.id<>")
            .push_bind(track_id)
            .push(
                " AND a.size_bytes>0 AND t.duration_ms>0 ORDER BY t.added_at DESC,t.id DESC LIMIT ",
            )
            .push_bind(MAX_CANDIDATES);
        let rows = candidates_query.build().fetch_all(&mut *tx).await?;
        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let track = track_from_row(&row)?;
            let candidate_features = features(&row);
            candidates.push(rank(
                &seed,
                seed_features.as_ref(),
                track,
                candidate_features.as_ref(),
            ));
        }
        tx.commit().await?;
        let candidate_count = candidates.len() as u32;
        let (items, uses_audio_features) = diversify(candidates, limit as usize);
        Ok(TrackRadio {
            seed,
            items,
            uses_audio_features,
            candidate_count,
        })
    }
}

pub(super) fn query(owner: &str) -> QueryBuilder<'_, Sqlite> {
    let mut builder = QueryBuilder::new("SELECT aa.features_json, ");
    // Preserve the shared authorized Track projection instead of maintaining a
    // second DTO query. TRACK_SELECT ends at the favorite owner's parameter.
    builder
        .push(&TRACK_SELECT["SELECT ".len()..])
        .push_bind(owner)
        .push(" LEFT JOIN audio_analysis aa ON aa.media_asset_id=a.id AND aa.extractor_version=")
        .push_bind(EXTRACTOR_VERSION)
        .push(" AND aa.status='complete'");
    builder
}

pub(super) fn features(row: &SqliteRow) -> Option<AudioFeatures> {
    let json: Option<String> = row.try_get("features_json").ok()?;
    let json = json.filter(|json| json.len() <= 16_384)?;
    let features: AudioFeatures = serde_json::from_str(&json).ok()?;
    valid_features(&features).then_some(features)
}

fn valid_features(f: &AudioFeatures) -> bool {
    let bounded =
        |value: f64, lower: f64, upper: f64| value.is_finite() && (lower..=upper).contains(&value);
    if f.extractor_version != EXTRACTOR_VERSION
        || !(8_000..=192_000).contains(&f.sample_rate)
        || !(1..=8).contains(&f.channels)
        || !(1..=115_200_000).contains(&f.analyzed_frames)
        || !(1..=600_000).contains(&f.analyzed_ms)
        || !bounded(f.rms, 0.0, 16.0)
        || !bounded(f.peak, f.rms, 16.0)
        || !bounded(f.zero_crossing_rate, 0.0, 1.0)
    {
        return false;
    }
    for (value, amplitude) in [(f.rms_dbfs, f.rms), (f.peak_dbfs, f.peak)] {
        match value {
            Some(db)
                if amplitude > 0.0
                    && db.is_finite()
                    && (db - 20.0 * amplitude.log10()).abs() < 0.01 => {}
            None if amplitude == 0.0 => {}
            _ => return false,
        }
    }
    let nyquist = f64::from(f.sample_rate) / 2.0;
    if [
        f.spectral_centroid_hz,
        f.spectral_bandwidth_hz,
        f.spectral_rolloff_hz,
    ]
    .iter()
    .any(|value| value.is_some_and(|value| !bounded(value, 0.0, nyquist)))
        || f.spectral_flatness
            .is_some_and(|value| !bounded(value, 0.0, 1.0))
    {
        return false;
    }
    if let Some(bands) = f.band_energy
        && (bands.iter().any(|&value| !bounded(value, 0.0, 1.0))
            || (bands.iter().sum::<f64>() - 1.0).abs() > 0.02)
    {
        return false;
    }
    true
}

/// A descriptor distance, not a calibrated perceptual/genre/mood classifier.
/// Frequencies stay in Hertz across different source sample rates. Four octaves
/// and48dB span the normalized frequency and digital-amplitude components.
fn audio_distance(left: &AudioFeatures, right: &AudioFeatures) -> Option<f64> {
    if !valid_features(left)
        || !valid_features(right)
        || left.rms <= 1e-8
        || right.rms <= 1e-8
        || left.spectral_windows == 0
        || right.spectral_windows == 0
    {
        return None;
    }
    let octave = |a: f64, b: f64| ((a + 1.0).log2() - (b + 1.0).log2()).abs().min(4.0) / 4.0;
    let centroid = octave(left.spectral_centroid_hz?, right.spectral_centroid_hz?);
    let rolloff = octave(left.spectral_rolloff_hz?, right.spectral_rolloff_hz?);
    let left_bands = left.band_energy?;
    let right_bands = right.band_energy?;
    let bands = left_bands
        .iter()
        .zip(right_bands)
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>()
        / 2.0;
    let rms = (left.rms_dbfs? - right.rms_dbfs?).abs().min(48.0) / 48.0;
    Some((0.25 * centroid + 0.20 * rolloff + 0.40 * bands + 0.15 * rms).clamp(0.0, 1.0))
}

pub(super) struct Candidate {
    pub(super) track: Track,
    pub(super) score: f64,
    pub(super) reason: String,
    pub(super) audio: bool,
    artist: String,
    album: String,
}
fn same_tag(left: &str, right: &str) -> bool {
    !left.trim().is_empty() && left.trim().to_lowercase() == right.trim().to_lowercase()
}
pub(super) fn rank(
    seed: &Track,
    seed_features: Option<&AudioFeatures>,
    track: Track,
    candidate_features: Option<&AudioFeatures>,
) -> Candidate {
    let same_artist = same_tag(&seed.artist, &track.artist);
    let same_genre = same_tag(&seed.genre, &track.genre);
    let year_affinity = seed
        .year
        .zip(track.year)
        .map(|(a, b)| 1.0 - (a.abs_diff(b) as f64 / 20.0).min(1.0))
        .unwrap_or(0.0);
    let metadata = f64::from(u8::from(same_artist)) * 0.65
        + f64::from(u8::from(same_genre)) * 0.30
        + year_affinity * 0.05;
    let distance = seed_features
        .zip(candidate_features)
        .and_then(|(a, b)| audio_distance(a, b));
    let score = distance
        .map(|value| 0.75 * (1.0 - value) + 0.25 * metadata)
        .unwrap_or(0.40 * metadata);
    let reason = if distance.is_some() {
        "Similar measured spectrum and signal level"
    } else if same_artist {
        "Same artist in your library"
    } else if same_genre {
        "Same genre tag in your library"
    } else if year_affinity > 0.0 {
        "Nearby release year in your library"
    } else {
        "Another available track in your library"
    };
    let artist = track.artist.trim().to_lowercase();
    let album = if track.album.trim().is_empty() {
        String::new()
    } else {
        format!(
            "{}\n{}",
            if track.album_artist.trim().is_empty() {
                &track.artist
            } else {
                &track.album_artist
            }
            .trim()
            .to_lowercase(),
            track.album.trim().to_lowercase()
        )
    };
    Candidate {
        track,
        score,
        reason: reason.into(),
        audio: distance.is_some(),
        artist,
        album,
    }
}

pub(super) fn diversify(mut candidates: Vec<Candidate>, limit: usize) -> (Vec<RadioTrack>, bool) {
    let mut artists = HashMap::<String, u32>::new();
    let mut albums = HashMap::<String, u32>::new();
    let mut items = Vec::with_capacity(limit.min(candidates.len()));
    let mut uses_audio = false;
    while items.len() < limit && !candidates.is_empty() {
        let utility = |candidate: &Candidate| {
            candidate.score
                - f64::from(*artists.get(&candidate.artist).unwrap_or(&0)) * 0.14
                - f64::from(*albums.get(&candidate.album).unwrap_or(&0)) * 0.10
        };
        let index = candidates
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                utility(left)
                    .total_cmp(&utility(right))
                    .then_with(|| right.track.id.cmp(&left.track.id))
            })
            .map(|(index, _)| index)
            .expect("nonempty bounded candidates");
        let candidate = candidates.swap_remove(index);
        if !candidate.artist.is_empty() {
            *artists.entry(candidate.artist).or_default() += 1;
        }
        if !candidate.album.is_empty() {
            *albums.entry(candidate.album).or_default() += 1;
        }
        uses_audio |= candidate.audio;
        items.push(RadioTrack {
            track: candidate.track,
            reason: candidate.reason,
        });
    }
    (items, uses_audio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    struct Fixture {
        _directory: tempfile::TempDir,
        pool: sqlx::SqlitePool,
        media: MediaService,
    }
    impl Fixture {
        async fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let pool = SqlitePoolOptions::new()
                .max_connections(2)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(directory.path().join("radio.db"))
                        .create_if_missing(true)
                        .foreign_keys(true),
                )
                .await
                .unwrap();
            sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
            for owner in ["alice", "bob"] {
                sqlx::query(
                    "INSERT INTO users(id,username,password_hash) VALUES(?,?,'test-fixture')",
                )
                .bind(owner)
                .bind(owner)
                .execute(&pool)
                .await
                .unwrap();
            }
            Self {
                media: MediaService::new(pool.clone(), directory.path().to_owned(), 1024),
                pool,
                _directory: directory,
            }
        }
        async fn add(
            &self,
            owner: &str,
            artist: &str,
            album: &str,
            genre: &str,
            features: Option<AudioFeatures>,
            added_at: i64,
        ) -> String {
            let id = uuid::Uuid::new_v4().to_string();
            let asset = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',1024)")
                .bind(&asset).bind(&asset).bind("private-original").execute(&self.pool).await.unwrap();
            sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,genre,duration_ms,codec,added_at) VALUES(?,?,?,'Real catalog fixture',?,?,?,120000,'FLAC',?)")
                .bind(&id).bind(owner).bind(&asset).bind(artist).bind(album).bind(genre).bind(added_at).execute(&self.pool).await.unwrap();
            if let Some(features) = features {
                sqlx::query("INSERT INTO audio_analysis(media_asset_id,extractor_version,status,features_json,created_at,updated_at) VALUES(?,?,'complete',?,0,0)")
                    .bind(asset).bind(EXTRACTOR_VERSION).bind(serde_json::to_string(&features).unwrap()).execute(&self.pool).await.unwrap();
            }
            id
        }
    }
    fn tone(hz: f64, rms: f64) -> AudioFeatures {
        let mut bands = [0.0; 8];
        let band = crate::analysis::BAND_UPPER_HZ
            .iter()
            .position(|upper| hz < *upper)
            .unwrap_or(7);
        bands[band] = 1.0;
        AudioFeatures {
            extractor_version: EXTRACTOR_VERSION.into(),
            sample_rate: 48_000,
            channels: 2,
            analyzed_frames: 48_000,
            analyzed_ms: 1000,
            complete: true,
            rms,
            peak: rms * 2.0_f64.sqrt(),
            rms_dbfs: Some(20.0 * rms.log10()),
            peak_dbfs: Some(20.0 * (rms * 2.0_f64.sqrt()).log10()),
            zero_crossing_rate: 2.0 * hz / 48_000.0,
            spectral_windows: 45,
            spectral_centroid_hz: Some(hz),
            spectral_bandwidth_hz: Some(25.0),
            spectral_rolloff_hz: Some(hz + 50.0),
            spectral_flatness: Some(0.01),
            band_energy: Some(bands),
            tempo_bpm: None,
            tempo_confidence: None,
            chroma: None,
            key: None,
            key_confidence: None,
        }
    }

    #[test]
    fn distance_is_finite_deterministic_and_reflects_frequency_and_signal_level() {
        let seed = tone(1000.0, 0.3);
        assert_eq!(audio_distance(&seed, &seed), Some(0.0));
        let nearby = audio_distance(&seed, &tone(1100.0, 0.3)).unwrap();
        let far = audio_distance(&seed, &tone(8000.0, 0.3)).unwrap();
        assert!(nearby < far);
        assert!(
            audio_distance(&seed, &tone(1000.0, 0.2)).unwrap()
                < audio_distance(&seed, &tone(1000.0, 0.002)).unwrap()
        );
        let mut other_rate = seed.clone();
        other_rate.sample_rate = 44_100;
        assert_eq!(audio_distance(&seed, &other_rate), Some(0.0));
        let mut corrupt = seed.clone();
        corrupt.spectral_centroid_hz = Some(f64::NAN);
        assert_eq!(audio_distance(&seed, &corrupt), None);
        corrupt = seed.clone();
        corrupt.band_energy = Some([1.0; 8]);
        assert_eq!(audio_distance(&seed, &corrupt), None);
        corrupt = seed.clone();
        corrupt.rms_dbfs = Some(100.0);
        assert_eq!(audio_distance(&seed, &corrupt), None);
        let mut silence = seed.clone();
        silence.rms = 0.0;
        silence.peak = 0.0;
        silence.rms_dbfs = None;
        silence.peak_dbfs = None;
        silence.spectral_centroid_hz = None;
        silence.spectral_rolloff_hz = None;
        silence.band_energy = None;
        assert_eq!(audio_distance(&seed, &silence), None);
    }

    #[tokio::test]
    async fn real_owned_radio_uses_measured_features_and_is_reproducible() {
        let fixture = Fixture::new().await;
        let seed = fixture
            .add("alice", "", "", "", Some(tone(1000.0, 0.3)), 0)
            .await;
        let near = fixture
            .add("alice", "", "", "", Some(tone(1100.0, 0.3)), 1)
            .await;
        let far = fixture
            .add("alice", "", "", "", Some(tone(8000.0, 0.3)), 2)
            .await;
        fixture
            .add("bob", "", "", "", Some(tone(1000.0, 0.3)), 3)
            .await;
        let first = fixture.media.track_radio("alice", &seed, 50).await.unwrap();
        let second = fixture.media.track_radio("alice", &seed, 50).await.unwrap();
        assert_eq!(first.seed.id, seed);
        assert_eq!(first.candidate_count, 2);
        assert!(first.uses_audio_features);
        assert_eq!(
            first
                .items
                .iter()
                .map(|item| &item.track.id)
                .collect::<Vec<_>>(),
            vec![&near, &far]
        );
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(&second).unwrap()
        );
        assert!(
            first
                .items
                .iter()
                .all(|item| item.reason.contains("measured"))
        );
        assert!(
            !serde_json::to_string(&first)
                .unwrap()
                .contains("private-original")
        );
    }

    #[tokio::test]
    async fn metadata_fallback_uses_actual_whole_tags_and_sparse_catalogs() {
        let fixture = Fixture::new().await;
        let seed = fixture
            .add("alice", "Ensemble", "Seed", "Jazz; Live", None, 0)
            .await;
        let same = fixture
            .add("alice", "ensemble", "Other", "Jazz; Live", None, 1)
            .await;
        let other = fixture
            .add("alice", "Quartet", "Concert", "Jazz", None, 2)
            .await;
        let radio = fixture.media.track_radio("alice", &seed, 50).await.unwrap();
        assert!(!radio.uses_audio_features);
        assert_eq!(radio.items[0].track.id, same);
        assert!(radio.items[0].reason.contains("artist"));
        assert_eq!(radio.items[1].track.id, other);
        assert_eq!(
            radio.items[1].reason,
            "Another available track in your library"
        );
        let only = fixture.add("bob", "One", "Only", "", None, 0).await;
        let sparse = fixture.media.track_radio("bob", &only, 20).await.unwrap();
        assert_eq!(sparse.candidate_count, 0);
        assert!(sparse.items.is_empty());
    }

    #[tokio::test]
    async fn privacy_size_and_seed_validation_precede_recommendations() {
        let fixture = Fixture::new().await;
        let seed = fixture.add("alice", "A", "B", "C", None, 0).await;
        for limit in [0, 51, u32::MAX] {
            let error = fixture
                .media
                .track_radio("alice", &seed, limit)
                .await
                .unwrap_err();
            assert!(matches!(
                error.downcast_ref::<DiscoveryError>(),
                Some(DiscoveryError::Invalid)
            ));
        }
        let denied = fixture
            .media
            .track_radio("bob", &seed, 20)
            .await
            .unwrap_err();
        assert!(matches!(
            denied.downcast_ref::<DiscoveryError>(),
            Some(DiscoveryError::NotFound)
        ));
        let missing = fixture
            .media
            .track_radio("alice", &uuid::Uuid::new_v4().to_string(), 20)
            .await
            .unwrap_err();
        assert!(matches!(
            missing.downcast_ref::<DiscoveryError>(),
            Some(DiscoveryError::NotFound)
        ));
        let invalid = fixture
            .media
            .track_radio("alice", "not-a-uuid", 20)
            .await
            .unwrap_err();
        assert!(matches!(
            invalid.downcast_ref::<DiscoveryError>(),
            Some(DiscoveryError::Invalid)
        ));
    }

    #[tokio::test]
    async fn repeated_artists_and_albums_do_not_monopolize_an_available_variety() {
        let fixture = Fixture::new().await;
        let seed = fixture.add("alice", "A", "Seed", "Jazz", None, 0).await;
        for n in 0..8 {
            fixture
                .add("alice", "A", "Same album", "Jazz", None, n)
                .await;
        }
        fixture
            .add("alice", "B", "Second album", "Jazz", None, 10)
            .await;
        fixture
            .add("alice", "C", "Third album", "Jazz", None, 11)
            .await;
        let radio = fixture.media.track_radio("alice", &seed, 5).await.unwrap();
        let artists: std::collections::HashSet<_> = radio
            .items
            .iter()
            .map(|item| item.track.artist.as_str())
            .collect();
        assert_eq!(artists.len(), 3);
        assert_eq!(radio.items[0].track.artist, "A");
    }

    #[tokio::test]
    async fn malformed_or_old_cached_features_fall_back_without_breaking_playability() {
        let fixture = Fixture::new().await;
        let seed = fixture
            .add("alice", "A", "Seed", "Jazz", Some(tone(1000.0, 0.3)), 0)
            .await;
        let candidate = fixture
            .add("alice", "A", "Other", "Jazz", Some(tone(1100.0, 0.3)), 1)
            .await;
        for json in ["not json".to_owned(), "{}".into(), {
            let mut old = tone(1100.0, 0.3);
            old.extractor_version = "old-extractor".into();
            serde_json::to_string(&old).unwrap()
        }] {
            sqlx::query("UPDATE audio_analysis SET features_json=? WHERE media_asset_id=(SELECT media_asset_id FROM tracks WHERE id=?)")
                .bind(json).bind(&candidate).execute(&fixture.pool).await.unwrap();
            let radio = fixture.media.track_radio("alice", &seed, 5).await.unwrap();
            assert!(!radio.uses_audio_features);
            assert_eq!(radio.items[0].track.id, candidate);
            assert!(radio.items[0].reason.contains("artist"));
        }
    }

    #[tokio::test]
    async fn candidate_coverage_is_bounded_to_500_recent_owned_rows() {
        let fixture = Fixture::new().await;
        let seed = fixture.add("alice", "Seed artist", "", "", None, 0).await;
        let oldest = fixture.add("alice", "Seed artist", "", "", None, 0).await;
        for added_at in 1..=MAX_CANDIDATES {
            fixture.add("alice", "Other", "", "", None, added_at).await;
        }
        let radio = fixture.media.track_radio("alice", &seed, 50).await.unwrap();
        assert_eq!(radio.candidate_count, MAX_CANDIDATES as u32);
        assert_eq!(radio.items.len(), 50);
        assert!(radio.items.iter().all(|item| item.track.id != oldest));
    }
}
