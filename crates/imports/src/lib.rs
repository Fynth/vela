//! Durable direct-audio imports with bounded downloads and private-by-default catalog publication.
mod egress;
mod source;

pub use egress::{ControlledHttp, public_address};
pub use source::{DirectAudioSource, ImportSource, ResolvedAsset};

use anyhow::{Result, bail};
use futures_util::StreamExt;
use reqwest::Method;
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
    sync::{Mutex, Notify},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vela_domain::{ImportJob, ImportPreview};
use vela_media::MediaService;

/// Error text is deliberately static and excludes request URLs, file paths and remote response bodies.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{0}")]
    Policy(&'static str),
    #[error("{0}")]
    Transient(&'static str),
    #[error("Remote server returned HTTP {0}")]
    Http(u16),
    #[error("Audio file exceeds the configured file-size limit")]
    TooLarge,
    #[error("Import storage quota or free-space reserve exceeded")]
    StorageQuota,
    #[error("Temporary storage could not be written")]
    Storage,
    #[error("Imported file is not valid supported audio")]
    Media,
    #[error("Import job state could not be saved")]
    Database,
    #[error("Import cancelled")]
    Cancelled,
    #[error("Server is shutting down")]
    Shutdown,
}

#[derive(Clone, Debug)]
pub struct ImportLimits {
    pub max_active_jobs: u32,
    pub max_active_jobs_per_owner: u32,
    /// Global managed media quota checked before and after each import download.
    pub max_storage_bytes: u64,
    pub reserve_free_bytes: u64,
    pub transfer_timeout: Duration,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_active_jobs: 64,
            max_active_jobs_per_owner: 8,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            reserve_free_bytes: 64 * 1024 * 1024,
            transfer_timeout: Duration::from_secs(900),
        }
    }
}

#[derive(Clone)]
pub struct ImportService {
    pool: SqlitePool,
    media: MediaService,
    staging_dir: PathBuf,
    max_file_bytes: u64,
    limits: ImportLimits,
    http: ControlledHttp,
    source: DirectAudioSource,
    active: Arc<Mutex<HashMap<String, CancellationToken>>>,
    worker_lock: Arc<Mutex<()>>,
    wake: Arc<Notify>,
}

struct ClaimedJob {
    job: ImportJob,
    owner_id: String,
    attempt: i64,
}

impl ImportService {
    pub fn new(
        pool: SqlitePool,
        media: MediaService,
        data_dir: PathBuf,
        max_file_bytes: u64,
    ) -> Self {
        Self {
            pool,
            media,
            staging_dir: data_dir.join("staging").join("imports"),
            max_file_bytes: max_file_bytes.min(i64::MAX as u64),
            limits: ImportLimits::default(),
            http: ControlledHttp::default(),
            source: DirectAudioSource,
            active: Arc::new(Mutex::new(HashMap::new())),
            worker_lock: Arc::new(Mutex::new(())),
            wake: Arc::new(Notify::new()),
        }
    }

    /// Server configuration can lower or raise the defaults without changing public URL egress policy.
    pub fn with_limits(mut self, limits: ImportLimits) -> Self {
        self.limits = limits;
        self
    }

    pub async fn preview(&self, input: &str) -> Result<ImportPreview> {
        let future = async {
            let url = self.http.validate(input).await?;
            if !self.source.recognizes(&url) {
                return Err(ImportError::Policy("Unsupported source"));
            }
            let assets = self.source.resolve(&self.http, url.clone()).await?;
            let asset = assets
                .into_iter()
                .next()
                .ok_or(ImportError::Policy("Source has no downloadable assets"))?;
            self.check_size(asset.size_bytes)?;
            Ok(ImportPreview {
                source: self.source.name().to_string(),
                url: url.to_string(),
                filename: asset.filename,
                content_type: asset.content_type,
                size_bytes: asset.size_bytes.map(|n| n as i64),
            })
        };
        Ok(tokio::time::timeout(Duration::from_secs(30), future)
            .await
            .map_err(|_| ImportError::Transient("Source preview timed out"))??)
    }

