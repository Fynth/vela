//! Query fixtures use real migrated SQLite; worker/media execution is exercised
//! separately by the server, imports and scanner integration suites.
use sqlx::{Row, SqlitePool};
use vela_core::{Error, database, jobs::JobService};
use vela_domain::{JobKind, JobProgress, JobQuery, JobStateFilter, JobStatus, User};

struct Fixture {
    pool: SqlitePool,
    service: JobService,
    admin: User,
    listener: User,
    other: User,
    _directory: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::open(directory.path()).await.unwrap();
        let admin = user("owner-admin", true);
        let listener = user("owner-listener", false);
        let other = user("owner-other", true);
        for account in [&admin, &listener, &other] {
            sqlx::query(
                "INSERT INTO users(id,username,password_hash,is_admin) VALUES(?,?,'fixture',?)",
            )
            .bind(&account.id)
            .bind(&account.username)
            .bind(account.is_admin)
            .execute(&pool)
            .await
            .unwrap();
        }
        Self {
            service: JobService::new(pool.clone()),
            pool,
            admin,
            listener,
            other,
            _directory: directory,
        }
    }

    async fn source(&self, owner: &User, id: u128) -> String {
        let id = uuid(id);
        sqlx::query(
            "INSERT INTO scan_sources(id,owner_id,name,root_id) VALUES(?,?,'Family collection',?)",
        )
        .bind(&id)
        .bind(&owner.id)
        .bind(&id)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    async fn import(&self, owner: &User, id: u128, status: &str, time: i64) -> String {
        let id = uuid(id);
        sqlx::query("INSERT INTO import_jobs(id,owner_id,url,status,downloaded_bytes,total_bytes,error,created_at,updated_at) VALUES(?,?,'https://example.org/private.flac?token=SECRET-QUERY',?,42,99,'/private/music SECRET-DIAGNOSTIC',?,?)")
            .bind(&id)
            .bind(&owner.id)
            .bind(status)
            .bind(time)
            .bind(time + 1)
            .execute(&self.pool)
            .await
            .unwrap();
        id
    }

    async fn scan(&self, owner: &User, id: u128, source: &str, status: &str, time: i64) -> String {
        let id = uuid(id);
        sqlx::query("INSERT INTO scan_jobs(id,owner_id,source_id,status,visited,imported,duplicates,unchanged,skipped,failed_files,error,created_at,updated_at) VALUES(?,?,?,?,19,2,3,4,5,5,'/private/source SECRET-DIAGNOSTIC',?,?)")
            .bind(&id)
            .bind(&owner.id)
            .bind(source)
            .bind(status)
            .bind(time)
            .bind(time + 1)
            .execute(&self.pool)
            .await
            .unwrap();
        id
    }

    async fn track(&self, owner: &User, id: u128, title: &str) -> String {
        let id = uuid(id);
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,'media/private-fixture.audio','audio/flac',99)")
            .bind(&id).bind(&id).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,duration_ms,codec) VALUES(?,?,?,?,1000,'FLAC')")
            .bind(&id).bind(&owner.id).bind(&id).bind(title).execute(&self.pool).await.unwrap();
        id
    }

    async fn link_track(&self, job: &str, track: &str) {
        sqlx::query("UPDATE import_jobs SET track_id=? WHERE id=?")
            .bind(track)
            .bind(job)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn close(self) {
        self.pool.close().await;
    }
}

fn user(id: &str, is_admin: bool) -> User {
    User {
        id: id.into(),
        username: id.into(),
        is_admin,
    }
}

fn uuid(id: u128) -> String {
    uuid::Uuid::from_u128(id).to_string()
}

