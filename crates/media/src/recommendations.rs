//! Private, on-demand discovery over real owned media. Reads never collect
//! listening events, decode audio, start a worker or publish personal signals.
use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use sqlx::{QueryBuilder, Row, Sqlite, sqlite::SqliteRow};
use vela_domain::{
    AudioFeatures, RecommendationExclusions, RecommendationFeedback, RecommendationFeedbackReceipt,
    Recommendations, Track,
};

use crate::{MediaService, discovery, track_from_row};

const ALGORITHM_VERSION: &str = "vela-personal-1";
const MAX_CANDIDATES: i64 = 500;
const MAX_ANCHORS: usize = 12;
const MAX_EXCLUSIONS: i64 = 1000;

#[derive(Debug, thiserror::Error)]
pub enum RecommendationError {
    #[error("Recommendation size must be 1–30 and track identifiers must be valid UUIDs")]
    Invalid,
    #[error("Track not found")]
    NotFound,
    #[error("Hidden recommendation limit reached; restore a hidden track first")]
    Conflict,
}

struct Anchor {
    track: Track,
    features: Option<AudioFeatures>,
    from_history: bool,
}

impl MediaService {
    /// Examines at most500 newest eligible, nonhidden owned tracks. Anchors
    /// combine at most12 favorites/recent qualified listens. Recommendations
    /// are calculated afresh in one read snapshot: no stale taste cache can
    /// outlive a privacy change. The count is before anchor/played suppression.
    pub async fn recommendations(&self, owner: &str, limit: u32) -> Result<Recommendations> {
        if !(1..=30).contains(&limit) {
            return Err(RecommendationError::Invalid.into());
        }
        let mut tx = self.pool.begin().await?;
        // This first read establishes the snapshot and captures a common clock.
        let preferences = sqlx::query(
            "SELECT unixepoch() AS now,COALESCE((SELECT enabled FROM listening_preferences WHERE user_id=?),0) AS enabled,COALESCE((SELECT revision FROM listening_preferences WHERE user_id=?),0) AS revision,COALESCE((SELECT retention_days FROM listening_preferences WHERE user_id=?),90) AS retention_days",
        )
        .bind(owner)
        .bind(owner)
        .bind(owner)
        .fetch_one(&mut *tx)
        .await?;
        let now: i64 = preferences.try_get("now")?;
        let enabled: bool = preferences.try_get("enabled")?;
        let revision: i64 = preferences.try_get("revision")?;
        let retention: i64 = preferences.try_get("retention_days")?;
        let since = now.saturating_sub(retention.saturating_mul(86_400));
        let hidden_count = exclusion_count(&mut tx, owner).await?;

        let favorite_ids = favorite_selection(owner)
            .build()
            .fetch_all(&mut *tx)
            .await?;
        let favorite_rows = hydrate_selected(&mut tx, owner, &favorite_ids).await?;
        let mut favorite_anchors = Vec::with_capacity(favorite_rows.len());
        for row in favorite_rows {
            favorite_anchors.push(Anchor {
                track: track_from_row(&row)?,
                features: discovery::features(&row),
                from_history: false,
            });
        }

        let mut recent_anchors = Vec::new();
        // Deliberately no history SQL in the disabled branch, including for
        // suppression. Retained but disabled history has no derived influence.
        if enabled {
            let recent_ids = recent_selection(owner, since, now)
                .build()
                .fetch_all(&mut *tx)
                .await?;
            for row in hydrate_selected(&mut tx, owner, &recent_ids).await? {
                recent_anchors.push(Anchor {
                    track: track_from_row(&row)?,
                    features: discovery::features(&row),
                    from_history: true,
                });
            }
        }

        // Alternate explicit favorites and qualified history, skipping repeated
        // track IDs and filling free slots from the other source when sparse.
        let mut anchors = Vec::with_capacity(MAX_ANCHORS);
        let mut anchor_ids = HashSet::with_capacity(MAX_ANCHORS);
        let mut favorite_anchors = favorite_anchors.into_iter();
        let mut recent_anchors = recent_anchors.into_iter();
        while anchors.len() < MAX_ANCHORS {
            let mut exhausted = true;
            for next in [favorite_anchors.next(), recent_anchors.next()]
                .into_iter()
                .flatten()
            {
                exhausted = false;
                if anchors.len() < MAX_ANCHORS && anchor_ids.insert(next.track.id.clone()) {
                    anchors.push(next);
                }
            }
            if exhausted {
                break;
            }
        }

        let candidate_ids = candidate_selection(owner)
            .build()
            .fetch_all(&mut *tx)
            .await?;
        let rows = hydrate_selected(&mut tx, owner, &candidate_ids).await?;
        let candidate_count = rows.len() as u32;
        let mut played_ids = HashSet::<String>::new();
        if enabled && !rows.is_empty() {
            // Only IDs in the already bounded candidate window are returned;
            // never transfer an account's full retained history into memory.
            let mut played = QueryBuilder::<Sqlite>::new(
                "SELECT DISTINCT track_id FROM listening_sessions WHERE user_id=",
            );
            played
                .push_bind(owner)
                .push(" AND qualified=1 AND started_at>=")
                .push_bind(since)
                .push(" AND started_at<=")
                .push_bind(now)
                .push(" AND track_id IN (");
            {
                let mut separated = played.separated(",");
                for row in &rows {
                    separated.push_bind(row.try_get::<String, _>("id")?);
                }
            }
            played.push(")");
            played_ids = played
                .build_query_scalar::<String>()
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .collect();
        }
        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let track = track_from_row(&row)?;
            if anchor_ids.contains(&track.id) || played_ids.contains(&track.id) {
                continue;
            }
            let features = discovery::features(&row);
            let mut strongest = None;
            for anchor in &anchors {
                let mut candidate = discovery::rank(
                    &anchor.track,
                    anchor.features.as_ref(),
                    track.clone(),
                    features.as_ref(),
                );
                // The existing measured/tag score is bounded; repeated listens
                // do not multiply affinity or turn self-reports into popularity.
                candidate.reason.push_str(if anchor.from_history {
                    "; compared with a qualified listen in your private history"
                } else {
                    "; compared with one of your favorites"
                });
                if strongest
                    .as_ref()
                    .is_none_or(|best: &discovery::Candidate| candidate.score > best.score)
                {
                    strongest = Some(candidate);
                }
            }
            candidates.push(strongest.unwrap_or_else(|| {
                let mut candidate = discovery::rank(&track, None, track.clone(), None);
                candidate.score = 0.0;
                candidate.audio = false;
                candidate.reason = "Explore an available track in your library; no favorite or consented listening anchors yet".into();
                candidate
            }));
        }
        let (discovery, uses_audio_features) = discovery::diversify(candidates, limit as usize);

