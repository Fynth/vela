//! Private, durable, read-only folder ingestion into immutable managed storage.
mod filesystem;

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vela_domain::{CreateScanSource, ScanJob, ScanSource, StorageRoot};
use vela_media::MediaService;

use filesystem::{FileError, Step, Walker};

#[derive(Clone)]
pub struct RootConfig {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ScanLimits {
    pub max_depth: u32,
    pub max_entries: u64,
    pub max_files: u64,
    pub max_sources: u32,
    pub max_active_jobs: u32,
    pub max_owner_jobs: u32,
    pub min_free_bytes: u64,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_depth: 32,
            max_entries: 1_000_000,
            max_files: 100_000,
            max_sources: 32,
            max_active_jobs: 8,
            max_owner_jobs: 2,
            min_free_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Music folder source was not found")]
    NotFound,
}

#[derive(Clone)]
pub struct Scanner {
    pool: SqlitePool,
    media: MediaService,
    staging: PathBuf,
    max_file_bytes: u64,
    roots: Arc<Vec<RootConfig>>,
    limits: ScanLimits,
    active: Arc<Mutex<HashMap<String, CancellationToken>>>,
    worker_lock: Arc<Mutex<()>>,
    wake: Arc<Notify>,
}

struct ClaimedJob {
    id: String,
    owner: String,
    source: String,
    root: String,
}

#[derive(Default)]
struct Counters {
    visited: u64,
    imported: u64,
    duplicates: u64,
    unchanged: u64,
    skipped: u64,
    failed: u64,
}

impl Scanner {
    pub fn new(
        pool: SqlitePool,
        media: MediaService,
        data_dir: PathBuf,
        max_file_bytes: u64,
        roots: Vec<RootConfig>,
    ) -> Self {
        Self {
            pool,
            media,
            staging: data_dir.join("staging"),
            max_file_bytes: max_file_bytes.min(i64::MAX as u64),
            roots: Arc::new(roots),
            limits: ScanLimits::default(),
            active: Arc::new(Mutex::new(HashMap::new())),
            worker_lock: Arc::new(Mutex::new(())),
            wake: Arc::new(Notify::new()),
        }
    }

    pub fn with_limits(mut self, mut limits: ScanLimits) -> Self {
        // Runtime profiles can reduce these ceilings, never make source input
        // allocate an unbounded descriptor stack or persistent fingerprint map.
        limits.max_depth = limits.max_depth.clamp(1, 64);
        limits.max_entries = limits.max_entries.min(i64::MAX as u64 - 1);
        limits.max_files = limits.max_files.min(1_000_000);
        limits.max_sources = limits.max_sources.min(32);
        limits.max_active_jobs = limits.max_active_jobs.min(64);
        limits.max_owner_jobs = limits.max_owner_jobs.min(limits.max_active_jobs);
        self.limits = limits;
        self
    }

    pub async fn initialize(&self) -> Result<()> {
        let _guard = self
            .worker_lock
            .try_lock()
            .context("Scanner is already running")?;
        tokio::fs::create_dir_all(&self.staging)
            .await
            .context("Scanner temporary storage is unavailable")?;
        self.cleanup_staging().await
    }

    pub async fn roots(&self) -> Result<Vec<StorageRoot>> {
        let roots = self.roots.clone();
        tokio::task::spawn_blocking(move || {
            roots
                .iter()
                .map(|root| StorageRoot {
                    id: root.id.clone(),
                    name: root.name.clone(),
                    available: filesystem::open_root(&root.path).is_ok(),
                })
                .collect()
        })
        .await
        .context("Storage folder probe worker failed")
    }

    pub async fn list_sources(&self, owner: &str) -> Result<Vec<ScanSource>> {
        let rows = sqlx::query(
            "SELECT id,name,root_id,last_scan_at FROM scan_sources WHERE owner_id=? ORDER BY name,id LIMIT ?",
        )
        .bind(owner)
        .bind(i64::from(self.limits.max_sources))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(source_from_row).collect()
    }