#[tokio::test]
async fn mixed_histories_page_each_collision_once_in_global_order() {
    let fixture = Fixture::new().await;
    let source = fixture.source(&fixture.admin, 1000).await;
    for (id, time) in [(20, 50), (10, 50), (99, 49), (30, 51)] {
        fixture.import(&fixture.admin, id, "completed", time).await;
    }
    for (id, time) in [(20, 50), (10, 50), (1, 49)] {
        fixture
            .scan(&fixture.admin, id, &source, "completed", time)
            .await;
    }
    fixture.import(&fixture.other, 100, "completed", 999).await;
    let foreign_source = fixture.source(&fixture.other, 1001).await;
    fixture
        .scan(&fixture.other, 101, &foreign_source, "completed", 999)
        .await;
    let expected = vec![
        (JobKind::Import, uuid(30)),
        (JobKind::Scan, uuid(20)),
        (JobKind::Import, uuid(20)),
        (JobKind::Scan, uuid(10)),
        (JobKind::Import, uuid(10)),
        (JobKind::Import, uuid(99)),
        (JobKind::Scan, uuid(1)),
    ];
    for limit in [1, 2, 3, 100] {
        let mut query = JobQuery {
            limit: Some(limit),
            ..JobQuery::default()
        };
        let mut observed = Vec::new();
        loop {
            let page = fixture.service.list(&fixture.admin, &query).await.unwrap();
            assert!(!page.has_active);
            assert!(page.items.len() <= limit as usize);
            observed.extend(page.items.into_iter().map(|job| (job.kind, job.id)));
            query.cursor = page.next_cursor;
            if query.cursor.is_none() {
                break;
            }
            assert!(
                observed.len() < 20,
                "pagination must advance across identity ties"
            );
        }
        assert_eq!(observed, expected);
    }
    // A cursor is only a seek key; it does not require a corresponding row
    // and does not grant access to another owner's job.
    let page = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                cursor: Some(format!("v1:51:{}:scan", uuid(30))),
                limit: Some(2),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(page.items[0].id, uuid(30));
    assert_eq!(page.items[0].kind, JobKind::Import);
    let anchor = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                limit: Some(1),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap()
        .next_cursor;
    sqlx::query("DELETE FROM import_jobs WHERE id=?")
        .bind(uuid(30))
        .execute(&fixture.pool)
        .await
        .unwrap();
    let after_deletion = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                cursor: anchor,
                limit: Some(1),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (after_deletion.items[0].kind, &after_deletion.items[0].id),
        (JobKind::Scan, &uuid(20))
    );
    fixture.close().await;
}