        let mut rotation = Vec::new();
        if enabled {
            let rotation_ids = rotation_selection(owner, since, now)
                .build()
                .fetch_all(&mut *tx)
                .await?;
            let rotation_rows = hydrate_selected(&mut tx, owner, &rotation_ids).await?;
            if !rotation_rows.is_empty() {
                let evidence: HashMap<_, _> = rotation_ids
                    .iter()
                    .map(|row| {
                        Ok((
                            row.try_get::<String, _>("id")?,
                            (
                                row.try_get::<i64, _>("play_count")?,
                                row.try_get::<i64, _>("last_listened")?,
                            ),
                        ))
                    })
                    .collect::<Result<_>>()?;
                let mut ranked = Vec::with_capacity(rotation_rows.len());
                for row in rotation_rows {
                    let track = track_from_row(&row)?;
                    let (count, last) = evidence[&track.id];
                    let mut candidate = discovery::rank(&track, None, track.clone(), None);
                    let recency = 604_800.0 / (604_800.0 + now.saturating_sub(last).max(0) as f64);
                    candidate.score = 0.65 * count as f64 / (count as f64 + 3.0) + 0.35 * recency;
                    candidate.reason = format!(
                        "{count} qualified {} in your retained private history",
                        if count == 1 { "listen" } else { "listens" }
                    );
                    candidate.audio = false;
                    ranked.push(candidate);
                }
                rotation = discovery::diversify(ranked, limit as usize).0;
            }
        }
        tx.commit().await?;
        Ok(Recommendations {
            algorithm_version: ALGORITHM_VERSION.into(),
            generated_at: now,
            privacy_revision: revision,
            history_enabled: enabled,
            candidate_count,
            anchor_count: anchors.len() as u32,
            hidden_count,
            uses_audio_features,
            discovery,
            rotation,
        })
    }

    /// Explicit desired state makes repeated feedback idempotent; it never
    /// changes favorites, media access, history, playlists or collection consent.
    pub async fn set_recommendation_feedback(
        &self,
        owner: &str,
        input: RecommendationFeedback,
    ) -> Result<RecommendationFeedbackReceipt> {
        if !uuid::Uuid::parse_str(&input.track_id).is_ok_and(|id| id.to_string() == input.track_id)
        {
            return Err(RecommendationError::Invalid.into());
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let owned: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tracks WHERE owner_id=? AND id=?)")
                .bind(owner)
                .bind(&input.track_id)
                .fetch_one(&mut *tx)
                .await?;
        if !owned {
            return Err(RecommendationError::NotFound.into());
        }
        if input.hidden {
            let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM recommendation_exclusions WHERE user_id=? AND track_id=?)")
                .bind(owner).bind(&input.track_id).fetch_one(&mut *tx).await?;
            if !existing && exclusion_count(&mut tx, owner).await? >= MAX_EXCLUSIONS as u32 {
                return Err(RecommendationError::Conflict.into());
            }
            sqlx::query("INSERT INTO recommendation_exclusions(user_id,track_id,created_at) VALUES(?,?,unixepoch()) ON CONFLICT(user_id,track_id) DO NOTHING")
                .bind(owner).bind(&input.track_id).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM recommendation_exclusions WHERE user_id=? AND track_id=?")
                .bind(owner)
                .bind(&input.track_id)
                .execute(&mut *tx)
                .await?;
        }
        let hidden_count = exclusion_count(&mut tx, owner).await?;
        tx.commit().await?;
        Ok(RecommendationFeedbackReceipt { hidden_count })
    }

    pub async fn clear_recommendation_feedback(
        &self,
        owner: &str,
    ) -> Result<RecommendationFeedbackReceipt> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM recommendation_exclusions WHERE user_id=?")
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(RecommendationFeedbackReceipt { hidden_count: 0 })
    }

    pub async fn recommendation_exclusions(&self, owner: &str) -> Result<RecommendationExclusions> {
        let track_ids = sqlx::query_scalar("SELECT track_id FROM recommendation_exclusions WHERE user_id=? ORDER BY track_id ASC LIMIT 1000")
            .bind(owner).fetch_all(&self.pool).await?;
        Ok(RecommendationExclusions {
            format_version: 1,
            track_ids,
        })
    }
}