    pub async fn create_source(&self, owner: &str, input: CreateScanSource) -> Result<ScanSource> {
        let name = input.name.trim();
        if name.is_empty() || name.len() > 120 || name.chars().any(char::is_control) {
            return Err(
                ScanError::Invalid("Source name must contain 1–120 safe bytes".into()).into(),
            );
        }
        if !self.roots.iter().any(|root| root.id == input.root_id) {
            return Err(ScanError::Invalid("Choose a configured music folder".into()).into());
        }
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_sources WHERE owner_id=?")
            .bind(owner)
            .fetch_one(&mut *transaction)
            .await?;
        if count >= i64::from(self.limits.max_sources) {
            return Err(ScanError::Conflict("Music folder source limit reached".into()).into());
        }
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scan_sources WHERE owner_id=? AND root_id=?")
                .bind(owner)
                .bind(&input.root_id)
                .fetch_one(&mut *transaction)
                .await?;
        if exists != 0 {
            return Err(
                ScanError::Conflict("This music folder already has a source".into()).into(),
            );
        }
        let source = ScanSource {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            root_id: input.root_id,
            last_scan_at: None,
        };
        sqlx::query("INSERT INTO scan_sources(id,owner_id,name,root_id) VALUES(?,?,?,?)")
            .bind(&source.id)
            .bind(owner)
            .bind(&source.name)
            .bind(&source.root_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(source)
    }

    pub async fn remove_source(&self, owner: &str, id: &str) -> Result<bool> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scan_sources WHERE owner_id=? AND id=?")
                .bind(owner)
                .bind(id)
                .fetch_one(&mut *transaction)
                .await?;
        if exists == 0 {
            return Ok(false);
        }
        let active: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM scan_jobs WHERE owner_id=? AND source_id=? AND status IN ('queued','running')",
        )
        .bind(owner)
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;
        if active != 0 {
            return Err(ScanError::Conflict(
                "Cancel the active scan before removing its source".into(),
            )
            .into());
        }
        sqlx::query("DELETE FROM scan_sources WHERE owner_id=? AND id=?")
            .bind(owner)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn enqueue(&self, owner: &str, source: &str) -> Result<ScanJob> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let root: Option<String> =
            sqlx::query_scalar("SELECT root_id FROM scan_sources WHERE owner_id=? AND id=?")
                .bind(owner)
                .bind(source)
                .fetch_optional(&mut *transaction)
                .await?;
        let Some(root) = root else {
            return Err(ScanError::NotFound.into());
        };
        if !self.roots.iter().any(|configured| configured.id == root) {
            return Err(
                ScanError::Invalid("Source music folder is no longer configured".into()).into(),
            );
        }
        let counts = sqlx::query(
            "SELECT COUNT(*) AS total,COALESCE(SUM(owner_id=?),0) AS owner,COALESCE(SUM(source_id=?),0) AS source FROM scan_jobs WHERE status IN ('queued','running')",
        )
        .bind(owner)
        .bind(source)
        .fetch_one(&mut *transaction)
        .await?;
        if counts.try_get::<i64, _>("source")? != 0 {
            return Err(
                ScanError::Conflict("This source already has an active scan".into()).into(),
            );
        }
        if counts.try_get::<i64, _>("total")? >= i64::from(self.limits.max_active_jobs)
            || counts.try_get::<i64, _>("owner")? >= i64::from(self.limits.max_owner_jobs)
        {
            return Err(ScanError::Conflict(
                "Scan queue is full; wait for an active scan to finish".into(),
            )
            .into());
        }
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO scan_jobs(id,owner_id,source_id,status) VALUES(?,?,?,'queued')")
            .bind(&id)
            .bind(owner)
            .bind(source)
            .execute(&mut *transaction)
            .await?;
        let row = sqlx::query("SELECT * FROM scan_jobs WHERE id=? AND owner_id=?")
            .bind(&id)
            .bind(owner)
            .fetch_one(&mut *transaction)
            .await?;
        let job = job_from_row(&row)?;
        transaction.commit().await?;
        self.wake.notify_one();
        Ok(job)
    }

