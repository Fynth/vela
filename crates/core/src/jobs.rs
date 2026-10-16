//! A bounded, owner-private read model for independently executed background jobs.
use crate::{Error, Result};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool, Transaction, sqlite::SqliteRow};
use vela_domain::{
    BackgroundJob, JobKind, JobPage, JobProgress, JobQuery, JobStateFilter, JobStatus, User,
};

const INVALID_CURSOR: &str = "Invalid task cursor";
const IMPORT_FAILURE: &str = "Audio import failed. Review its details or retry.";
const SCAN_FAILURE: &str = "Folder scan failed. Review the music source and try again.";

#[derive(Clone)]
pub struct JobService {
    pool: SqlitePool,
}

impl JobService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, user: &User, query: &JobQuery) -> Result<JobPage> {
        if query.kind == Some(JobKind::Scan) && !user.is_admin {
            return Err(Error::Forbidden);
        }
        let cursor = query.cursor.as_deref().map(Cursor::parse).transpose()?;
        let limit = query.limit.unwrap_or(50).clamp(1, 100) as usize;
        // Deferred reads share one WAL snapshot, including joined action availability
        // and activity outside the selected filter/page. No worker locks are needed.
        let mut transaction = self.pool.begin().await?;
        let mut jobs = Vec::with_capacity(2 * (limit + 1));
        if query.kind != Some(JobKind::Scan) {
            jobs.extend(
                import_rows(&mut transaction, user, query.state, cursor.as_ref(), limit).await?,
            );
        }
        if user.is_admin && query.kind != Some(JobKind::Import) {
            jobs.extend(
                scan_rows(&mut transaction, user, query.state, cursor.as_ref(), limit).await?,
            );
        }
        let has_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM import_jobs WHERE owner_id=? AND status IN ('queued','downloading','processing')) OR (? AND EXISTS(SELECT 1 FROM scan_jobs WHERE owner_id=? AND status IN ('queued','running')))",
        )
        .bind(&user.id)
        .bind(user.is_admin)
        .bind(&user.id)
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        jobs.sort_unstable_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
                .then_with(|| kind_rank(right.kind).cmp(&kind_rank(left.kind)))
        });
        let has_more = jobs.len() > limit;
        jobs.truncate(limit);
        let next_cursor = if has_more {
            jobs.last().map(Cursor::encode)
        } else {
            None
        };
        Ok(JobPage {
            items: jobs,
            next_cursor,
            has_active,
        })
    }
}

struct Cursor {
    created_at: i64,
    id: String,
    kind: JobKind,
}

impl Cursor {
    fn parse(value: &str) -> Result<Self> {
        let invalid = || Error::Invalid(INVALID_CURSOR.into());
        if value.len() > 128 {
            return Err(invalid());
        }
        let mut fields = value.split(':');
        if fields.next() != Some("v1") {
            return Err(invalid());
        }
        let timestamp = fields.next().ok_or_else(invalid)?;
        let created_at = timestamp.parse::<i64>().map_err(|_| invalid())?;
        if created_at < 0 || timestamp != created_at.to_string() {
            return Err(invalid());
        }
        let id = fields.next().ok_or_else(invalid)?;
        let uuid = uuid::Uuid::parse_str(id).map_err(|_| invalid())?;
        if id != uuid.hyphenated().to_string() {
            return Err(invalid());
        }
        let kind = match fields.next() {
            Some("import") => JobKind::Import,
            Some("scan") => JobKind::Scan,
            _ => return Err(invalid()),
        };
        if fields.next().is_some() {
            return Err(invalid());
        }
        Ok(Self {
            created_at,
            id: id.to_owned(),
            kind,
        })
    }

    fn encode(job: &BackgroundJob) -> String {
        format!(
            "v1:{}:{}:{}",
            job.created_at,
            job.id,
            match job.kind {
                JobKind::Import => "import",
                JobKind::Scan => "scan",
            }
        )
    }
}

fn kind_rank(kind: JobKind) -> u8 {
    match kind {
        JobKind::Import => 0,
        JobKind::Scan => 1,
    }
}

fn seek_and_limit<'a>(
    query: &mut QueryBuilder<'a, Sqlite>,
    kind: JobKind,
    cursor: Option<&'a Cursor>,
    limit: usize,
) {
    if let Some(cursor) = cursor {
        // The per-table tuple matches its owner/history index. Include an exact
        // tuple only when this branch sorts after the cursor's kind globally.
        query.push(if kind_rank(kind) < kind_rank(cursor.kind) {
            " AND (j.created_at,j.id) <= ("
        } else {
            " AND (j.created_at,j.id) < ("
        });
        query
            .push_bind(cursor.created_at)
            .push(",")
            .push_bind(&cursor.id)
            .push(")");
    }
    query
        .push(" ORDER BY j.created_at DESC,j.id DESC LIMIT ")
        .push_bind((limit + 1) as i64);
}