    pub async fn enqueue(&self, owner_id: &str, input: &str) -> Result<ImportJob> {
        let url = self.http.validate(input).await?;
        let id = Uuid::new_v4().to_string();
        // A single statement prevents concurrent enqueue requests from exceeding queue limits.
        let row = sqlx::query("INSERT INTO import_jobs(id,owner_id,url,status) SELECT ?,?,?,'queued' WHERE (SELECT count(*) FROM import_jobs WHERE status IN ('queued','downloading','processing')) < ? AND (SELECT count(*) FROM import_jobs WHERE owner_id=? AND status IN ('queued','downloading','processing')) < ? RETURNING *")
            .bind(&id).bind(owner_id).bind(url.as_str()).bind(i64::from(self.limits.max_active_jobs))
            .bind(owner_id).bind(i64::from(self.limits.max_active_jobs_per_owner))
            .fetch_optional(&self.pool).await?;
        let Some(row) = row else {
            bail!("Import queue is full; wait for active jobs to finish");
        };
        self.wake.notify_one();
        job_from_row(&row)
    }

    pub async fn list(&self, owner_id: &str) -> Result<Vec<ImportJob>> {
        sqlx::query(
            "SELECT * FROM import_jobs WHERE owner_id=? ORDER BY created_at DESC,id DESC LIMIT 100",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(job_from_row)
        .collect()
    }

    /// Publication is atomic and cannot be cancelled after the processing stage begins.
    pub async fn cancel(&self, owner_id: &str, job_id: &str) -> Result<bool> {
        let changed = sqlx::query("UPDATE import_jobs SET status='cancelled',error=NULL,updated_at=unixepoch() WHERE id=? AND owner_id=? AND status IN ('queued','downloading')")
            .bind(job_id).bind(owner_id).execute(&self.pool).await?.rows_affected() > 0;
        if changed && let Some(token) = self.active.lock().await.get(job_id) {
            token.cancel();
        }
        self.wake.notify_one();
        Ok(changed)
    }

    pub async fn retry(&self, owner_id: &str, job_id: &str) -> Result<bool> {
        let changed = sqlx::query("UPDATE import_jobs SET status='queued',attempt=0,downloaded_bytes=0,total_bytes=NULL,error=NULL,track_id=NULL,updated_at=unixepoch() WHERE id=? AND owner_id=? AND status IN ('failed','cancelled') AND (SELECT count(*) FROM import_jobs WHERE status IN ('queued','downloading','processing')) < ? AND (SELECT count(*) FROM import_jobs WHERE owner_id=? AND status IN ('queued','downloading','processing')) < ?")
            .bind(job_id).bind(owner_id).bind(i64::from(self.limits.max_active_jobs))
            .bind(owner_id).bind(i64::from(self.limits.max_active_jobs_per_owner))
            .execute(&self.pool).await?.rows_affected() > 0;
        if changed {
            self.wake.notify_one();
        }
        Ok(changed)
    }

    pub async fn run(&self, shutdown: CancellationToken) -> Result<()> {
        let _guard = self
            .worker_lock
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Import worker is already running"))?;
        fs::create_dir_all(&self.staging_dir).await?;
        self.cleanup_staging().await?;
        // Restart resumes from zero. Exact media hashing makes retry after publication idempotent.
        sqlx::query("UPDATE import_jobs SET status='queued',attempt=0,downloaded_bytes=0,total_bytes=NULL,error=NULL,updated_at=unixepoch() WHERE status IN ('downloading','processing')")
            .execute(&self.pool).await?;
        loop {
            if shutdown.is_cancelled() {
                break;
            }
            let Some(job) = self.claim().await? else {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = self.wake.notified() => {},
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }
                continue;
            };
            let cancel = CancellationToken::new();
            self.active
                .lock()
                .await
                .insert(job.job.id.clone(), cancel.clone());
            let path = self
                .staging_dir
                .join(format!("vela-import-{}.part", job.job.id));
            let result = self.execute(&job, &path, &cancel, &shutdown).await;
            // Always remove this worker's partial file, including errors and cancellation.
            let cleanup = fs::remove_file(&path).await;
            let cleanup_failed = cleanup
                .as_ref()
                .is_err_and(|e| e.kind() != std::io::ErrorKind::NotFound);
            if cleanup_failed {
                tracing::warn!(job_id=%job.job.id, "Import staging cleanup failed");
            }
            self.active.lock().await.remove(&job.job.id);
            self.finish(&job, result).await?;
            // Infrastructure failure must not let repeated jobs accumulate abandoned files.
            if cleanup_failed {
                bail!("Import staging cleanup failed");
            }
        }
        Ok(())
    }