    pub async fn jobs(&self, owner: &str) -> Result<Vec<ScanJob>> {
        sqlx::query(
            "SELECT * FROM scan_jobs WHERE owner_id=? ORDER BY created_at DESC,id DESC LIMIT 100",
        )
        .bind(owner)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(job_from_row)
        .collect()
    }

    pub async fn cancel(&self, owner: &str, id: &str) -> Result<bool> {
        let changed = sqlx::query(
            "UPDATE scan_jobs SET cancel_requested=1,status=CASE WHEN status='queued' THEN 'cancelled' ELSE status END,updated_at=unixepoch(),finished_at=CASE WHEN status='queued' THEN unixepoch() ELSE finished_at END WHERE owner_id=? AND id=? AND status IN ('queued','running')",
        )
        .bind(owner)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if changed != 0 {
            if let Some(token) = self.active.lock().await.get(id) {
                token.cancel();
            }
            self.wake.notify_one();
        }
        Ok(changed != 0)
    }

    /// Serial worker. Publication is awaited even when shutdown/cancel arrives.
    pub async fn run(&self, shutdown: CancellationToken) -> Result<()> {
        let _guard = self
            .worker_lock
            .try_lock()
            .context("Scanner is already running")?;
        self.cleanup_staging().await?;
        sqlx::query("UPDATE scan_jobs SET status=CASE WHEN cancel_requested=1 THEN 'cancelled' ELSE 'queued' END,visited=0,imported=0,duplicates=0,unchanged=0,skipped=0,failed_files=0,error=NULL,updated_at=unixepoch(),finished_at=CASE WHEN cancel_requested=1 THEN unixepoch() ELSE NULL END WHERE status='running'")
            .execute(&self.pool).await?;
        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }
            let Some(job) = self.claim().await? else {
                tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = self.wake.notified() => {},
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }
                continue;
            };
            let cancel = CancellationToken::new();
            self.active
                .lock()
                .await
                .insert(job.id.clone(), cancel.clone());
            let cancelled: bool = sqlx::query_scalar(
                "SELECT cancel_requested FROM scan_jobs WHERE id=? AND owner_id=?",
            )
            .bind(&job.id)
            .bind(&job.owner)
            .fetch_one(&self.pool)
            .await?;
            if cancelled {
                cancel.cancel();
            }
            let result = self.process(&job, shutdown.clone(), cancel).await;
            self.active.lock().await.remove(&job.id);
            match result {
                Ok(()) => self.finish(&job, "completed", None).await?,
                Err(error) => {
                    if let Some(file_error) = error.downcast_ref::<FileError>() {
                        match file_error {
                            FileError::Shutdown => self.finish(&job, "queued", None).await?,
                            FileError::Cancelled => self.finish(&job, "cancelled", None).await?,
                            _ => {
                                self.finish(&job, "failed", Some(file_error.to_string()))
                                    .await?
                            }
                        }
                    } else {
                        // Infrastructure errors remain fatal; don't silently label
                        // a broken database or panicked blocking worker as bad music.
                        self.finish(&job, "failed", Some("Scanner infrastructure failed".into()))
                            .await?;
                        return Err(error.context("Scanner infrastructure failed"));
                    }
                }
            }
        }
    }

    async fn claim(&self) -> Result<Option<ClaimedJob>> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let row = sqlx::query("SELECT j.id,j.owner_id,j.source_id,s.root_id FROM scan_jobs j JOIN scan_sources s ON s.id=j.source_id AND s.owner_id=j.owner_id WHERE j.status='queued' AND j.cancel_requested=0 ORDER BY j.created_at,j.id LIMIT 1")
            .fetch_optional(&mut *transaction).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let job = ClaimedJob {
            id: row.try_get("id")?,
            owner: row.try_get("owner_id")?,
            source: row.try_get("source_id")?,
            root: row.try_get("root_id")?,
        };
        sqlx::query("UPDATE scan_jobs SET status='running',visited=0,imported=0,duplicates=0,unchanged=0,skipped=0,failed_files=0,error=NULL,finished_at=NULL,updated_at=unixepoch() WHERE id=? AND status='queued'")
            .bind(&job.id).execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(job))
    }

    async fn process(
        &self,
        job: &ClaimedJob,
        shutdown: CancellationToken,
        cancel: CancellationToken,
    ) -> Result<()> {
        filesystem::interrupted(&shutdown, &cancel)?;
        let root = self
            .roots
            .iter()
            .find(|root| root.id == job.root)
            .ok_or(FileError::Root)?;
        let path = root.path.clone();
        let limits = self.limits.clone();
        let mut walker = tokio::task::spawn_blocking(move || Walker::new(&path, limits))
            .await
            .context("Folder traversal worker failed")??;
        let mut counters = Counters::default();
        let mut map_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scan_files WHERE source_id=?")
                .bind(&job.source)
                .fetch_one(&self.pool)
                .await?;
        loop {
            let shutdown_step = shutdown.clone();
            let cancel_step = cancel.clone();
            let (next_walker, step) = tokio::task::spawn_blocking(move || {
                let step = walker.next(&shutdown_step, &cancel_step);
                (walker, step)
            })
            .await
            .context("Folder traversal worker failed")?;
            walker = next_walker;
            counters.visited = walker.visited;
            counters.skipped = walker.skipped;
            let step = match step {
                Ok(step) => step,
                Err(error) => {
                    self.progress(job, &counters).await?;
                    return Err(error.into());
                }
            };
            match step {
                Step::Done => {
                    self.progress(job, &counters).await?;
                    return Ok(());
                }
                Step::Progress => {
                    self.progress(job, &counters).await?;
                    continue;
                }
                Step::Candidate(candidate) => {
                    let known = sqlx::query("SELECT f.fingerprint,f.track_id FROM scan_files f JOIN tracks t ON t.id=f.track_id WHERE f.source_id=? AND f.path_key=? AND t.owner_id=?")
                        .bind(&job.source).bind(&candidate.path_key).bind(&job.owner)
                        .fetch_optional(&self.pool).await?;
                    if known.as_ref().is_some_and(|row| {
                        row.get::<String, _>("fingerprint") == candidate.fingerprint
                    }) {
                        counters.unchanged += 1;
                        self.progress(job, &counters).await?;
                        continue;
                    }
                    if known.is_none() && (map_count as u64) >= self.limits.max_files {
                        return Err(FileError::Files.into());
                    }
                    let staging = self.staging.clone();
                    let max_bytes = self.max_file_bytes;
                    let reserve = self.limits.min_free_bytes;
                    let copy_shutdown = shutdown.clone();
                    let copy_cancel = cancel.clone();
                    let snapshot = tokio::task::spawn_blocking(move || {
                        filesystem::snapshot(
                            candidate,
                            &staging,
                            max_bytes,
                            reserve,
                            &copy_shutdown,
                            &copy_cancel,
                        )
                    })
                    .await
                    .context("Folder snapshot worker failed")?;
                    let (temporary, candidate) = match snapshot {
                        Ok(snapshot) => snapshot,
                        Err(
                            error @ (FileError::Cancelled
                            | FileError::Shutdown
                            | FileError::Space
                            | FileError::Write),
                        ) => {
                            self.progress(job, &counters).await?;
                            return Err(error.into());
                        }
                        Err(_) => {
                            counters.failed += 1;
                            self.progress(job, &counters).await?;
                            continue;
                        }
                    };
                    filesystem::interrupted(&shutdown, &cancel)?;
                    let uploaded = self
                        .media
                        .ingest(&job.owner, temporary.path(), &candidate.filename)
                        .await;
                    match uploaded {
                        Ok(uploaded) => {
                            // Finish the atomic file publication and fingerprint commit
                            // before honoring cancellation. Dedup preserves catalog edits.
                            sqlx::query("INSERT INTO scan_files(source_id,path_key,fingerprint,track_id) VALUES(?,?,?,?) ON CONFLICT(source_id,path_key) DO UPDATE SET fingerprint=excluded.fingerprint,track_id=excluded.track_id,updated_at=unixepoch()")
                                .bind(&job.source).bind(&candidate.path_key).bind(&candidate.fingerprint).bind(&uploaded.track.id)
                                .execute(&self.pool).await?;
                            if known.is_none() {
                                map_count += 1;
                            }
                            if uploaded.duplicate {
                                counters.duplicates += 1;
                            } else {
                                counters.imported += 1;
                            }
                        }
                        Err(error) => {
                            if error.downcast_ref::<sqlx::Error>().is_some()
                                || error.downcast_ref::<tokio::task::JoinError>().is_some()
                            {
                                return Err(error.context("Scanned audio publication failed"));
                            }
                            counters.failed += 1;
                        }
                    }
                    drop(temporary);
                    self.progress(job, &counters).await?;
                }
            }
        }
    }

    async fn progress(&self, job: &ClaimedJob, counters: &Counters) -> Result<()> {
        sqlx::query("UPDATE scan_jobs SET visited=?,imported=?,duplicates=?,unchanged=?,skipped=?,failed_files=?,updated_at=unixepoch() WHERE id=? AND owner_id=? AND status='running'")
            .bind(counters.visited.min(i64::MAX as u64) as i64)
            .bind(counters.imported.min(i64::MAX as u64) as i64)
            .bind(counters.duplicates.min(i64::MAX as u64) as i64)
            .bind(counters.unchanged.min(i64::MAX as u64) as i64)
            .bind(counters.skipped.min(i64::MAX as u64) as i64)
            .bind(counters.failed.min(i64::MAX as u64) as i64)
            .bind(&job.id).bind(&job.owner).execute(&self.pool).await?;
        Ok(())
    }

    async fn finish(&self, job: &ClaimedJob, status: &str, error: Option<String>) -> Result<()> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let changed = sqlx::query("UPDATE scan_jobs SET status=CASE WHEN cancel_requested=1 THEN 'cancelled' ELSE ? END,error=?,updated_at=unixepoch(),finished_at=CASE WHEN cancel_requested=1 OR ? != 'queued' THEN unixepoch() ELSE NULL END WHERE id=? AND owner_id=? AND status='running'")
            .bind(status).bind(error).bind(status).bind(&job.id).bind(&job.owner)
            .execute(&mut *transaction).await?.rows_affected();
        if changed != 1 {
            bail!("Scan job state could not be saved");
        }
        if status == "completed" {
            sqlx::query("UPDATE scan_sources SET last_scan_at=unixepoch() WHERE id=? AND owner_id=? AND EXISTS(SELECT 1 FROM scan_jobs WHERE id=? AND status='completed')")
                .bind(&job.source).bind(&job.owner).bind(&job.id).execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        tracing::info!(job_id=%job.id, "Folder scan reached a durable checkpoint");
        Ok(())
    }

    async fn cleanup_staging(&self) -> Result<()> {
        let staging = self.staging.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let entries =
                std::fs::read_dir(&staging).context("Scanner temporary storage is unavailable")?;
            for entry in entries {
                let entry = entry.context("Scanner temporary storage could not be listed")?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                let recognized = name
                    .strip_prefix("scan-")
                    .and_then(|name| name.strip_suffix(".part"))
                    .is_some_and(|random| {
                        random.len() == 16
                            && random.bytes().all(|byte| byte.is_ascii_alphanumeric())
                    });
                if recognized
                    && entry
                        .file_type()
                        .context("Scanner temporary entry is unavailable")?
                        .is_file()
                {
                    std::fs::remove_file(entry.path())
                        .context("Scanner abandoned snapshot could not be removed")?;
                }
            }
            Ok(())
        })
        .await
        .context("Scanner cleanup worker failed")?
    }
}

