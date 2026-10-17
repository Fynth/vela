//! Private, durable orchestration of versioned content analysis.
//! The cache may reuse identical bytes, but every API read and enqueue checks
//! current track ownership independently of playlist or metadata permissions.

use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use sqlx::{Row, SqlitePool};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vela_domain::{
    AnalysisLibraryStatus, AnalysisReceipt, AnalysisStatus, EnqueueAnalysis, TrackAnalysis,
};

use crate::{
    MediaService,
    analysis::{AnalysisLimits, AudioFeatures, EXTRACTOR_VERSION, analyze_file},
};

const MAX_BATCH: usize = 50;
const MAX_GLOBAL_ACTIVE: i64 = 128;
const MAX_OWNER_ACTIVE: i64 = 50;
const MAX_FEATURE_BYTES: usize = 16 * 1024;
const FAILED_MESSAGE: &str =
    "Audio could not be analyzed within supported formats and safety limits";

/// Fixed validation/policy errors, without input paths, tags or parser details.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Track was not found")]
    NotFound,
}

#[derive(Clone)]
pub struct AnalysisService {
    pool: SqlitePool,
    media: MediaService,
    wake: Arc<Notify>,
    worker_lock: Arc<Mutex<()>>,
}

struct ClaimedJob {
    asset_id: String,
    owner: Option<String>,
    track_id: Option<String>,
}

impl AnalysisService {
    pub fn new(pool: SqlitePool, media: MediaService) -> Self {
        Self {
            pool,
            media,
            wake: Arc::new(Notify::new()),
            worker_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Actual owned tracks in one read snapshot; shared asset rows never add
    /// another owner's tracks or work to these counts.
    pub async fn summary(&self, owner: &str) -> Result<AnalysisLibraryStatus> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS total,COALESCE(SUM(a.status IS NULL),0) AS not_analyzed,COALESCE(SUM(a.status='queued'),0) AS queued,COALESCE(SUM(a.status='running'),0) AS running,COALESCE(SUM(a.status='complete'),0) AS complete,COALESCE(SUM(a.status='failed'),0) AS failed FROM tracks t LEFT JOIN audio_analysis a ON a.media_asset_id=t.media_asset_id AND a.extractor_version=? WHERE t.owner_id=?",
        )
        .bind(EXTRACTOR_VERSION)
        .bind(owner)
        .fetch_one(&self.pool)
        .await?;
        Ok(AnalysisLibraryStatus {
            total: row.try_get("total")?,
            not_analyzed: row.try_get("not_analyzed")?,
            queued: row.try_get("queued")?,
            running: row.try_get("running")?,
            complete: row.try_get("complete")?,
            failed: row.try_get("failed")?,
        })
    }