#[tokio::test]
async fn owner_and_role_scopes_redact_urls_diagnostics_and_foreign_track_references() {
    let fixture = Fixture::new().await;
    let visible_track = fixture
        .track(&fixture.listener, 500, "Owned recording")
        .await;
    let private_track = fixture
        .track(&fixture.other, 501, "Other owner private title")
        .await;
    let completed = fixture.import(&fixture.listener, 1, "completed", 4).await;
    fixture.link_track(&completed, &visible_track).await;
    let corrupt_reference = fixture.import(&fixture.listener, 2, "completed", 3).await;
    fixture.link_track(&corrupt_reference, &private_track).await;
    let failed = fixture.import(&fixture.listener, 3, "failed", 2).await;
    fixture.link_track(&failed, &visible_track).await;
    fixture.import(&fixture.other, 4, "downloading", 100).await;
    let source = fixture.source(&fixture.listener, 510).await;
    fixture
        .scan(&fixture.listener, 5, &source, "running", 100)
        .await;
    let page = fixture
        .service
        .list(&fixture.listener, &JobQuery::default())
        .await
        .unwrap();
    assert_eq!(page.items.len(), 3);
    assert!(
        !page.has_active,
        "a regular account cannot observe scan activity"
    );
    assert_eq!(page.items[0].title, "Owned recording");
    assert_eq!(
        page.items[0].track_id.as_deref(),
        Some(visible_track.as_str())
    );
    assert_eq!(page.items[1].title, "Audio import");
    assert!(page.items[1].track_id.is_none());
    assert_eq!(page.items[2].title, "Audio import");
    assert!(page.items[2].track_id.is_none());
    assert!(page.items.iter().all(|job| job.source_id.is_none()));
    let json = serde_json::to_string(&page).unwrap();
    for hidden in [
        "https://",
        "SECRET",
        "/private/",
        "Other owner",
        "owner_id",
        "storage_path",
        &private_track,
    ] {
        assert!(!json.contains(hidden), "read model exposed {hidden}");
    }
    assert!(matches!(
        fixture
            .service
            .list(
                &fixture.listener,
                &JobQuery {
                    kind: Some(JobKind::Scan),
                    ..JobQuery::default()
                }
            )
            .await,
        Err(Error::Forbidden)
    ));
    // No administrator can inspect someone else's work through this service.
    assert!(
        fixture
            .service
            .list(&fixture.admin, &JobQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    sqlx::query("DELETE FROM tracks WHERE id=?")
        .bind(&visible_track)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let page = fixture
        .service
        .list(&fixture.listener, &JobQuery::default())
        .await
        .unwrap();
    assert_eq!(page.items[0].title, "Audio import");
    assert!(page.items[0].track_id.is_none());
    fixture.close().await;
}

#[tokio::test]
async fn status_filters_preserve_real_progress_and_activity_outside_page_and_kind() {
    let fixture = Fixture::new().await;
    for (id, status) in [
        "queued",
        "downloading",
        "processing",
        "completed",
        "failed",
        "cancelled",
    ]
    .into_iter()
    .enumerate()
    {
        fixture
            .import(&fixture.admin, id as u128 + 1, status, id as i64)
            .await;
    }
    for (id, status) in ["queued", "running", "completed", "failed", "cancelled"]
        .into_iter()
        .enumerate()
    {
        let source = fixture.source(&fixture.admin, 100 + id as u128).await;
        fixture
            .scan(&fixture.admin, id as u128 + 20, &source, status, id as i64)
            .await;
    }
    let source = fixture.source(&fixture.admin, 110).await;
    let cancelling = fixture
        .scan(&fixture.admin, 30, &source, "running", 0)
        .await;
    sqlx::query("UPDATE scan_jobs SET cancel_requested=1 WHERE id=?")
        .bind(&cancelling)
        .execute(&fixture.pool)
        .await
        .unwrap();
    for (state, count) in [
        (JobStateFilter::All, 12),
        (JobStateFilter::Active, 6),
        (JobStateFilter::Completed, 2),
        (JobStateFilter::Failed, 2),
        (JobStateFilter::Cancelled, 2),
    ] {
        let page = fixture
            .service
            .list(
                &fixture.admin,
                &JobQuery {
                    state,
                    ..JobQuery::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(page.items.len(), count);
        assert!(page.has_active);
        for job in &page.items {
            match &job.progress {
                JobProgress::Import {
                    downloaded_bytes,
                    total_bytes,
                } => assert_eq!((*downloaded_bytes, *total_bytes), (42, Some(99))),
                JobProgress::Scan {
                    visited,
                    imported,
                    duplicates,
                    unchanged,
                    skipped,
                    failed_files,
                } => assert_eq!(
                    (
                        *visited,
                        *imported,
                        *duplicates,
                        *unchanged,
                        *skipped,
                        *failed_files
                    ),
                    (19, 2, 3, 4, 5, 5)
                ),
            }
        }
    }
    let filtered = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                kind: Some(JobKind::Scan),
                state: JobStateFilter::Completed,
                limit: Some(1),
                cursor: Some(format!("v1:99:{}:scan", uuid(999))),
            },
        )
        .await
        .unwrap();
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].status, JobStatus::Completed);
    assert!(filtered.has_active);
    let empty = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                kind: Some(JobKind::Import),
                state: JobStateFilter::Completed,
                cursor: Some(format!("v1:0:{}:import", uuid(0))),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(
        empty.has_active,
        "polling must continue when filtered/older page is empty"
    );
    // Kind selection must not suppress activity belonging to the other kind.
    sqlx::query("UPDATE import_jobs SET status='completed' WHERE owner_id=? AND status IN ('queued','downloading','processing')").bind(&fixture.admin.id).execute(&fixture.pool).await.unwrap();
    let imports_only = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                kind: Some(JobKind::Import),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap();
    assert!(
        imports_only.has_active,
        "visible scans keep import-only view polling"
    );
    sqlx::query("UPDATE scan_jobs SET status='completed' WHERE owner_id=? AND status IN ('queued','running')").bind(&fixture.admin.id).execute(&fixture.pool).await.unwrap();
    sqlx::query("UPDATE import_jobs SET status='queued' WHERE id=?")
        .bind(uuid(1))
        .execute(&fixture.pool)
        .await
        .unwrap();
    let scans_only = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                kind: Some(JobKind::Scan),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap();
    assert!(
        scans_only.has_active,
        "visible imports keep scan-only view polling"
    );
    sqlx::query("UPDATE import_jobs SET status='completed' WHERE id=?")
        .bind(uuid(1))
        .execute(&fixture.pool)
        .await
        .unwrap();
    fixture.import(&fixture.other, 999, "queued", 999).await;
    assert!(
        !fixture
            .service
            .list(&fixture.admin, &JobQuery::default())
            .await
            .unwrap()
            .has_active
    );
    fixture.close().await;
}