fn favorite_selection(owner: &str) -> QueryBuilder<'_, Sqlite> {
    // CROSS JOIN keeps the indexed favorite set ahead of catalog hydration.
    // Starting with tracks instead makes an empty/sparse favorite set walk the
    // whole owned catalog merely to find its newest twelve favorites.
    let mut query = QueryBuilder::new(
        "SELECT t.id FROM favorites selected CROSS JOIN tracks t ON t.id=selected.track_id JOIN media_assets a ON a.id=t.media_asset_id",
    );
    eligible(&mut query, owner);
    query
        .push(" AND selected.user_id=")
        .push_bind(owner)
        .push(" ORDER BY t.added_at DESC,t.id ASC LIMIT ")
        .push_bind(MAX_ANCHORS as i64);
    query
}

fn history_selection(owner: &str, since: i64, now: i64) -> QueryBuilder<'_, Sqlite> {
    // Aggregate only this account's qualified retained sessions, then seek the
    // referenced tracks. CROSS JOIN prevents SQLite from scanning every owned
    // track and probing a small materialized history aggregate for each one.
    let mut query = QueryBuilder::new(
        "SELECT t.id,listened.play_count,listened.last_listened FROM (SELECT track_id,COUNT(*) AS play_count,MAX(updated_at) AS last_listened FROM listening_sessions WHERE user_id=",
    );
    query
        .push_bind(owner)
        .push(" AND qualified=1 AND started_at>=")
        .push_bind(since)
        .push(" AND started_at<=")
        .push_bind(now)
        .push(" GROUP BY track_id) listened CROSS JOIN tracks t ON t.id=listened.track_id JOIN media_assets a ON a.id=t.media_asset_id");
    eligible(&mut query, owner);
    query
}

fn recent_selection(owner: &str, since: i64, now: i64) -> QueryBuilder<'_, Sqlite> {
    let mut query = history_selection(owner, since, now);
    query
        .push(" ORDER BY listened.last_listened DESC,t.id ASC LIMIT ")
        .push_bind(MAX_ANCHORS as i64);
    query
}

fn rotation_selection(owner: &str, since: i64, now: i64) -> QueryBuilder<'_, Sqlite> {
    let mut query = history_selection(owner, since, now);
    query
        .push(" ORDER BY (0.65*listened.play_count/(listened.play_count+3.0)+0.35*604800.0/(604800.0+MAX(0,")
        .push_bind(now)
        .push("-listened.last_listened))) DESC,t.id ASC LIMIT ")
        .push_bind(MAX_CANDIDATES);
    query
}

fn candidate_selection(owner: &str) -> QueryBuilder<'_, Sqlite> {
    // Resolve eligibility and the hard window before projecting features and
    // full metadata, including when many tracks share the same added_at.
    let mut query =
        QueryBuilder::new("SELECT t.id FROM tracks t JOIN media_assets a ON a.id=t.media_asset_id");
    eligible(&mut query, owner);
    query
        .push(" ORDER BY t.added_at DESC,t.id ASC LIMIT ")
        .push_bind(MAX_CANDIDATES);
    query
}

fn hydration_query<'a>(owner: &'a str, ids: &'a [String]) -> QueryBuilder<'a, Sqlite> {
    let mut query = discovery::query(owner);
    query
        .push(" WHERE t.owner_id=")
        .push_bind(owner)
        .push(" AND t.id IN (");
    {
        let mut separated = query.separated(",");
        for id in ids {
            separated.push_bind(id);
        }
    }
    query.push(")");
    query
}

async fn hydrate_selected(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    owner: &str,
    selected: &[SqliteRow],
) -> Result<Vec<SqliteRow>> {
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    let ids = selected
        .iter()
        .map(|row| row.try_get::<String, _>("id"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows: HashMap<_, _> = hydration_query(owner, &ids)
        .build()
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|row| Ok((row.try_get::<String, _>("id")?, row)))
        .collect::<Result<_>>()?;
    // IN has no order guarantee. Preserve the exact source ordering so that
    // favorite/history alternation and equal-scoring anchor reasons stay stable.
    ids.into_iter()
        .map(|id| {
            rows.remove(&id)
                .context("Selected recommendation track missing from its read snapshot")
        })
        .collect()
}

fn eligible<'a>(query: &mut QueryBuilder<'a, Sqlite>, owner: &'a str) {
    query
        .push(" WHERE t.owner_id=")
        .push_bind(owner)
        .push(" AND a.size_bytes>0 AND t.duration_ms>0 AND NOT EXISTS(SELECT 1 FROM recommendation_exclusions hidden WHERE hidden.user_id=")
        .push_bind(owner)
        .push(" AND hidden.track_id=t.id)");
}