    /// Enqueue a bounded atomic batch of owned tracks. Explicit re-enqueue is
    /// the retry action for failed content; completed cache entries are reused.
    pub async fn enqueue(&self, owner: &str, input: EnqueueAnalysis) -> Result<AnalysisReceipt> {
        if input.track_ids.is_empty() || input.track_ids.len() > MAX_BATCH {
            return Err(AnalysisError::Invalid("Select between 1 and 50 tracks".into()).into());
        }
        let mut unique = HashSet::with_capacity(input.track_ids.len());
        for id in &input.track_ids {
            validate_id(id)?;
            if !unique.insert(id.as_str()) {
                return Err(AnalysisError::Invalid("Track IDs must be unique".into()).into());
            }
        }
        // A reserved writer serializes authorization, quota checks and retries.
        // No writes happen until the complete batch is authorized.
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut assets = Vec::with_capacity(input.track_ids.len());
        for id in &input.track_ids {
            let asset: Option<String> =
                sqlx::query_scalar("SELECT media_asset_id FROM tracks WHERE owner_id=? AND id=?")
                    .bind(owner)
                    .bind(id)
                    .fetch_optional(&mut *transaction)
                    .await?;
            assets.push(asset.ok_or(AnalysisError::NotFound)?);
        }
        let mut pending = Vec::with_capacity(assets.len());
        for asset in assets {
            let status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM audio_analysis WHERE media_asset_id=? AND extractor_version=?",
            )
            .bind(&asset)
            .bind(EXTRACTOR_VERSION)
            .fetch_optional(&mut *transaction)
            .await?;
            if status.is_none() || status.as_deref() == Some("failed") {
                pending.push(asset);
            }
        }
        let additional = pending.len() as i64;
        if additional > 0 {
            let global: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM audio_analysis WHERE extractor_version=? AND status IN ('queued','running')",
            )
            .bind(EXTRACTOR_VERSION)
            .fetch_one(&mut *transaction)
            .await?;
            let owned: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM audio_analysis WHERE extractor_version=? AND requested_by=? AND status IN ('queued','running')",
            )
            .bind(EXTRACTOR_VERSION)
            .bind(owner)
            .fetch_one(&mut *transaction)
            .await?;
            if global + additional > MAX_GLOBAL_ACTIVE {
                return Err(
                    AnalysisError::Conflict("Analysis queue is full; retry later".into()).into(),
                );
            }
            if owned + additional > MAX_OWNER_ACTIVE {
                return Err(AnalysisError::Conflict(
                    "Your analysis queue is full; retry after existing work finishes".into(),
                )
                .into());
            }
            for asset in pending {
                sqlx::query(
                    "INSERT INTO audio_analysis(media_asset_id,extractor_version,requested_by,status,features_json,created_at,updated_at) VALUES(?,?,?,'queued',NULL,unixepoch(),unixepoch()) ON CONFLICT(media_asset_id,extractor_version) DO UPDATE SET requested_by=excluded.requested_by,status='queued',features_json=NULL,updated_at=unixepoch() WHERE audio_analysis.status='failed'",
                )
                .bind(asset)
                .bind(EXTRACTOR_VERSION)
                .bind(owner)
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        if additional > 0 {
            self.wake.notify_one();
        }
        Ok(AnalysisReceipt {
            queued: additional as u32,
        })
    }

    /// One owner-scoped join prevents the shared physical cache from becoming
    /// an oracle for foreign track identifiers or publication status.
    pub async fn get(&self, owner: &str, track_id: &str) -> Result<Option<TrackAnalysis>> {
        validate_id(track_id)?;
        let row = sqlx::query(
            "SELECT a.status,a.features_json FROM tracks t LEFT JOIN audio_analysis a ON a.media_asset_id=t.media_asset_id AND a.extractor_version=? WHERE t.owner_id=? AND t.id=?",
        )
        .bind(EXTRACTOR_VERSION)
        .bind(owner)
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let status: Option<String> = row.try_get("status")?;
        let status = match status.as_deref() {
            None => AnalysisStatus::NotAnalyzed,
            Some("queued") => AnalysisStatus::Queued,
            Some("running") => AnalysisStatus::Running,
            Some("complete") => AnalysisStatus::Complete,
            Some("failed") => AnalysisStatus::Failed,
            _ => anyhow::bail!("Invalid stored audio analysis state"),
        };
        let features = if status == AnalysisStatus::Complete {
            let raw: String = row.try_get("features_json")?;
            ensure!(
                raw.len() <= MAX_FEATURE_BYTES,
                "Audio analysis cache exceeds limit"
            );
            let features: AudioFeatures =
                serde_json::from_str(&raw).context("Invalid stored audio analysis features")?;
            ensure!(
                features.extractor_version == EXTRACTOR_VERSION,
                "Audio analysis cache version mismatch"
            );
            Some(features)
        } else {
            None
        };
        Ok(Some(TrackAnalysis {
            track_id: track_id.to_owned(),
            extractor_version: EXTRACTOR_VERSION.to_owned(),
            status,
            features,
            error: (status == AnalysisStatus::Failed).then(|| FAILED_MESSAGE.to_owned()),
        }))
    }

    /// One supervised engine per application service, including its clones.
    /// Interrupted current-version jobs are replayed after restart. Old version
    /// results remain distinct; no unversioned overwrite or decoder migration.
    pub async fn run(&self, shutdown: CancellationToken) -> Result<()> {
        let _guard = self
            .worker_lock
            .try_lock()
            .context("Audio analysis worker is already running")?;
        if shutdown.is_cancelled() {
            return Ok(());
        }
        // A version upgrade cannot execute an unknown historical algorithm.
        // Retire only its unfinished work, retaining all completed descriptors.
        // Current-version quota checks also work before this startup cleanup.
        sqlx::query(
            "UPDATE audio_analysis SET status='failed',features_json=NULL,updated_at=unixepoch() WHERE extractor_version<>? AND status IN ('queued','running')",
        )
        .bind(EXTRACTOR_VERSION)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "UPDATE audio_analysis SET status='queued',features_json=NULL,updated_at=unixepoch() WHERE extractor_version=? AND status='running'",
        )
        .bind(EXTRACTOR_VERSION)
        .execute(&self.pool)
        .await?;
        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }
            let Some(job) = self.claim().await? else {
                tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = self.wake.notified() => {},
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {},
                }
                continue;
            };
            self.execute(&job, &shutdown).await?;
        }
    }

    async fn claim(&self) -> Result<Option<ClaimedJob>> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let row = sqlx::query(
            "UPDATE audio_analysis SET status='running',updated_at=unixepoch() WHERE media_asset_id=(SELECT media_asset_id FROM audio_analysis WHERE extractor_version=? AND status='queued' ORDER BY created_at,media_asset_id LIMIT 1) AND extractor_version=? AND status='queued' RETURNING media_asset_id,requested_by",
        )
        .bind(EXTRACTOR_VERSION)
        .bind(EXTRACTOR_VERSION)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let asset_id: String = row.try_get("media_asset_id")?;
        let owner: Option<String> = row.try_get("requested_by")?;
        let track_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM tracks WHERE owner_id=? AND media_asset_id=? ORDER BY id LIMIT 1",
        )
        .bind(&owner)
        .bind(&asset_id)
        .fetch_optional(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(Some(ClaimedJob {
            asset_id,
            owner,
            track_id,
        }))
    }

    async fn execute(&self, job: &ClaimedJob, shutdown: &CancellationToken) -> Result<()> {
        if shutdown.is_cancelled() {
            return self.finish(job, "queued", None).await;
        }
        let (Some(owner), Some(track_id)) = (&job.owner, &job.track_id) else {
            return self.finish(job, "failed", None).await;
        };
        let Some(asset) = self.media.asset(owner, track_id).await? else {
            return self.finish(job, "failed", None).await;
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        let limits = AnalysisLimits {
            cancel: Some(cancelled.clone()),
            ..Default::default()
        };
        let mut task = tokio::task::spawn_blocking(move || analyze_file(&asset.path, limits));
        let result = tokio::select! {
            biased;
            _ = shutdown.cancelled() => {
                cancelled.store(true, Ordering::Relaxed);
                // Do not detach blocking decoding or open another engine while
                // its buffers/file remain alive. A panic is infrastructure failure.
                let _ = task.await.context("Audio analysis blocking worker failed")?;
                self.finish(job, "queued", None).await?;
                return Ok(());
            },
            result = &mut task => result.context("Audio analysis blocking worker failed")?,
        };
        if shutdown.is_cancelled() {
            return self.finish(job, "queued", None).await;
        }
        match result {
            Ok(features) => {
                let raw = serde_json::to_string(&features)
                    .context("Cannot serialize audio analysis features")?;
                ensure!(
                    raw.len() <= MAX_FEATURE_BYTES,
                    "Audio analysis features exceed limit"
                );
                self.finish(job, "complete", Some(&raw)).await
            }
            // Decoder/source errors are per-job and deliberately discarded.
            // No parser text, storage path or embedded metadata enters logs/DTOs.
            Err(_) => self.finish(job, "failed", None).await,
        }
    }

    async fn finish(&self, job: &ClaimedJob, status: &str, features: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE audio_analysis SET status=?,features_json=?,updated_at=unixepoch() WHERE media_asset_id=? AND extractor_version=? AND status='running'",
        )
        .bind(status)
        .bind(features)
        .bind(&job.asset_id)
        .bind(EXTRACTOR_VERSION)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<()> {
    if id.len() != 36
        || Uuid::parse_str(id)
            .ok()
            .is_none_or(|parsed| parsed.to_string() != id)
    {
        return Err(AnalysisError::Invalid("Track ID must be a canonical UUID".into()).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
    use tempfile::TempDir;
    use vela_domain::Track;

    const FLAC: &[u8] = include_bytes!("../tests/fixtures/signal.flac");
    const MP3: &[u8] = include_bytes!("../tests/fixtures/beacon.mp3");

    struct Fixture {
        directory: TempDir,
        pool: SqlitePool,
        media: MediaService,
        service: AnalysisService,
    }

    impl Fixture {
        async fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let options = SqliteConnectOptions::new()
                .filename(directory.path().join("vela.db"))
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(5));
            let pool = SqlitePoolOptions::new()
                .max_connections(4)
                .connect_with(options)
                .await
                .unwrap();
            // Actual relational/media migrations, independent of the core crate.
            for migration in [
                include_str!("../../../migrations/0001_initial.sql"),
                include_str!("../../../migrations/0002_catalog_groups.sql"),
                include_str!("../../../migrations/0003_catalog_metadata_revision.sql"),
                include_str!("../../../migrations/0009_content_audio_analysis.sql"),
            ] {
                sqlx::raw_sql(migration).execute(&pool).await.unwrap();
            }
            for owner in ["alice", "bob", "charlie", "dana"] {
                sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,?)")
                    .bind(owner)
                    .bind(owner)
                    .bind("isolated test password hash")
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            let media =
                MediaService::new(pool.clone(), directory.path().to_owned(), 16 * 1024 * 1024);
            media.initialize().await.unwrap();
            let service = AnalysisService::new(pool.clone(), media.clone());
            Self {
                directory,
                pool,
                media,
                service,
            }
        }

        async fn ingest(&self, owner: &str, bytes: &[u8], name: &str) -> Track {
            let path = self
                .directory
                .path()
                .join("staging")
                .join(Uuid::new_v4().to_string());
            tokio::fs::write(&path, bytes).await.unwrap();
            self.media.ingest(owner, &path, name).await.unwrap().track
        }

        async fn wait(&self, owner: &str, id: &str, expected: AnalysisStatus) -> TrackAnalysis {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let result = self.service.get(owner, id).await.unwrap().unwrap();
                    if result.status == expected {
                        return result;
                    }
                    assert_ne!(result.status, AnalysisStatus::Failed, "{result:?}");
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap()
        }

        /// Metadata-only quota fixtures; these deliberately do not run a decoder
        /// or masquerade as actual media ingestion/product playback evidence.
        async fn metadata_tracks(&self, owner: &str, count: usize) -> Vec<String> {
            let mut transaction = self.pool.begin().await.unwrap();
            let mut result = Vec::with_capacity(count);
            for _ in 0..count {
                let id = Uuid::new_v4().to_string();
                let hash = blake3::hash(id.as_bytes()).to_hex().to_string();
                sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',1)")
                    .bind(&id).bind(hash).bind("quota fixture").execute(&mut *transaction).await.unwrap();
                sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,codec,duration_ms,added_at) VALUES(?,?,?,'Quota fixture','FLAC',1000,0)")
                    .bind(&id).bind(owner).bind(&id).execute(&mut *transaction).await.unwrap();
                result.push(id);
            }
            transaction.commit().await.unwrap();
            result
        }
    }

    fn batch(ids: &[String]) -> EnqueueAnalysis {
        EnqueueAnalysis {
            track_ids: ids.to_vec(),
        }
    }

    #[tokio::test]
    async fn real_worker_decodes_flac_and_mp3_persists_and_reuses_cache() {
        let fixture = Fixture::new().await;
        let flac = fixture.ingest("alice", FLAC, "signal.flac").await;
        let mp3 = fixture.ingest("alice", MP3, "beacon.mp3").await;
        let ids = vec![flac.id.clone(), mp3.id.clone()];
        let summary = fixture.service.summary("alice").await.unwrap();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.not_analyzed, 2);
        assert_eq!(
            summary.queued + summary.running + summary.complete + summary.failed,
            0
        );
        assert_eq!(
            fixture
                .service
                .get("alice", &flac.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            AnalysisStatus::NotAnalyzed
        );
        assert_eq!(
            fixture
                .service
                .enqueue("alice", batch(&ids))
                .await
                .unwrap()
                .queued,
            2
        );
        assert_eq!(fixture.service.summary("alice").await.unwrap().queued, 2);
        assert_eq!(fixture.service.summary("bob").await.unwrap().total, 0);
        assert_eq!(
            fixture
                .service
                .enqueue("alice", batch(&ids))
                .await
                .unwrap()
                .queued,
            0
        );
        let shutdown = CancellationToken::new();
        let worker = tokio::spawn({
            let service = fixture.service.clone();
            let shutdown = shutdown.clone();
            async move { service.run(shutdown).await }
        });
        for id in &ids {
            let result = fixture.wait("alice", id, AnalysisStatus::Complete).await;
            let features = result.features.unwrap();
            assert_eq!(features.extractor_version, EXTRACTOR_VERSION);
            assert_eq!(features.sample_rate, 44_100);
            assert!(features.complete);
            assert!(features.rms > 0.0);
            assert!((900..=1100).contains(&features.analyzed_ms));
            assert!(features.spectral_centroid_hz.unwrap().is_finite());
        }
        shutdown.cancel();
        worker.await.unwrap().unwrap();
        let summary = fixture.service.summary("alice").await.unwrap();
        assert_eq!(summary.complete, 2);
        assert_eq!(
            summary.not_analyzed + summary.queued + summary.running + summary.failed,
            0
        );
        let restarted = AnalysisService::new(fixture.pool.clone(), fixture.media.clone());
        let before = restarted.get("alice", &flac.id).await.unwrap().unwrap();
        sqlx::query("UPDATE tracks SET title='Edited catalog title' WHERE id=?")
            .bind(&flac.id)
            .execute(&fixture.pool)
            .await
            .unwrap();
        let after = restarted.get("alice", &flac.id).await.unwrap().unwrap();
        assert_eq!(before.features, after.features);
        assert_eq!(
            restarted
                .enqueue("alice", batch(&ids))
                .await
                .unwrap()
                .queued,
            0
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audio_analysis")
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(rows, 2);
        for (id, expected) in [(&flac.id, FLAC), (&mp3.id, MP3)] {
            let asset = fixture.media.asset("alice", id).await.unwrap().unwrap();
            assert_eq!(tokio::fs::read(asset.path).await.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn foreign_batches_are_atomic_and_shared_bytes_do_not_grant_foreign_track_reads() {
        let fixture = Fixture::new().await;
        let alice = fixture.ingest("alice", FLAC, "signal.flac").await;
        let bob = fixture.ingest("bob", MP3, "beacon.mp3").await;
        let error = fixture
            .service
            .enqueue("alice", batch(&[alice.id.clone(), bob.id.clone()]))
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<AnalysisError>(),
            Some(AnalysisError::NotFound)
        ));
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audio_analysis")
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
        assert!(
            fixture
                .service
                .get("bob", &alice.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            fixture
                .service
                .get("alice", &bob.id)
                .await
                .unwrap()
                .is_none()
        );
        let own_same_bytes = fixture.ingest("bob", FLAC, "signal.flac").await;
        fixture
            .service
            .enqueue("alice", batch(std::slice::from_ref(&alice.id)))
            .await
            .unwrap();
        assert_eq!(
            fixture
                .service
                .enqueue("bob", batch(std::slice::from_ref(&own_same_bytes.id)))
                .await
                .unwrap()
                .queued,
            0
        );
        assert_eq!(
            fixture
                .service
                .get("bob", &own_same_bytes.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            AnalysisStatus::Queued
        );
        assert!(
            fixture
                .service
                .get("bob", &alice.id)
                .await
                .unwrap()
                .is_none()
        );
        let raw = serde_json::to_string(
            &fixture
                .service
                .get("bob", &own_same_bytes.id)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(!raw.contains("alice"));
        assert!(!raw.contains("requested_by"));
        assert!(!raw.contains("storage_path"));
        assert!(!raw.contains(fixture.directory.path().to_str().unwrap()));
    }

    #[tokio::test]
    async fn validation_owner_and_global_quotas_are_atomic() {
        let fixture = Fixture::new().await;
        let ids = fixture.metadata_tracks("alice", 51).await;
        for input in [
            batch(&[]),
            batch(&ids),
            batch(&[ids[0].clone(), ids[0].clone()]),
            batch(&["../../private".to_owned()]),
            batch(&["00000000-0000-0000-0000-00000000000A".to_owned()]),
        ] {
            let error = fixture.service.enqueue("alice", input).await.unwrap_err();
            assert!(matches!(
                error.downcast_ref::<AnalysisError>(),
                Some(AnalysisError::Invalid(_))
            ));
        }
        assert_eq!(
            fixture
                .service
                .enqueue("alice", batch(&ids[..50]))
                .await
                .unwrap()
                .queued,
            50
        );
        let error = fixture
            .service
            .enqueue("alice", batch(&ids[50..]))
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<AnalysisError>(),
            Some(AnalysisError::Conflict(_))
        ));
        // Idempotent work remains possible while a quota is full.
        assert_eq!(
            fixture
                .service
                .enqueue("alice", batch(&ids[..50]))
                .await
                .unwrap()
                .queued,
            0
        );
        let bob = fixture.metadata_tracks("bob", 50).await;
        let charlie = fixture.metadata_tracks("charlie", 28).await;
        let dana = fixture.metadata_tracks("dana", 1).await;
        fixture.service.enqueue("bob", batch(&bob)).await.unwrap();
        fixture
            .service
            .enqueue("charlie", batch(&charlie))
            .await
            .unwrap();
        let error = fixture
            .service
            .enqueue("dana", batch(&dana))
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<AnalysisError>(),
            Some(AnalysisError::Conflict(_))
        ));
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audio_analysis WHERE status='queued'")
                .fetch_one(&fixture.pool)
                .await
                .unwrap();
        assert_eq!(rows, 128);
        assert_eq!(
            fixture
                .service
                .get("dana", &dana[0])
                .await
                .unwrap()
                .unwrap()
                .status,
            AnalysisStatus::NotAnalyzed
        );
    }

    #[tokio::test]
    async fn concurrent_enqueue_cannot_exceed_requester_quota() {
        let fixture = Fixture::new().await;
        let ids = fixture.metadata_tracks("alice", 51).await;
        let (first, second) = tokio::join!(
            fixture.service.enqueue("alice", batch(&ids[..25])),
            fixture.service.enqueue("alice", batch(&ids[25..])),
        );
        assert_ne!(first.is_ok(), second.is_ok());
        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audio_analysis WHERE requested_by='alice' AND status='queued'",
        )
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
        assert!(rows == 25 || rows == 26);
    }

    #[tokio::test]
    async fn extractor_upgrade_retires_stale_work_without_blocking_current_jobs() {
        let fixture = Fixture::new().await;
        let track = fixture.ingest("alice", FLAC, "signal.flac").await;
        let asset: String = sqlx::query_scalar("SELECT media_asset_id FROM tracks WHERE id=?")
            .bind(&track.id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
        // Historical-version state fixture over an actually ingested asset.
        // Old algorithms are not run or claimed as verified extractions here.
        for version in 0..128 {
            sqlx::query("INSERT INTO audio_analysis(media_asset_id,extractor_version,requested_by,status,created_at,updated_at) VALUES(?,?,'alice','queued',0,0)")
                .bind(&asset).bind(format!("historical-fixture-{version}")).execute(&fixture.pool).await.unwrap();
        }
        sqlx::query("INSERT INTO audio_analysis(media_asset_id,extractor_version,requested_by,status,created_at,updated_at) VALUES(?,'historical-running','alice','running',0,0)")
            .bind(&asset).execute(&fixture.pool).await.unwrap();
        let preserved = r#"{"old_descriptor_fixture":true}"#;
        sqlx::query("INSERT INTO audio_analysis(media_asset_id,extractor_version,requested_by,status,features_json,created_at,updated_at) VALUES(?,'historical-complete','alice','complete',?,0,0)")
            .bind(&asset).bind(preserved).execute(&fixture.pool).await.unwrap();
        // This runs before worker startup: stale work must consume neither
        // current global quota nor the current requester's quota.
        assert_eq!(
            fixture
                .service
                .enqueue("alice", batch(std::slice::from_ref(&track.id)))
                .await
                .unwrap()
                .queued,
            1
        );
        let summary = fixture.service.summary("alice").await.unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.queued, 1);
        let shutdown = CancellationToken::new();
        let worker = tokio::spawn({
            let service = fixture.service.clone();
            let shutdown = shutdown.clone();
            async move { service.run(shutdown).await }
        });
        fixture
            .wait("alice", &track.id, AnalysisStatus::Complete)
            .await;
        shutdown.cancel();
        worker.await.unwrap().unwrap();
        let retired: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audio_analysis WHERE extractor_version<>? AND status='failed'",
        )
        .bind(EXTRACTOR_VERSION)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
        assert_eq!(retired, 129);
        let old: String = sqlx::query_scalar("SELECT features_json FROM audio_analysis WHERE extractor_version='historical-complete'")
            .fetch_one(&fixture.pool).await.unwrap();
        assert_eq!(old, preserved);
        let status: String = sqlx::query_scalar(
            "SELECT status FROM audio_analysis WHERE extractor_version='historical-complete'",
        )
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
        assert_eq!(status, "complete");
    }

    #[tokio::test]
    async fn restarted_running_job_and_interrupted_claim_are_replayed() {
        let fixture = Fixture::new().await;
        let track = fixture.ingest("alice", FLAC, "signal.flac").await;
        fixture
            .service
            .enqueue("alice", batch(std::slice::from_ref(&track.id)))
            .await
            .unwrap();
        let claimed = fixture.service.claim().await.unwrap().unwrap();
        assert_eq!(
            fixture
                .service
                .get("alice", &track.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            AnalysisStatus::Running
        );
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        fixture.service.execute(&claimed, &cancelled).await.unwrap();
        assert_eq!(
            fixture
                .service
                .get("alice", &track.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            AnalysisStatus::Queued
        );
        // Claim again without finishing, as an actual interrupted DB checkpoint.
        fixture.service.claim().await.unwrap().unwrap();
        let restarted = AnalysisService::new(fixture.pool.clone(), fixture.media.clone());
        let shutdown = CancellationToken::new();
        let worker = tokio::spawn({
            let shutdown = shutdown.clone();
            async move { restarted.run(shutdown).await }
        });
        fixture
            .wait("alice", &track.id, AnalysisStatus::Complete)
            .await;
        shutdown.cancel();
        worker.await.unwrap().unwrap();
        let running: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audio_analysis WHERE status='running'")
                .fetch_one(&fixture.pool)
                .await
                .unwrap();
        assert_eq!(running, 0);
    }

    #[tokio::test]
    async fn actual_decoder_failure_is_sanitized_and_explicit_retry_recovers() {
        let fixture = Fixture::new().await;
        let track = fixture.ingest("alice", MP3, "beacon.mp3").await;
        let asset = fixture
            .media
            .asset("alice", &track.id)
            .await
            .unwrap()
            .unwrap();
        // Deliberately damage only this isolated fixture's published bytes.
        tokio::fs::write(&asset.path, b"secret-parser-source-path-invalid-audio")
            .await
            .unwrap();
        fixture
            .service
            .enqueue("alice", batch(std::slice::from_ref(&track.id)))
            .await
            .unwrap();
        let shutdown = CancellationToken::new();
        let worker = tokio::spawn({
            let service = fixture.service.clone();
            let shutdown = shutdown.clone();
            async move { service.run(shutdown).await }
        });
        let failure = fixture
            .wait("alice", &track.id, AnalysisStatus::Failed)
            .await;
        assert_eq!(failure.error.as_deref(), Some(FAILED_MESSAGE));
        assert!(failure.features.is_none());
        let raw = serde_json::to_string(&failure).unwrap();
        assert!(!raw.contains("secret-parser"));
        assert!(!raw.contains(asset.path.to_str().unwrap()));
        tokio::fs::write(&asset.path, MP3).await.unwrap();
        assert_eq!(
            fixture
                .service
                .enqueue("alice", batch(std::slice::from_ref(&track.id)))
                .await
                .unwrap()
                .queued,
            1
        );
        fixture
            .wait("alice", &track.id, AnalysisStatus::Complete)
            .await;
        shutdown.cancel();
        worker.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn actual_active_worker_shutdown_drains_and_never_leaves_running_state() {
        let fixture = Fixture::new().await;
        // Original 20-second, 8-channel PCM WAV makes the active decode observable
        // without injecting a fake decoder or adding production test hooks.
        let rate = 48_000_u32;
        let channels = 8_u16;
        let frames = rate * 20;
        let size = frames * u32::from(channels) * 2;
        let mut bytes = Vec::with_capacity(size as usize + 44);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(size + 36).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        bytes.extend_from_slice(&(channels * 2).to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&size.to_le_bytes());
        for frame in 0..frames {
            let sample = if frame % 48 < 24 { 1000_i16 } else { -1000_i16 };
            for _ in 0..channels {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }
        let track = fixture.ingest("alice", &bytes, "shutdown.wav").await;
        fixture
            .service
            .enqueue("alice", batch(std::slice::from_ref(&track.id)))
            .await
            .unwrap();
        let shutdown = CancellationToken::new();
        let worker = tokio::spawn({
            let service = fixture.service.clone();
            let shutdown = shutdown.clone();
            async move { service.run(shutdown).await }
        });
        fixture
            .wait("alice", &track.id, AnalysisStatus::Running)
            .await;
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(10), worker)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let result = fixture
            .service
            .get("alice", &track.id)
            .await
            .unwrap()
            .unwrap();
        // Completion racing cancellation is also durable and safe. In either
        // outcome the blocking engine has exited and a restart loses no job.
        assert!(matches!(
            result.status,
            AnalysisStatus::Queued | AnalysisStatus::Complete
        ));
        if result.status == AnalysisStatus::Complete {
            assert!(result.features.is_some());
        } else {
            assert!(result.features.is_none());
        }
        let asset = fixture
            .media
            .asset("alice", &track.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(tokio::fs::read(asset.path).await.unwrap(), bytes);
    }

    #[tokio::test]
    async fn duplicate_worker_is_rejected_and_database_failure_is_fatal() {
        let fixture = Fixture::new().await;
        let shutdown = CancellationToken::new();
        let worker = tokio::spawn({
            let service = fixture.service.clone();
            let shutdown = shutdown.clone();
            async move { service.run(shutdown).await }
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if fixture.service.worker_lock.try_lock().is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(fixture.service.run(CancellationToken::new()).await.is_err());
        // Real infrastructure error in an isolated fixture, not fake success.
        sqlx::query("DROP TABLE audio_analysis")
            .execute(&fixture.pool)
            .await
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(2), worker)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_err());
        shutdown.cancel();
    }
}