#[tokio::test]
async fn snapshot_actions_follow_state_changes_and_source_activity_without_rewriting_history() {
    let fixture = Fixture::new().await;
    for (id, status) in [
        "queued",
        "downloading",
        "processing",
        "completed",
        "failed",
        "cancelled",
    ]
    .into_iter()
    .enumerate()
    {
        fixture
            .import(&fixture.admin, id as u128 + 1, status, 1)
            .await;
    }
    let source = fixture.source(&fixture.admin, 100).await;
    let completed = fixture
        .scan(&fixture.admin, 20, &source, "completed", 1)
        .await;
    let failed = fixture.scan(&fixture.admin, 21, &source, "failed", 1).await;
    let cancelled = fixture
        .scan(&fixture.admin, 22, &source, "cancelled", 1)
        .await;
    let page = fixture
        .service
        .list(&fixture.admin, &JobQuery::default())
        .await
        .unwrap();
    for job in &page.items {
        match job.kind {
            JobKind::Import => {
                assert_eq!(
                    job.can_cancel,
                    matches!(job.status, JobStatus::Queued | JobStatus::Downloading)
                );
                assert_eq!(
                    job.can_retry,
                    matches!(job.status, JobStatus::Failed | JobStatus::Cancelled)
                );
                assert!(!job.can_rescan);
            }
            JobKind::Scan => {
                assert!(job.can_rescan);
                assert!(!job.can_cancel);
                assert!(!job.can_retry);
            }
        }
    }
    let running = fixture
        .scan(&fixture.admin, 23, &source, "running", 2)
        .await;
    let page = fixture
        .service
        .list(&fixture.admin, &JobQuery::default())
        .await
        .unwrap();
    assert!(page.items.iter().all(|job| !job.can_rescan));
    let active = page.items.iter().find(|job| job.id == running).unwrap();
    assert_eq!(active.status, JobStatus::Scanning);
    assert!(active.can_cancel);
    sqlx::query("UPDATE scan_jobs SET cancel_requested=1,updated_at=7 WHERE id=?")
        .bind(&running)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let page = fixture
        .service
        .list(&fixture.admin, &JobQuery::default())
        .await
        .unwrap();
    let active = page.items.iter().find(|job| job.id == running).unwrap();
    assert_eq!(active.status, JobStatus::Cancelling);
    assert_eq!(active.updated_at, 7);
    assert!(!active.can_cancel);
    sqlx::query("UPDATE scan_jobs SET status='cancelled',updated_at=8 WHERE id=?")
        .bind(&running)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let page = fixture
        .service
        .list(&fixture.admin, &JobQuery::default())
        .await
        .unwrap();
    assert!(
        page.items
            .iter()
            .filter(|job| job.kind == JobKind::Scan)
            .all(|job| job.can_rescan)
    );
    for id in [completed, failed, cancelled] {
        assert_eq!(
            page.items
                .iter()
                .find(|job| job.id == id)
                .unwrap()
                .created_at,
            1
        );
    }
    sqlx::query("DELETE FROM scan_sources WHERE id=?")
        .bind(&source)
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert!(
        fixture
            .service
            .list(&fixture.admin, &JobQuery::default())
            .await
            .unwrap()
            .items
            .iter()
            .all(|job| job.kind == JobKind::Import)
    );
    fixture.close().await;
}