async fn exclusion_count(tx: &mut sqlx::Transaction<'_, Sqlite>, owner: &str) -> Result<u32> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recommendation_exclusions WHERE user_id=?")
            .bind(owner)
            .fetch_one(&mut **tx)
            .await?;
    Ok(u32::try_from(count)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::EXTRACTOR_VERSION;
    use sqlx::Execute;
    use sqlx::sqlite::{
        SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
    };

    struct Fixture {
        _directory: tempfile::TempDir,
        pool: sqlx::SqlitePool,
        media: MediaService,
        now: i64,
    }

    impl Fixture {
        async fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let pool = SqlitePoolOptions::new()
                .max_connections(2)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(directory.path().join("personal.db"))
                        .create_if_missing(true)
                        .foreign_keys(true)
                        .journal_mode(SqliteJournalMode::Wal)
                        .synchronous(SqliteSynchronous::Normal),
                )
                .await
                .unwrap();
            sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
            for owner in ["alice", "bob"] {
                sqlx::query(
                    "INSERT INTO users(id,username,password_hash) VALUES(?,?,'isolated-fixture')",
                )
                .bind(owner)
                .bind(owner)
                .execute(&pool)
                .await
                .unwrap();
                sqlx::query("INSERT INTO listening_devices(user_id,device_id,enabled) VALUES(?,'00000000-0000-4000-8000-000000000001',1)")
                    .bind(owner).execute(&pool).await.unwrap();
            }
            let now = sqlx::query_scalar("SELECT unixepoch()")
                .fetch_one(&pool)
                .await
                .unwrap();
            Self {
                media: MediaService::new(pool.clone(), directory.path().into(), 1024),
                pool,
                _directory: directory,
                now,
            }
        }

        async fn add(&self, owner: &str, artist: &str, album: &str, added: i64) -> String {
            let id = uuid::Uuid::new_v4().to_string();
            let asset = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',1024)")
                .bind(&asset).bind(&asset).bind("private-recommendation-original").execute(&self.pool).await.unwrap();
            sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,genre,duration_ms,codec,added_at) VALUES(?,?,?,'Synthetic metadata fixture',?,?,'Jazz',120000,'FLAC',?)")
                .bind(&id).bind(owner).bind(asset).bind(artist).bind(album).bind(added).execute(&self.pool).await.unwrap();
            id
        }

        async fn favorite(&self, owner: &str, id: &str) {
            sqlx::query("INSERT INTO favorites(user_id,track_id) VALUES(?,?)")
                .bind(owner)
                .bind(id)
                .execute(&self.pool)
                .await
                .unwrap();
        }

        async fn consent(&self, owner: &str, enabled: bool, retention: i64, revision: i64) {
            sqlx::query("INSERT INTO listening_preferences(user_id,enabled,retention_days,revision,updated_at) VALUES(?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET enabled=excluded.enabled,retention_days=excluded.retention_days,revision=excluded.revision")
                .bind(owner).bind(enabled).bind(retention).bind(revision).bind(self.now).execute(&self.pool).await.unwrap();
        }

        async fn listen(&self, owner: &str, track: &str, started: i64, qualified: bool) -> String {
            let session = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES(?,?,'00000000-0000-4000-8000-000000000001',?,1,30000,30000,?,?,?,1)")
                .bind(owner).bind(&session).bind(track).bind(started).bind(started + 30).bind(qualified).execute(&self.pool).await.unwrap();
            session
        }

        async fn features(&self, track: &str, features: &AudioFeatures) {
            sqlx::query("INSERT INTO audio_analysis(media_asset_id,extractor_version,status,features_json,created_at,updated_at) SELECT media_asset_id,?,'complete',?,0,0 FROM tracks WHERE id=? ON CONFLICT(media_asset_id,extractor_version) DO UPDATE SET features_json=excluded.features_json")
                .bind(EXTRACTOR_VERSION).bind(serde_json::to_string(features).unwrap()).bind(track).execute(&self.pool).await.unwrap();
        }

        async fn hide(&self, owner: &str, id: &str, hidden: bool) -> RecommendationFeedbackReceipt {
            self.media
                .set_recommendation_feedback(
                    owner,
                    RecommendationFeedback {
                        track_id: id.into(),
                        hidden,
                    },
                )
                .await
                .unwrap()
        }
    }

    fn tone(hz: f64) -> AudioFeatures {
        let rms: f64 = 0.3;
        let peak = rms * 2.0_f64.sqrt();
        let mut bands = [0.0; 8];
        let band = crate::analysis::BAND_UPPER_HZ
            .iter()
            .position(|upper| hz < *upper)
            .unwrap_or(7);
        bands[band] = 1.0;
        AudioFeatures {
            extractor_version: EXTRACTOR_VERSION.into(),
            sample_rate: 48_000,
            channels: 1,
            analyzed_frames: 48_000,
            analyzed_ms: 1000,
            complete: true,
            rms,
            peak,
            rms_dbfs: Some(20.0 * rms.log10()),
            peak_dbfs: Some(20.0 * peak.log10()),
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
        }
    }

    async fn query_plan(
        pool: &sqlx::SqlitePool,
        mut query: QueryBuilder<'_, Sqlite>,
    ) -> Vec<String> {
        let sql = format!("EXPLAIN QUERY PLAN {}", query.sql());
        let arguments = query.build().take_arguments().unwrap().unwrap();
        sqlx::query_with(&sql, arguments)
            .fetch_all(pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get("detail"))
            .collect()
    }

    #[tokio::test]
    async fn sparse_evidence_seeks_tracks_instead_of_walking_the_owned_catalog() {
        let f = Fixture::new().await;
        let track = f.add("alice", "Sparse evidence", "Album", 1).await;
        f.favorite("alice", &track).await;
        f.listen("alice", &track, f.now - 60, true).await;
        let since = f.now - 90 * 86_400;
        for (name, query) in [
            ("favorites", favorite_selection("alice")),
            ("recent", recent_selection("alice", since, f.now)),
            ("rotation", rotation_selection("alice", since, f.now)),
        ] {
            let plan = query_plan(&f.pool, query).await;
            // Regression for sparse/empty evidence over large libraries: an
            // owner-only catalog index search still visits every owned track.
            // Each evidence ID must instead drive a primary/composite-key seek.
            let track_steps: Vec<_> = plan
                .iter()
                .filter(|detail| detail.starts_with("SEARCH t ") || detail.starts_with("SCAN t"))
                .collect();
            assert_eq!(track_steps.len(), 1, "{name}: {plan:?}");
            assert!(track_steps[0].contains("id=?"), "{name}: {plan:?}");
        }
        let plan = query_plan(&f.pool, candidate_selection("alice")).await;
        assert!(
            plan.iter()
                .any(|detail| detail.contains("tracks_owner_newest_order")),
            "{plan:?}"
        );
        let ids: Vec<_> = (0..MAX_CANDIDATES)
            .map(|n| uuid::Uuid::from_u128(1 + n as u128).to_string())
            .collect();
        let plan = query_plan(&f.pool, hydration_query("alice", &ids)).await;
        assert!(
            plan.iter()
                .any(|detail| detail.starts_with("SEARCH t ") && detail.contains("id=?")),
            "{plan:?}"
        );
    }

    #[tokio::test]
    async fn bounded_hydration_preserves_source_order_and_eligibility() {
        let f = Fixture::new().await;
        let older = f.add("alice", "Older", "Album", 1).await;
        let newer = f.add("alice", "Newer", "Album", 2).await;
        let hidden = f.add("alice", "Hidden", "Album", 3).await;
        let ineligible = f.add("alice", "Zero duration", "Album", 4).await;
        let foreign = f.add("bob", "Foreign", "Private", 5).await;
        for id in [&older, &newer, &hidden, &ineligible, &foreign] {
            f.favorite("alice", id).await;
        }
        sqlx::query("UPDATE tracks SET duration_ms=0 WHERE id=?")
            .bind(&ineligible)
            .execute(&f.pool)
            .await
            .unwrap();
        f.hide("alice", &hidden, true).await;
        f.listen("alice", &older, f.now - 60, true).await;
        f.listen("alice", &newer, f.now - 120, true).await;
        f.listen("alice", &hidden, f.now - 30, true).await;
        f.listen("alice", &ineligible, f.now - 30, true).await;
        let mut tx = f.pool.begin().await.unwrap();
        for (mut selection, expected) in [
            (
                favorite_selection("alice"),
                vec![newer.clone(), older.clone()],
            ),
            (
                recent_selection("alice", f.now - 90 * 86_400, f.now),
                vec![older.clone(), newer.clone()],
            ),
        ] {
            let selected = selection.build().fetch_all(&mut *tx).await.unwrap();
            let rows = hydrate_selected(&mut tx, "alice", &selected).await.unwrap();
            assert_eq!(
                rows.iter()
                    .map(|row| row.get::<String, _>("id"))
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(rows.iter().all(|row| row.get::<bool, _>("favorite")));
        }
    }

    #[tokio::test]
    async fn disabled_history_has_no_effect_and_performs_no_history_queries() {
        let f = Fixture::new().await;
        let track = f.add("alice", "A", "Album", 1).await;
        f.listen("alice", &track, f.now - 60, true).await;
        f.consent("alice", false, 90, 17).await;
        // Isolated fixture proof: even a missing history table cannot affect
        // the disabled service branch. No real data directory is involved.
        sqlx::query("DROP TABLE listening_sessions")
            .execute(&f.pool)
            .await
            .unwrap();
        let result = f.media.recommendations("alice", 12).await.unwrap();
        assert!(!result.history_enabled);
        assert_eq!(result.privacy_revision, 17);
        assert_eq!(result.anchor_count, 0);
        assert!(result.rotation.is_empty());
        assert_eq!(result.discovery[0].track.id, track);
        assert!(result.discovery[0].reason.contains("no favorite"));
    }

    #[tokio::test]
    async fn favorites_and_measured_descriptors_are_real_owner_scoped_signals() {
        let f = Fixture::new().await;
        let anchor = f.add("alice", "Favorite artist", "Original", 0).await;
        let near = f.add("alice", "Near artist", "Near", 1).await;
        let far = f.add("alice", "Far artist", "Far", 2).await;
        let foreign = f.add("bob", "Secret artist", "Private", 3).await;
        f.favorite("alice", &anchor).await;
        // A malicious cross-owner favorite cannot make an anchor or candidate.
        f.favorite("alice", &foreign).await;
        f.features(&anchor, &tone(1000.0)).await;
        f.features(&near, &tone(1100.0)).await;
        f.features(&far, &tone(8000.0)).await;
        let first = f.media.recommendations("alice", 12).await.unwrap();
        let second = f.media.recommendations("alice", 12).await.unwrap();
        assert_eq!(first.anchor_count, 1);
        assert_eq!(first.candidate_count, 3);
        assert!(first.uses_audio_features);
        assert_eq!(first.discovery[0].track.id, near);
        assert_eq!(first.discovery[1].track.id, far);
        assert!(
            first
                .discovery
                .iter()
                .all(|item| item.reason.contains("favorite"))
        );
        assert!(
            first
                .discovery
                .iter()
                .all(|item| item.reason.contains("measured"))
        );
        assert_eq!(
            first
                .discovery
                .iter()
                .map(|item| &item.track.id)
                .collect::<Vec<_>>(),
            second
                .discovery
                .iter()
                .map(|item| &item.track.id)
                .collect::<Vec<_>>()
        );
        let json = serde_json::to_string(&first).unwrap();
        assert!(!json.contains("Secret artist"));
        assert!(!json.contains("private-recommendation-original"));
        assert!(first.rotation.is_empty());
    }

    #[tokio::test]
    async fn retained_qualified_sessions_only_and_cumulative_updates_do_not_amplify() {
        let f = Fixture::new().await;
        let played = f.add("alice", "Actual artist", "Played", 1).await;
        let expired = f.add("alice", "Expired", "Old", 2).await;
        let partial = f.add("alice", "Rapid skip", "Partial", 3).await;
        let future = f.add("alice", "Future", "Future", 4).await;
        let novel = f.add("alice", "Actual artist", "Unheard", 5).await;
        f.consent("alice", true, 7, 4).await;
        let session = f.listen("alice", &played, f.now - 100, true).await;
        f.listen("alice", &played, f.now - 60, true).await;
        f.listen("alice", &expired, f.now - 8 * 86_400, true).await;
        f.listen("alice", &partial, f.now - 60, false).await;
        f.listen("alice", &future, f.now + 3600, true).await;
        sqlx::query(
            "UPDATE listening_sessions SET sequence=100,listened_ms=60000 WHERE session_id=?",
        )
        .bind(session)
        .execute(&f.pool)
        .await
        .unwrap();
        let result = f.media.recommendations("alice", 30).await.unwrap();
        assert!(result.history_enabled);
        assert_eq!(result.privacy_revision, 4);
        assert_eq!(result.anchor_count, 1);
        assert_eq!(result.rotation.len(), 1);
        assert_eq!(result.rotation[0].track.id, played);
        assert!(result.rotation[0].reason.starts_with("2 qualified listens"));
        assert_eq!(result.discovery[0].track.id, novel);
        assert!(result.discovery[0].reason.contains("qualified listen"));
        for id in [&expired, &partial, &future] {
            assert!(result.discovery.iter().any(|item| &item.track.id == id));
        }
        f.consent("alice", false, 7, 5).await;
        let disabled = f.media.recommendations("alice", 30).await.unwrap();
        assert_eq!(disabled.anchor_count, 0);
        assert_eq!(disabled.privacy_revision, 5);
        assert!(disabled.rotation.is_empty());
        assert!(
            disabled
                .discovery
                .iter()
                .any(|item| item.track.id == played)
        );
        assert!(
            disabled
                .discovery
                .iter()
                .all(|item| !item.reason.contains("compared with a qualified"))
        );
    }

    #[tokio::test]
    async fn sparse_already_played_library_returns_honest_empty_discovery() {
        let f = Fixture::new().await;
        assert!(
            f.media
                .recommendations("alice", 12)
                .await
                .unwrap()
                .discovery
                .is_empty()
        );
        let only = f.add("alice", "One", "Only", 1).await;
        let cold = f.media.recommendations("alice", 12).await.unwrap();
        assert_eq!(cold.discovery.len(), 1);
        f.consent("alice", true, 90, 1).await;
        f.listen("alice", &only, f.now - 60, true).await;
        let listened = f.media.recommendations("alice", 12).await.unwrap();
        assert!(listened.discovery.is_empty());
        assert_eq!(listened.candidate_count, 1);
        assert_eq!(listened.rotation[0].track.id, only);
        f.hide("alice", &only, true).await;
        let hidden = f.media.recommendations("alice", 12).await.unwrap();
        assert_eq!(hidden.candidate_count, 0);
        assert_eq!(hidden.anchor_count, 0);
        assert_eq!(hidden.hidden_count, 1);
        assert!(hidden.rotation.is_empty());
    }

    #[tokio::test]
    async fn cold_start_is_deterministic_and_diversifies_actual_artists_and_albums() {
        let f = Fixture::new().await;
        for added in 0..8 {
            f.add("alice", "A", "Same album", added).await;
        }
        f.add("alice", "B", "Other album", 10).await;
        f.add("alice", "C", "Third album", 11).await;
        let first = f.media.recommendations("alice", 5).await.unwrap();
        let second = f.media.recommendations("alice", 5).await.unwrap();
        let artists: HashSet<_> = first
            .discovery
            .iter()
            .map(|item| &item.track.artist)
            .collect();
        assert_eq!(artists.len(), 3);
        assert!(!first.uses_audio_features);
        assert_eq!(first.anchor_count, 0);
        assert_eq!(
            first
                .discovery
                .iter()
                .map(|item| &item.track.id)
                .collect::<Vec<_>>(),
            second
                .discovery
                .iter()
                .map(|item| &item.track.id)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn shared_physical_assets_do_not_share_history_hidden_state_or_ownership() {
        let f = Fixture::new().await;
        let alice = f.add("alice", "Alice original", "A", 1).await;
        let bob = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,duration_ms,codec,added_at) SELECT ?,'bob',media_asset_id,'Private Bob metadata','Bob private artist',duration_ms,codec,2 FROM tracks WHERE id=?")
            .bind(&bob).bind(&alice).execute(&f.pool).await.unwrap();
        f.consent("bob", true, 90, 1).await;
        f.listen("bob", &bob, f.now - 60, true).await;
        f.hide("bob", &bob, true).await;
        let result = f.media.recommendations("alice", 12).await.unwrap();
        assert_eq!(result.hidden_count, 0);
        assert_eq!(result.anchor_count, 0);
        assert_eq!(result.discovery[0].track.id, alice);
        assert!(result.rotation.is_empty());
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("Bob private")
        );
        assert!(
            f.media
                .recommendation_exclusions("alice")
                .await
                .unwrap()
                .track_ids
                .is_empty()
        );
        let denied = f
            .media
            .set_recommendation_feedback(
                "alice",
                RecommendationFeedback {
                    track_id: bob,
                    hidden: false,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            denied.downcast_ref::<RecommendationError>(),
            Some(RecommendationError::NotFound)
        ));
    }

    #[tokio::test]
    async fn feedback_is_idempotent_revocable_owner_scoped_and_does_not_change_music() {
        let f = Fixture::new().await;
        let alice = f.add("alice", "Artist", "Album", 1).await;
        let bob = f.add("bob", "Other", "Private", 2).await;
        f.favorite("alice", &alice).await;
        f.hide("bob", &bob, true).await;
        assert_eq!(f.hide("alice", &alice, true).await.hidden_count, 1);
        assert_eq!(f.hide("alice", &alice, true).await.hidden_count, 1);
        let export = f.media.recommendation_exclusions("alice").await.unwrap();
        assert_eq!(export.format_version, 1);
        assert_eq!(export.track_ids, vec![alice.clone()]);
        let favorite: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM favorites WHERE user_id='alice'")
                .fetch_one(&f.pool)
                .await
                .unwrap();
        assert_eq!(favorite, 1);
        assert_eq!(f.hide("alice", &alice, false).await.hidden_count, 0);
        assert_eq!(f.hide("alice", &alice, false).await.hidden_count, 0);
        f.hide("alice", &alice, true).await;
        assert_eq!(
            f.media
                .clear_recommendation_feedback("alice")
                .await
                .unwrap()
                .hidden_count,
            0
        );
        assert_eq!(
            f.media
                .clear_recommendation_feedback("alice")
                .await
                .unwrap()
                .hidden_count,
            0
        );
        assert_eq!(
            f.media
                .recommendation_exclusions("bob")
                .await
                .unwrap()
                .track_ids,
            vec![bob]
        );
        let preferences: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM listening_preferences WHERE user_id='alice'")
                .fetch_one(&f.pool)
                .await
                .unwrap();
        assert_eq!(preferences, 0);
    }

    #[tokio::test]
    async fn hidden_tracks_are_removed_before_candidate_cap_and_old_rotation_remains() {
        let f = Fixture::new().await;
        let old_played = f.add("alice", "Old", "Outside newest window", -2).await;
        let old_novel = f.add("alice", "Old unexplored", "Old album", -1).await;
        let mut newest = Vec::new();
        for added in 0..501 {
            newest.push(f.add("alice", "Recent", "Recent album", added).await);
        }
        f.consent("alice", true, 90, 1).await;
        f.listen("alice", &old_played, f.now - 60, true).await;
        let bounded = f.media.recommendations("alice", 30).await.unwrap();
        assert_eq!(bounded.candidate_count, 500);
        assert_eq!(bounded.rotation[0].track.id, old_played);
        assert!(
            bounded
                .discovery
                .iter()
                .all(|item| item.track.id != old_novel)
        );
        for id in newest.iter().rev().take(6) {
            f.hide("alice", id, true).await;
        }
        //503 total minus6 hidden =497, including the old played candidate;
        // filtering after LIMIT would incorrectly examine only494 here.
        let hidden = f.media.recommendations("alice", 30).await.unwrap();
        assert_eq!(hidden.candidate_count, 497);
        assert_eq!(hidden.hidden_count, 6);
        assert!(
            hidden.discovery.iter().all(|item| !newest
                .iter()
                .rev()
                .take(6)
                .any(|id| item.track.id == *id))
        );
        assert!(
            hidden
                .rotation
                .iter()
                .any(|item| item.track.id == old_played)
        );
    }

    #[tokio::test]
    async fn favorite_and_history_anchor_union_never_exceeds_twelve() {
        let f = Fixture::new().await;
        f.consent("alice", true, 90, 1).await;
        for n in 0..20 {
            let id = f.add("alice", &format!("Artist{n}"), "Album", n).await;
            if n < 15 {
                f.favorite("alice", &id).await;
            }
            if n >= 5 {
                f.listen("alice", &id, f.now - 60 - n, true).await;
            }
        }
        let result = f.media.recommendations("alice", 30).await.unwrap();
        assert_eq!(result.anchor_count, 12);
        assert!(result.rotation.len() <= 15);
        assert!(
            result
                .discovery
                .iter()
                .all(|item| item.track.artist[6..].parse::<i64>().unwrap() < 5)
        );
        f.consent("alice", false, 90, 2).await;
        let disabled = f.media.recommendations("alice", 30).await.unwrap();
        assert_eq!(disabled.anchor_count, 12);
        assert!(disabled.rotation.is_empty());
        assert!(
            disabled
                .discovery
                .iter()
                .all(|item| !item.reason.contains("private history"))
        );
    }

    #[tokio::test]
    async fn corrupt_silent_and_obsolete_descriptors_use_honest_tag_fallback() {
        let f = Fixture::new().await;
        let anchor = f.add("alice", "Artist", "Favorite", 1).await;
        let candidate = f.add("alice", "Artist", "Candidate", 2).await;
        f.favorite("alice", &anchor).await;
        f.features(&anchor, &tone(1000.0)).await;
        f.features(&candidate, &tone(1100.0)).await;
        assert!(
            f.media
                .recommendations("alice", 1)
                .await
                .unwrap()
                .uses_audio_features
        );
        let mut old = tone(1100.0);
        old.extractor_version = "obsolete-extractor".into();
        let mut silence = tone(1100.0);
        silence.rms = 0.0;
        silence.peak = 0.0;
        silence.rms_dbfs = None;
        silence.peak_dbfs = None;
        for json in [
            "not json".into(),
            "{}".into(),
            serde_json::to_string(&old).unwrap(),
            serde_json::to_string(&silence).unwrap(),
        ] {
            sqlx::query("UPDATE audio_analysis SET features_json=? WHERE media_asset_id=(SELECT media_asset_id FROM tracks WHERE id=?)")
                .bind(json).bind(&candidate).execute(&f.pool).await.unwrap();
            let result = f.media.recommendations("alice", 1).await.unwrap();
            assert!(!result.uses_audio_features);
            assert_eq!(result.discovery[0].track.id, candidate);
            assert!(result.discovery[0].reason.contains("Same artist"));
            assert!(!result.discovery[0].reason.contains("measured"));
        }
    }

    #[tokio::test]
    async fn feedback_quota_allows_existing_hide_and_undo_without_losing_rows() {
        let f = Fixture::new().await;
        let mut first = String::new();
        for n in 0..MAX_EXCLUSIONS {
            let id = f.add("alice", "Artist", "Album", n).await;
            if n == 0 {
                first = id.clone();
            }
            sqlx::query("INSERT INTO recommendation_exclusions(user_id,track_id,created_at) VALUES('alice',?,0)")
                .bind(id).execute(&f.pool).await.unwrap();
        }
        let extra = f.add("alice", "Extra", "Album", 2000).await;
        let error = f
            .media
            .set_recommendation_feedback(
                "alice",
                RecommendationFeedback {
                    track_id: extra.clone(),
                    hidden: true,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<RecommendationError>(),
            Some(RecommendationError::Conflict)
        ));
        assert_eq!(
            f.hide("alice", &first, true).await.hidden_count,
            MAX_EXCLUSIONS as u32
        );
        assert_eq!(
            f.media
                .recommendation_exclusions("alice")
                .await
                .unwrap()
                .track_ids
                .len(),
            MAX_EXCLUSIONS as usize
        );
        assert_eq!(
            f.hide("alice", &first, false).await.hidden_count,
            MAX_EXCLUSIONS as u32 - 1
        );
        let (left, right) = tokio::join!(
            f.media.set_recommendation_feedback(
                "alice",
                RecommendationFeedback {
                    track_id: first,
                    hidden: true
                }
            ),
            f.media.set_recommendation_feedback(
                "alice",
                RecommendationFeedback {
                    track_id: extra,
                    hidden: true
                }
            )
        );
        let outcomes = [left, right];
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        let error = outcomes
            .iter()
            .find_map(|result| result.as_ref().err())
            .unwrap();
        assert!(matches!(
            error.downcast_ref::<RecommendationError>(),
            Some(RecommendationError::Conflict)
        ));
        assert_eq!(
            f.media
                .recommendation_exclusions("alice")
                .await
                .unwrap()
                .track_ids
                .len(),
            MAX_EXCLUSIONS as usize
        );
        assert_eq!(
            f.media
                .clear_recommendation_feedback("alice")
                .await
                .unwrap()
                .hidden_count,
            0
        );
    }

    #[tokio::test]
    async fn concurrent_privacy_changes_never_mix_history_and_consent_snapshots() {
        let f = Fixture::new().await;
        let track = f.add("alice", "Artist", "Album", 1).await;
        f.listen("alice", &track, f.now - 60, true).await;
        f.consent("alice", false, 90, 0).await;
        let writer = async {
            for revision in 1..=40 {
                f.consent("alice", revision % 2 == 1, 90, revision).await;
                tokio::task::yield_now().await;
            }
        };
        let reader = async {
            for _ in 0..40 {
                let result = f.media.recommendations("alice", 12).await.unwrap();
                assert_eq!(result.history_enabled, result.privacy_revision % 2 == 1);
                if result.history_enabled {
                    assert_eq!(result.anchor_count, 1);
                    assert_eq!(result.rotation[0].track.id, track);
                    assert!(result.discovery.is_empty());
                } else {
                    assert_eq!(result.anchor_count, 0);
                    assert!(result.rotation.is_empty());
                    assert_eq!(result.discovery[0].track.id, track);
                }
            }
        };
        tokio::join!(writer, reader);
    }

    #[tokio::test]
    async fn bounds_validation_and_foreign_targets_never_mutate_preferences() {
        let f = Fixture::new().await;
        for limit in [0, 31, u32::MAX] {
            let error = f.media.recommendations("alice", limit).await.unwrap_err();
            assert!(matches!(
                error.downcast_ref::<RecommendationError>(),
                Some(RecommendationError::Invalid)
            ));
        }
        for id in [
            "bad-id".to_owned(),
            uuid::Uuid::new_v4().to_string().to_uppercase(),
        ] {
            let error = f
                .media
                .set_recommendation_feedback(
                    "alice",
                    RecommendationFeedback {
                        track_id: id,
                        hidden: true,
                    },
                )
                .await
                .unwrap_err();
            assert!(matches!(
                error.downcast_ref::<RecommendationError>(),
                Some(RecommendationError::Invalid)
            ));
        }
        let error = f
            .media
            .set_recommendation_feedback(
                "alice",
                RecommendationFeedback {
                    track_id: uuid::Uuid::new_v4().to_string(),
                    hidden: true,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<RecommendationError>(),
            Some(RecommendationError::NotFound)
        ));
        assert_eq!(
            f.media
                .recommendation_exclusions("alice")
                .await
                .unwrap()
                .track_ids
                .len(),
            0
        );
    }
}