fn source_from_row(row: &SqliteRow) -> Result<ScanSource> {
    Ok(ScanSource {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        root_id: row.try_get("root_id")?,
        last_scan_at: row.try_get("last_scan_at")?,
    })
}

fn job_from_row(row: &SqliteRow) -> Result<ScanJob> {
    Ok(ScanJob {
        id: row.try_get("id")?,
        source_id: row.try_get("source_id")?,
        status: row.try_get("status")?,
        visited: row.try_get("visited")?,
        imported: row.try_get("imported")?,
        duplicates: row.try_get("duplicates")?,
        unchanged: row.try_get("unchanged")?,
        skipped: row.try_get("skipped")?,
        failed_files: row.try_get("failed_files")?,
        error: row.try_get("error")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        finished_at: row.try_get("finished_at")?,
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    const FLAC: &[u8] = include_bytes!("../../media/tests/fixtures/signal.flac");

    #[tokio::test]
    async fn genuine_crash_abandoned_snapshot_is_recognized_and_cleaned() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("sources");
        let staging = directory.path().join("staging");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&staging).unwrap();
        std::fs::write(root.join("real.flac"), FLAC).unwrap();
        let shutdown = CancellationToken::new();
        let cancel = CancellationToken::new();
        let mut walker = Walker::new(&root, ScanLimits::default()).unwrap();
        let Step::Candidate(candidate) = walker.next(&shutdown, &cancel).unwrap() else {
            panic!("real file descriptor expected");
        };
        let (temporary, _) =
            filesystem::snapshot(candidate, &staging, 1024 * 1024, 0, &shutdown, &cancel).unwrap();
        let (file, abandoned) = temporary.keep().unwrap();
        drop(file);
        assert_eq!(std::fs::read(&abandoned).unwrap(), FLAC);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();
        let media = MediaService::new(pool.clone(), directory.path().to_owned(), 1024 * 1024);
        let scanner = Scanner::new(
            pool,
            media,
            directory.path().to_owned(),
            1024 * 1024,
            vec![],
        );
        scanner.initialize().await.unwrap();
        assert!(
            !abandoned.exists(),
            "cleanup must match the actual snapshot generator"
        );
        assert_eq!(std::fs::read(root.join("real.flac")).unwrap(), FLAC);
    }

    #[test]
    fn snapshot_rejects_changed_descriptor_oversize_and_cancel_without_source_writes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("sources");
        let staging = directory.path().join("staging");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&staging).unwrap();
        let original = root.join("real.flac");
        std::fs::write(&original, FLAC).unwrap();
        let shutdown = CancellationToken::new();
        let cancel = CancellationToken::new();
        let candidate = |shutdown: &CancellationToken, cancel: &CancellationToken| {
            let mut walker = Walker::new(&root, ScanLimits::default()).unwrap();
            let Step::Candidate(candidate) = walker.next(shutdown, cancel).unwrap() else {
                panic!("file expected");
            };
            candidate
        };
        let selected = candidate(&shutdown, &cancel);
        let mut changed = FLAC.to_vec();
        changed[100] ^= 1;
        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(&original, &changed).unwrap();
        assert!(matches!(
            filesystem::snapshot(selected, &staging, 1024 * 1024, 0, &shutdown, &cancel),
            Err(FileError::Changed)
        ));
        let selected = candidate(&shutdown, &cancel);
        assert!(matches!(
            filesystem::snapshot(
                selected,
                &staging,
                FLAC.len() as u64 - 1,
                0,
                &shutdown,
                &cancel
            ),
            Err(FileError::Size)
        ));
        let selected = candidate(&shutdown, &cancel);
        cancel.cancel();
        assert!(matches!(
            filesystem::snapshot(selected, &staging, 1024 * 1024, 0, &shutdown, &cancel),
            Err(FileError::Cancelled)
        ));
        assert_eq!(std::fs::read(original).unwrap(), changed);
        assert_eq!(std::fs::read_dir(staging).unwrap().count(), 0);
    }
}