fn filter_state(query: &mut QueryBuilder<'_, Sqlite>, kind: JobKind, state: JobStateFilter) {
    query.push(match state {
        JobStateFilter::All => "",
        JobStateFilter::Active if kind == JobKind::Import => {
            " AND j.status IN ('queued','downloading','processing')"
        }
        JobStateFilter::Active => " AND j.status IN ('queued','running')",
        JobStateFilter::Failed => " AND j.status='failed'",
        JobStateFilter::Completed => " AND j.status='completed'",
        JobStateFilter::Cancelled => " AND j.status='cancelled'",
    });
}

async fn import_rows(
    transaction: &mut Transaction<'_, Sqlite>,
    user: &User,
    state: JobStateFilter,
    cursor: Option<&Cursor>,
    limit: usize,
) -> Result<Vec<BackgroundJob>> {
    // Never select the URL or worker diagnostics into this read model. Even a
    // corrupt foreign track reference cannot disclose its title or identifier.
    let mut query = QueryBuilder::new(
        "SELECT j.id,j.status,j.downloaded_bytes,j.total_bytes,j.created_at,j.updated_at,t.id AS visible_track_id,COALESCE(t.title,'Audio import') AS title FROM import_jobs j LEFT JOIN tracks t ON t.id=j.track_id AND t.owner_id=j.owner_id AND j.status='completed' WHERE j.owner_id=",
    );
    query.push_bind(&user.id);
    filter_state(&mut query, JobKind::Import, state);
    seek_and_limit(&mut query, JobKind::Import, cursor, limit);
    query
        .build()
        .fetch_all(&mut **transaction)
        .await?
        .iter()
        .map(import_job)
        .collect()
}

fn import_job(row: &SqliteRow) -> Result<BackgroundJob> {
    let status = match row.try_get::<&str, _>("status")? {
        "queued" => JobStatus::Queued,
        "downloading" => JobStatus::Downloading,
        "processing" => JobStatus::Processing,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        "cancelled" => JobStatus::Cancelled,
        _ => return Err(Error::Internal(anyhow::anyhow!("Invalid import job state"))),
    };
    Ok(BackgroundJob {
        id: row.try_get("id")?,
        kind: JobKind::Import,
        title: row.try_get("title")?,
        status,
        progress: JobProgress::Import {
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            total_bytes: row.try_get("total_bytes")?,
        },
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        error: (status == JobStatus::Failed).then(|| IMPORT_FAILURE.into()),
        source_id: None,
        track_id: row.try_get("visible_track_id")?,
        can_cancel: matches!(status, JobStatus::Queued | JobStatus::Downloading),
        can_retry: matches!(status, JobStatus::Failed | JobStatus::Cancelled),
        can_rescan: false,
    })
}

async fn scan_rows(
    transaction: &mut Transaction<'_, Sqlite>,
    user: &User,
    state: JobStateFilter,
    cursor: Option<&Cursor>,
    limit: usize,
) -> Result<Vec<BackgroundJob>> {
    let mut query = QueryBuilder::new(
        "SELECT j.id,j.status,j.cancel_requested,j.visited,j.imported,j.duplicates,j.unchanged,j.skipped,j.failed_files,j.created_at,j.updated_at,s.id AS visible_source_id,s.name AS title,NOT EXISTS(SELECT 1 FROM scan_jobs active WHERE active.owner_id=j.owner_id AND active.source_id=s.id AND active.status IN ('queued','running')) AS source_idle FROM scan_jobs j JOIN scan_sources s ON s.id=j.source_id AND s.owner_id=j.owner_id WHERE j.owner_id=",
    );
    query.push_bind(&user.id);
    filter_state(&mut query, JobKind::Scan, state);
    seek_and_limit(&mut query, JobKind::Scan, cursor, limit);
    query
        .build()
        .fetch_all(&mut **transaction)
        .await?
        .iter()
        .map(scan_job)
        .collect()
}

fn scan_job(row: &SqliteRow) -> Result<BackgroundJob> {
    let pending: bool = row.try_get("cancel_requested")?;
    let status = match row.try_get::<&str, _>("status")? {
        "queued" => JobStatus::Queued,
        "running" if pending => JobStatus::Cancelling,
        "running" => JobStatus::Scanning,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        "cancelled" => JobStatus::Cancelled,
        _ => {
            return Err(Error::Internal(anyhow::anyhow!(
                "Invalid folder scan state"
            )));
        }
    };
    let terminal = matches!(
        status,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    );
    Ok(BackgroundJob {
        id: row.try_get("id")?,
        kind: JobKind::Scan,
        title: row.try_get("title")?,
        status,
        progress: JobProgress::Scan {
            visited: row.try_get("visited")?,
            imported: row.try_get("imported")?,
            duplicates: row.try_get("duplicates")?,
            unchanged: row.try_get("unchanged")?,
            skipped: row.try_get("skipped")?,
            failed_files: row.try_get("failed_files")?,
        },
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        error: (status == JobStatus::Failed).then(|| SCAN_FAILURE.into()),
        source_id: row.try_get("visible_source_id")?,
        track_id: None,
        can_cancel: matches!(status, JobStatus::Queued | JobStatus::Scanning) && !pending,
        can_retry: false,
        can_rescan: terminal && row.try_get("source_idle")?,
    })
}