    async fn claim(&self) -> Result<Option<ClaimedJob>> {
        // FIFO for fresh jobs; retries get bounded backoff and cannot starve freshly queued work.
        let row = sqlx::query("UPDATE import_jobs SET status='downloading',attempt=attempt+1,downloaded_bytes=0,total_bytes=NULL,error=NULL,updated_at=unixepoch() WHERE id=(SELECT id FROM import_jobs WHERE status='queued' AND (attempt=0 OR updated_at<=unixepoch()-CASE WHEN attempt=1 THEN 2 ELSE 5 END) ORDER BY attempt,created_at,id LIMIT 1) AND status='queued' RETURNING *")
            .fetch_optional(&self.pool).await?;
        row.map(|r| {
            Ok(ClaimedJob {
                job: job_from_row(&r)?,
                owner_id: r.try_get("owner_id")?,
                attempt: r.try_get("attempt")?,
            })
        })
        .transpose()
    }

    async fn execute(
        &self,
        job: &ClaimedJob,
        path: &Path,
        cancel: &CancellationToken,
        shutdown: &CancellationToken,
    ) -> std::result::Result<String, ImportError> {
        let still_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM import_jobs WHERE id=? AND status='downloading')",
        )
        .bind(&job.job.id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ImportError::Database)?;
        if !still_active {
            return Err(ImportError::Cancelled);
        }
        let download = self.download(job, path);
        let filename = tokio::select! {
            biased;
            _ = shutdown.cancelled() => return Err(ImportError::Shutdown),
            _ = cancel.cancelled() => return Err(ImportError::Cancelled),
            result = tokio::time::timeout(self.limits.transfer_timeout, download) => {
                result.map_err(|_| ImportError::Transient("Maximum download duration exceeded"))??
            }
        };
        let changed = sqlx::query("UPDATE import_jobs SET status='processing',updated_at=unixepoch() WHERE id=? AND status='downloading'")
            .bind(&job.job.id).execute(&self.pool).await.map_err(|_| ImportError::Database)?.rows_affected();
        if changed == 0 {
            return Err(ImportError::Cancelled);
        }
        let uploaded = self
            .media
            .ingest(&job.owner_id, path, &filename)
            .await
            .map_err(|_| ImportError::Media)?;
        Ok(uploaded.track.id)
    }

    async fn download(
        &self,
        job: &ClaimedJob,
        path: &Path,
    ) -> std::result::Result<String, ImportError> {
        let url = egress::parse_url(&job.job.url)?;
        let response = self.http.request(url, Method::GET).await?;
        source::check_status(response.status())?;
        let filename = source::filename(response.url());
        let (_, total) = source::inspect_response(&response, &filename)?;
        self.check_size(total)?;
        let expected = total.unwrap_or(self.max_file_bytes);
        // Ingestion publishes via a durable copy: staging and managed audio coexist briefly.
        self.check_disk_budget(expected, expected.saturating_mul(2))
            .await?;
        sqlx::query("UPDATE import_jobs SET total_bytes=?,updated_at=unixepoch() WHERE id=? AND status='downloading'")
            .bind(total.map(|n| n as i64)).bind(&job.job.id).execute(&self.pool).await.map_err(|_| ImportError::Database)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await
            .map_err(|_| ImportError::Storage)?;
        let mut body = response.bytes_stream();
        let mut received = 0_u64;
        let mut persisted_at = Instant::now();
        let mut disk_checked_at = 0_u64;
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(egress::network_error)?;
            received = received
                .checked_add(chunk.len() as u64)
                .ok_or(ImportError::TooLarge)?;
            self.check_size(Some(received))?;
            if received.saturating_sub(disk_checked_at) >= 8 * 1024 * 1024 {
                self.check_disk_budget(received, received.saturating_add(chunk.len() as u64))
                    .await?;
                disk_checked_at = received;
            }
            file.write_all(&chunk)
                .await
                .map_err(|_| ImportError::Storage)?;
            if persisted_at.elapsed() >= Duration::from_millis(500) {
                let changed = self.progress(&job.job.id, received).await?;
                if !changed {
                    return Err(ImportError::Cancelled);
                }
                persisted_at = Instant::now();
            }
        }
        if received == 0 {
            return Err(ImportError::Policy("Remote audio file is empty"));
        }
        if total.is_some_and(|expected| expected != received) {
            return Err(ImportError::Transient("Remote download was incomplete"));
        }
        file.sync_all().await.map_err(|_| ImportError::Storage)?;
        if !self.progress(&job.job.id, received).await? {
            return Err(ImportError::Cancelled);
        }
        self.check_disk_budget(received, received).await?;
        Ok(filename)
    }

    async fn progress(&self, id: &str, bytes: u64) -> std::result::Result<bool, ImportError> {
        Ok(sqlx::query("UPDATE import_jobs SET downloaded_bytes=?,updated_at=unixepoch() WHERE id=? AND status='downloading'")
            .bind(bytes as i64).bind(id).execute(&self.pool).await.map_err(|_| ImportError::Database)?.rows_affected() > 0)
    }

    fn check_size(&self, size: Option<u64>) -> std::result::Result<(), ImportError> {
        if size.is_some_and(|n| n > self.max_file_bytes) {
            return Err(ImportError::TooLarge);
        }
        Ok(())
    }

    async fn check_disk_budget(
        &self,
        media_bytes: u64,
        additional_disk_bytes: u64,
    ) -> std::result::Result<(), ImportError> {
        let stored: i64 =
            sqlx::query_scalar("SELECT COALESCE(sum(size_bytes),0) FROM media_assets")
                .fetch_one(&self.pool)
                .await
                .map_err(|_| ImportError::Database)?;
        if (stored as u64).saturating_add(media_bytes) > self.limits.max_storage_bytes {
            return Err(ImportError::StorageQuota);
        }
        let dir = self.staging_dir.clone();
        let free = tokio::task::spawn_blocking(move || fs2::available_space(dir))
            .await
            .map_err(|_| ImportError::Storage)?
            .map_err(|_| ImportError::Storage)?;
        if free < additional_disk_bytes.saturating_add(self.limits.reserve_free_bytes) {
            return Err(ImportError::StorageQuota);
        }
        Ok(())
    }

    async fn finish(
        &self,
        job: &ClaimedJob,
        result: std::result::Result<String, ImportError>,
    ) -> Result<()> {
        match result {
            Ok(track_id) => {
                sqlx::query("UPDATE import_jobs SET status='completed',track_id=?,error=NULL,updated_at=unixepoch() WHERE id=? AND status='processing'")
                    .bind(track_id).bind(&job.job.id).execute(&self.pool).await?;
                tracing::info!(job_id=%job.job.id, "Import completed");
            }
            Err(ImportError::Cancelled) => {}
            Err(ImportError::Shutdown) => {
                sqlx::query("UPDATE import_jobs SET status='queued',attempt=max(attempt-1,0),downloaded_bytes=0,total_bytes=NULL,error=NULL,updated_at=unixepoch() WHERE id=? AND status='downloading'")
                    .bind(&job.job.id).execute(&self.pool).await?;
            }
            Err(error) => {
                let retryable = matches!(error, ImportError::Transient(_)) && job.attempt < 3;
                let status = if retryable { "queued" } else { "failed" };
                sqlx::query("UPDATE import_jobs SET status=?,error=?,updated_at=unixepoch() WHERE id=? AND status IN ('downloading','processing')")
                    .bind(status).bind(error.to_string()).bind(&job.job.id).execute(&self.pool).await?;
                tracing::warn!(job_id=%job.job.id, retryable, error=%error, "Import attempt failed");
            }
        }
        Ok(())
    }

    async fn cleanup_staging(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.staging_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if let Some(id) = name
                .strip_prefix("vela-import-")
                .and_then(|name| name.strip_suffix(".part"))
                && Uuid::parse_str(id).is_ok()
                && entry.file_type().await?.is_file()
            {
                fs::remove_file(entry.path()).await?;
            }
        }
        Ok(())
    }
}

fn job_from_row(row: &SqliteRow) -> Result<ImportJob> {
    Ok(ImportJob {
        id: row.try_get("id")?,
        url: row.try_get("url")?,
        status: row.try_get("status")?,
        downloaded_bytes: row.try_get("downloaded_bytes")?,
        total_bytes: row.try_get("total_bytes")?,
        error: row.try_get("error")?,
        track_id: row.try_get("track_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[cfg(test)]
mod tests;