#[tokio::test]
async fn cursor_validation_and_limits_are_bounded_even_for_mixed_large_histories() {
    let fixture = Fixture::new().await;
    let source = fixture.source(&fixture.admin, 1000).await;
    let mut transaction = fixture.pool.begin().await.unwrap();
    for id in 1..=110_u128 {
        sqlx::query("INSERT INTO import_jobs(id,owner_id,url,status,created_at) VALUES(?,?,'https://example.org/a.flac','completed',10)")
            .bind(uuid(id)).bind(&fixture.admin.id).execute(&mut *transaction).await.unwrap();
        sqlx::query("INSERT INTO scan_jobs(id,owner_id,source_id,status,created_at) VALUES(?,?,?,'completed',10)")
            .bind(uuid(id)).bind(&fixture.admin.id).bind(&source).execute(&mut *transaction).await.unwrap();
    }
    transaction.commit().await.unwrap();
    for (limit, expected) in [
        (None, 50),
        (Some(0), 1),
        (Some(1), 1),
        (Some(100), 100),
        (Some(u32::MAX), 100),
    ] {
        let page = fixture
            .service
            .list(
                &fixture.admin,
                &JobQuery {
                    limit,
                    ..JobQuery::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(page.items.len(), expected);
        assert!(page.next_cursor.is_some());
    }
    for cursor in [
        String::new(),
        "v2:10:invalid:import".into(),
        "x".repeat(129),
        format!("v1:-1:{}:import", uuid(1)),
        format!("v1:+1:{}:import", uuid(1)),
        format!("v1:01:{}:import", uuid(1)),
        format!("v1:9223372036854775808:{}:import", uuid(1)),
        format!("v1:1:{}:other", uuid(1)),
        format!("v1:1:{}:import:extra", uuid(1)),
        "v1:1:00000000000000000000000000000001:import".into(),
        "v1:1:AAAAAAAA-0000-0000-0000-000000000001:scan".into(),
        "v1:1:' OR 1=1--:import".into(),
    ] {
        let result = fixture
            .service
            .list(
                &fixture.admin,
                &JobQuery {
                    cursor: Some(cursor),
                    ..JobQuery::default()
                },
            )
            .await;
        assert!(
            matches!(result, Err(Error::Invalid(ref message)) if message == "Invalid task cursor")
        );
    }
    let page = fixture
        .service
        .list(
            &fixture.admin,
            &JobQuery {
                cursor: Some(format!("v1:9223372036854775807:{}:scan", uuid(1))),
                ..JobQuery::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(page.items.len(), 50);
    fixture.close().await;
}

#[tokio::test]
async fn migrated_history_queries_seek_owner_indexes_without_full_history_sort() {
    let fixture = Fixture::new().await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=7 AND success=1"
        )
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
    for table in ["import_jobs", "scan_jobs"] {
        for state in ["", " AND j.status='failed'"] {
            let query = format!(
                "EXPLAIN QUERY PLAN SELECT j.id FROM {table} j WHERE j.owner_id=?{state} AND (j.created_at,j.id)<(?,?) ORDER BY j.created_at DESC,j.id DESC LIMIT 51"
            );
            let rows = sqlx::query(&query)
                .bind(&fixture.admin.id)
                .bind(10_i64)
                .bind(uuid(1))
                .fetch_all(&fixture.pool)
                .await
                .unwrap();
            let details = rows
                .iter()
                .map(|row| row.get::<String, _>("detail"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                details.contains(&format!("{table}_owner")),
                "missing owner/history index: {details}"
            );
            assert!(
                !details.contains("TEMP B-TREE"),
                "history seek must avoid an unbounded sort: {details}"
            );
        }
        let rows = sqlx::query(&format!("PRAGMA index_xinfo({table}_owner)"))
            .fetch_all(&fixture.pool)
            .await
            .unwrap();
        for name in ["created_at", "id"] {
            assert!(
                rows.iter()
                    .any(
                        |row| row.get::<Option<String>, _>("name").as_deref() == Some(name)
                            && row.get::<i64, _>("desc") == 1
                    )
            );
        }
    }
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_worker_commits_cannot_split_rows_from_activity_snapshot() {
    let fixture = Fixture::new().await;
    let job = fixture.import(&fixture.listener, 1, "queued", 1).await;
    let writer_pool = fixture.pool.clone();
    let writer = tokio::spawn(async move {
        for iteration in 0..150 {
            sqlx::query("UPDATE import_jobs SET status=?,updated_at=? WHERE id=?")
                .bind(if iteration % 2 == 0 {
                    "completed"
                } else {
                    "queued"
                })
                .bind(iteration)
                .bind(&job)
                .execute(&writer_pool)
                .await
                .unwrap();
            tokio::task::yield_now().await;
        }
    });
    for _ in 0..150 {
        let page = fixture
            .service
            .list(&fixture.listener, &JobQuery::default())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.has_active, page.items[0].status == JobStatus::Queued);
    }
    writer.await.unwrap();
    fixture.close().await;
}
