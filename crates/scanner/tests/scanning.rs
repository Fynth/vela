#![cfg(target_os = "linux")]
//! Real disk/SQLite/Lofty tests. No scanner or ingestion mocks.
use std::{path::PathBuf, time::Duration};

use sqlx::SqlitePool;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use vela_domain::{CreateScanSource, ScanJob, UpdateTrackMetadata};
use vela_media::MediaService;
use vela_scanner::{RootConfig, ScanError, ScanLimits, Scanner};

const FLAC: &[u8] = include_bytes!("../../media/tests/fixtures/signal.flac");
const MP3: &[u8] = include_bytes!("../../media/tests/fixtures/beacon.mp3");

struct Fixture {
    directory: TempDir,
    root: PathBuf,
    pool: SqlitePool,
    media: MediaService,
    scanner: Scanner,
}

impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("sources");
        tokio::fs::create_dir(&root).await.unwrap();
        let pool = vela_core::database::open(directory.path()).await.unwrap();
        for owner in ["alice", "bob"] {
            sqlx::query("INSERT INTO users(id,username,password_hash,is_admin) VALUES(?,?,?,1)")
                .bind(owner)
                .bind(owner)
                .bind("test only hash")
                .execute(&pool)
                .await
                .unwrap();
        }
        let media = MediaService::new(pool.clone(), directory.path().to_owned(), 1024 * 1024);
        media.initialize().await.unwrap();
        let scanner = Scanner::new(
            pool.clone(),
            media.clone(),
            directory.path().to_owned(),
            1024 * 1024,
            vec![RootConfig {
                id: "music".into(),
                name: "Music folder".into(),
                path: root.clone(),
            }],
        );
        scanner.initialize().await.unwrap();
        Self {
            directory,
            root,
            pool,
            media,
            scanner,
        }
    }

    async fn source(&self, owner: &str) -> String {
        self.scanner
            .create_source(
                owner,
                CreateScanSource {
                    name: "My music".into(),
                    root_id: "music".into(),
                },
            )
            .await
            .unwrap()
            .id
    }

    async fn scan(&self, owner: &str, source: &str) -> ScanJob {
        let job = self.scanner.enqueue(owner, source).await.unwrap();
        finish_job(&self.scanner, owner, &job.id).await
    }
}

async fn finish_job(scanner: &Scanner, owner: &str, id: &str) -> ScanJob {
    let shutdown = CancellationToken::new();
    let worker = {
        let scanner = scanner.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { scanner.run(shutdown).await })
    };
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let job = scanner
                .jobs(owner)
                .await
                .unwrap()
                .into_iter()
                .find(|job| job.id == id)
                .unwrap();
            if !matches!(job.status.as_str(), "queued" | "running") {
                break job;
            }
            assert!(!worker.is_finished(), "scanner worker stopped unexpectedly");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    shutdown.cancel();
    worker.await.unwrap().unwrap();
    let diagnostic = scanner
        .jobs(owner)
        .await
        .unwrap()
        .into_iter()
        .find(|job| job.id == id);
    result.unwrap_or_else(|_| {
        panic!("real scanner exceeded its test deadline; durable checkpoint: {diagnostic:?}")
    })
}

#[tokio::test]
async fn nested_audio_snapshot_rescan_preserves_catalog_edits_and_source_bytes() {
    let fixture = Fixture::new().await;
    let nested = fixture.root.join("Album/Disc 1");
    tokio::fs::create_dir_all(&nested).await.unwrap();
    let flac = nested.join("signal.FLAC");
    let mp3 = fixture.root.join("beacon.mp3");
    tokio::fs::write(&flac, FLAC).await.unwrap();
    tokio::fs::write(&mp3, MP3).await.unwrap();
    tokio::fs::write(fixture.root.join("notes.txt"), b"not audio")
        .await
        .unwrap();
    let source = fixture.source("alice").await;
    let job = fixture.scan("alice", &source).await;
    assert_eq!(job.status, "completed");
    assert_eq!(job.imported, 2);
    assert_eq!(job.failed_files, 0);
    assert_eq!(job.skipped, 1);
    assert_eq!(job.visited, 5);
    let tracks = fixture
        .media
        .list_tracks("alice", None, None, 100, false)
        .await
        .unwrap()
        .items;
    assert_eq!(tracks.len(), 2);
    let signal = tracks.iter().find(|track| track.title == "Signal").unwrap();
    let snapshot = fixture
        .media
        .metadata("alice", &signal.id)
        .await
        .unwrap()
        .unwrap();
    fixture
        .media
        .update_metadata(
            "alice",
            &signal.id,
            UpdateTrackMetadata {
                revision: snapshot.revision,
                title: "My own edited title".into(),
                artist: signal.artist.clone(),
                album: signal.album.clone(),
                album_artist: signal.album_artist.clone(),
                genre: signal.genre.clone(),
                year: signal.year,
                track_number: signal.track_number,
                disc_number: signal.disc_number,
            },
        )
        .await
        .unwrap();
    let repeated = fixture.scan("alice", &source).await;
    assert_eq!(repeated.status, "completed");
    assert_eq!(repeated.unchanged, 2);
    assert_eq!(repeated.imported + repeated.duplicates, 0);
    assert_eq!(
        fixture
            .media
            .metadata("alice", &signal.id)
            .await
            .unwrap()
            .unwrap()
            .track
            .title,
        "My own edited title"
    );
    // Rewriting identical source bytes changes its signature and exercises actual
    // ingest/dedup, which still preserves the edited catalog.
    tokio::fs::write(&flac, FLAC).await.unwrap();
    let rewritten = fixture.scan("alice", &source).await;
    assert_eq!(rewritten.duplicates, 1);
    assert_eq!(rewritten.unchanged, 1);
    assert_eq!(
        fixture
            .media
            .metadata("alice", &signal.id)
            .await
            .unwrap()
            .unwrap()
            .track
            .title,
        "My own edited title"
    );
    assert_eq!(tokio::fs::read(&flac).await.unwrap(), FLAC);
    assert_eq!(tokio::fs::read(&mp3).await.unwrap(), MP3);
    let asset = fixture
        .media
        .asset("alice", &signal.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tokio::fs::read(asset.path).await.unwrap(), FLAC);
    assert!(
        fixture.scanner.list_sources("alice").await.unwrap()[0]
            .last_scan_at
            .is_some()
    );
    tokio::fs::remove_file(&flac).await.unwrap();
    let missing = fixture.scan("alice", &source).await;
    assert_eq!(missing.unchanged, 1);
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 2);
    assert!(
        fixture
            .scanner
            .remove_source("alice", &source)
            .await
            .unwrap()
    );
    assert!(fixture.scanner.jobs("alice").await.unwrap().is_empty());
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scan_files")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn symlinks_fifo_hardlinks_invalid_paths_and_unsupported_entries_are_skipped() {
    use std::os::unix::{
        ffi::OsStringExt,
        fs::{FileTypeExt, symlink},
    };
    let fixture = Fixture::new().await;
    let outside = fixture.directory.path().join("private.flac");
    tokio::fs::write(&outside, FLAC).await.unwrap();
    symlink(&outside, fixture.root.join("escaped.flac")).unwrap();
    symlink(
        fixture.directory.path(),
        fixture.root.join("escaped-directory"),
    )
    .unwrap();
    std::fs::hard_link(&outside, fixture.root.join("hardlinked.flac")).unwrap();
    let fifo = fixture.root.join("pipe.flac");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let invalid = fixture
        .root
        .join(std::ffi::OsString::from_vec(b"bad-\xff.flac".to_vec()));
    tokio::fs::write(&invalid, FLAC).await.unwrap();
    tokio::fs::write(fixture.root.join("valid.flac"), FLAC)
        .await
        .unwrap();
    tokio::fs::write(fixture.root.join("broken.mp3"), b"invalid media")
        .await
        .unwrap();
    let source = fixture.source("alice").await;
    let job = fixture.scan("alice", &source).await;
    assert_eq!(job.status, "completed");
    assert_eq!(job.imported, 1);
    assert_eq!(job.skipped, 5);
    assert_eq!(job.failed_files, 1);
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 1);
    assert_eq!(tokio::fs::read(&outside).await.unwrap(), FLAC);
    let public = serde_json::to_string(&job).unwrap();
    assert!(!public.contains(fixture.directory.path().to_str().unwrap()));
    assert!(!public.contains("broken.mp3"));
    assert!(
        std::fs::symlink_metadata(&fifo)
            .unwrap()
            .file_type()
            .is_fifo()
    );
}

#[tokio::test]
async fn source_queue_ownership_and_limits_are_transactionally_enforced() {
    let fixture = Fixture::new().await;
    let source = fixture.source("alice").await;
    assert!(
        fixture
            .scanner
            .list_sources("bob")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        fixture
            .scanner
            .enqueue("bob", &source)
            .await
            .unwrap_err()
            .downcast_ref::<ScanError>(),
        Some(ScanError::NotFound)
    ));
    assert!(!fixture.scanner.remove_source("bob", &source).await.unwrap());
    assert!(matches!(
        fixture
            .scanner
            .create_source(
                "alice",
                CreateScanSource {
                    name: "Duplicate".into(),
                    root_id: "music".into()
                }
            )
            .await
            .unwrap_err()
            .downcast_ref::<ScanError>(),
        Some(ScanError::Conflict(_))
    ));
    assert!(matches!(
        fixture
            .scanner
            .create_source(
                "alice",
                CreateScanSource {
                    name: "\n".into(),
                    root_id: "music".into()
                }
            )
            .await
            .unwrap_err()
            .downcast_ref::<ScanError>(),
        Some(ScanError::Invalid(_))
    ));
    let (first, second) = tokio::join!(
        fixture.scanner.enqueue("alice", &source),
        fixture.scanner.enqueue("alice", &source)
    );
    let job = match (first, second) {
        (Ok(job), Err(error)) | (Err(error), Ok(job)) => {
            assert!(matches!(
                error.downcast_ref::<ScanError>(),
                Some(ScanError::Conflict(_))
            ));
            job
        }
        _ => panic!("exactly one simultaneous enqueue must succeed"),
    };
    assert!(fixture.scanner.jobs("bob").await.unwrap().is_empty());
    assert!(!fixture.scanner.cancel("bob", &job.id).await.unwrap());
    assert!(matches!(
        fixture
            .scanner
            .remove_source("alice", &source)
            .await
            .unwrap_err()
            .downcast_ref::<ScanError>(),
        Some(ScanError::Conflict(_))
    ));
    assert!(fixture.scanner.cancel("alice", &job.id).await.unwrap());
    assert_eq!(
        fixture.scanner.jobs("alice").await.unwrap()[0].status,
        "cancelled"
    );
    assert!(
        fixture
            .scanner
            .remove_source("alice", &source)
            .await
            .unwrap()
    );
    let limited = fixture.scanner.clone().with_limits(ScanLimits {
        max_sources: 0,
        ..ScanLimits::default()
    });
    assert!(matches!(
        limited
            .create_source(
                "alice",
                CreateScanSource {
                    name: "None".into(),
                    root_id: "music".into()
                }
            )
            .await
            .unwrap_err()
            .downcast_ref::<ScanError>(),
        Some(ScanError::Conflict(_))
    ));
    let alice = fixture.source("alice").await;
    let bob = fixture.source("bob").await;
    let limit = fixture.scanner.clone().with_limits(ScanLimits {
        max_active_jobs: 1,
        ..ScanLimits::default()
    });
    let (first, second) = tokio::join!(limit.enqueue("alice", &alice), limit.enqueue("bob", &bob));
    let (owner, job) = match (first, second) {
        (Ok(job), Err(error)) => {
            assert!(matches!(
                error.downcast_ref::<ScanError>(),
                Some(ScanError::Conflict(_))
            ));
            ("alice", job)
        }
        (Err(error), Ok(job)) => {
            assert!(matches!(
                error.downcast_ref::<ScanError>(),
                Some(ScanError::Conflict(_))
            ));
            ("bob", job)
        }
        _ => panic!("the global queue ceiling admits exactly one simultaneous owner"),
    };
    assert!(limit.cancel(owner, &job.id).await.unwrap());
}

#[tokio::test]
async fn database_rejects_foreign_scan_maps_and_owner_reassignment() {
    let fixture = Fixture::new().await;
    tokio::fs::write(fixture.root.join("signal.flac"), FLAC)
        .await
        .unwrap();
    let alice = fixture.source("alice").await;
    let bob = fixture.source("bob").await;
    assert_eq!(fixture.scan("alice", &alice).await.status, "completed");
    assert_eq!(fixture.scan("bob", &bob).await.status, "completed");
    let alice_track: String =
        sqlx::query_scalar("SELECT track_id FROM scan_files WHERE source_id=?")
            .bind(&alice)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    let bob_track: String = sqlx::query_scalar("SELECT track_id FROM scan_files WHERE source_id=?")
        .bind(&bob)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_ne!(
        alice_track, bob_track,
        "shared bytes keep private catalog identities"
    );
    for statement in [
        sqlx::query("INSERT INTO scan_files(source_id,path_key,fingerprint,track_id) VALUES(?,'forged','forged',?)")
            .bind(&alice).bind(&bob_track),
        sqlx::query("UPDATE scan_files SET track_id=? WHERE source_id=?")
            .bind(&bob_track).bind(&alice),
        sqlx::query("UPDATE scan_files SET source_id=? WHERE source_id=?")
            .bind(&bob).bind(&alice),
        sqlx::query("UPDATE scan_sources SET owner_id='bob' WHERE id=?")
            .bind(&alice),
        sqlx::query("UPDATE tracks SET owner_id='bob' WHERE id=?")
            .bind(&alice_track),
    ] {
        assert!(statement.execute(&fixture.pool).await.is_err());
    }
    let invalid: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_files f JOIN scan_sources s ON s.id=f.source_id JOIN tracks t ON t.id=f.track_id WHERE s.owner_id<>t.owner_id")
        .fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(invalid, 0);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&fixture.pool)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        tokio::fs::read(fixture.root.join("signal.flac"))
            .await
            .unwrap(),
        FLAC
    );
}

#[tokio::test]
async fn bounded_files_depth_disk_reserve_and_cleanup_are_real() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new().await;
    tokio::fs::create_dir(fixture.root.join("deep"))
        .await
        .unwrap();
    tokio::fs::write(fixture.root.join("deep/hidden.flac"), FLAC)
        .await
        .unwrap();
    tokio::fs::write(fixture.root.join("visible.flac"), FLAC)
        .await
        .unwrap();
    let source = fixture.source("alice").await;
    let shallow = fixture.scanner.clone().with_limits(ScanLimits {
        max_depth: 1,
        ..ScanLimits::default()
    });
    let job = shallow.enqueue("alice", &source).await.unwrap();
    let job = finish_job(&shallow, "alice", &job.id).await;
    assert_eq!(job.imported, 1);
    assert_eq!(job.skipped, 1);
    let no_files = fixture.scanner.clone().with_limits(ScanLimits {
        max_files: 0,
        ..ScanLimits::default()
    });
    let job = no_files.enqueue("alice", &source).await.unwrap();
    let job = finish_job(&no_files, "alice", &job.id).await;
    assert_eq!(job.status, "failed");
    assert_eq!(
        job.error.as_deref(),
        Some("Audio file or fingerprint limit exceeded")
    );
    tokio::fs::write(fixture.root.join("visible.flac"), MP3)
        .await
        .unwrap();
    let no_space = fixture.scanner.clone().with_limits(ScanLimits {
        min_free_bytes: u64::MAX,
        ..ScanLimits::default()
    });
    let job = no_space.enqueue("alice", &source).await.unwrap();
    let job = finish_job(&no_space, "alice", &job.id).await;
    assert_eq!(job.status, "failed");
    assert_eq!(
        job.error.as_deref(),
        Some("Temporary storage reserve is insufficient")
    );
    let staging = fixture.directory.path().join("staging");
    let owned = staging.join("scan-1234567890abcdef.part");
    let unrelated = staging.join("scan-user-notes.part");
    let link = staging.join("scan-abcdefghijklmnop.part");
    tokio::fs::write(&owned, b"abandoned").await.unwrap();
    tokio::fs::write(&unrelated, b"preserve").await.unwrap();
    symlink(&unrelated, &link).unwrap();
    fixture.scanner.initialize().await.unwrap();
    assert!(!owned.exists());
    assert_eq!(tokio::fs::read(&unrelated).await.unwrap(), b"preserve");
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[tokio::test]
async fn real_restart_recovers_running_jobs_and_fingerprints_without_reimport() {
    let fixture = Fixture::new().await;
    tokio::fs::write(fixture.root.join("signal.flac"), FLAC)
        .await
        .unwrap();
    let source = fixture.source("alice").await;
    assert_eq!(fixture.scan("alice", &source).await.imported, 1);
    let recovering = fixture.scanner.enqueue("alice", &source).await.unwrap();
    sqlx::query("UPDATE scan_jobs SET status='running',visited=999,imported=888 WHERE id=?")
        .bind(&recovering.id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    fixture.pool.close().await;
    let pool = vela_core::database::open(fixture.directory.path())
        .await
        .unwrap();
    let media = MediaService::new(
        pool.clone(),
        fixture.directory.path().to_owned(),
        1024 * 1024,
    );
    let reopened = Scanner::new(
        pool.clone(),
        media.clone(),
        fixture.directory.path().to_owned(),
        1024 * 1024,
        vec![RootConfig {
            id: "music".into(),
            name: "Music folder".into(),
            path: fixture.root.clone(),
        }],
    );
    reopened.initialize().await.unwrap();
    let recovered = finish_job(&reopened, "alice", &recovering.id).await;
    assert_eq!(recovered.status, "completed");
    assert_eq!(recovered.visited, 1);
    assert_eq!(recovered.imported, 0);
    assert_eq!(recovered.unchanged, 1);
    assert_eq!(media.stats("alice").await.unwrap().tracks, 1);
    let pending = reopened.enqueue("alice", &source).await.unwrap();
    sqlx::query("UPDATE scan_jobs SET status='running',cancel_requested=1 WHERE id=?")
        .bind(&pending.id)
        .execute(&pool)
        .await
        .unwrap();
    let recovered = finish_job(&reopened, "alice", &pending.id).await;
    assert_eq!(recovered.status, "cancelled");
    assert_eq!(recovered.imported, 0);
}

#[tokio::test]
async fn root_final_and_ancestor_symlinks_are_unavailable_and_never_traversed() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new().await;
    let final_link = fixture.directory.path().join("final-link");
    let parent_link = fixture.directory.path().join("parent-link");
    symlink(&fixture.root, &final_link).unwrap();
    symlink(fixture.directory.path(), &parent_link).unwrap();
    let scanner = Scanner::new(
        fixture.pool.clone(),
        fixture.media.clone(),
        fixture.directory.path().to_owned(),
        1024 * 1024,
        vec![
            RootConfig {
                id: "final".into(),
                name: "Final link".into(),
                path: final_link,
            },
            RootConfig {
                id: "ancestor".into(),
                name: "Ancestor link".into(),
                path: parent_link.join("sources"),
            },
        ],
    );
    let roots = scanner.roots().await.unwrap();
    assert_eq!(roots.len(), 2);
    assert!(roots.iter().all(|root| !root.available));
    let source = scanner
        .create_source(
            "alice",
            CreateScanSource {
                name: "Unsafe root".into(),
                root_id: "final".into(),
            },
        )
        .await
        .unwrap();
    let job = scanner.enqueue("alice", &source.id).await.unwrap();
    let job = finish_job(&scanner, "alice", &job.id).await;
    assert_eq!(job.status, "failed");
    assert_eq!(
        job.error.as_deref(),
        Some("The configured music folder is unavailable or unsafe")
    );
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 0);
    assert!(!serde_json::to_string(&roots).unwrap().contains("sources"));
}

#[tokio::test]
async fn running_cancel_and_shutdown_drain_snapshots_then_resume_safely() {
    // Each new path uses actual Lofty/artwork decoding even for equal bytes.
    // A small real batch exercises cancellation without turning a regression
    // check into a debug-build artwork throughput benchmark.
    const FILE_COUNT: i64 = 24;
    let fixture = Fixture::new().await;
    for index in 0..FILE_COUNT {
        tokio::fs::write(fixture.root.join(format!("song-{index:03}.flac")), FLAC)
            .await
            .unwrap();
    }
    let source = fixture.source("alice").await;
    let job = fixture.scanner.enqueue("alice", &source).await.unwrap();
    let shutdown = CancellationToken::new();
    let worker = {
        let scanner = fixture.scanner.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { scanner.run(shutdown).await })
    };
    wait_for_progress(&fixture.scanner, "alice", &job.id).await;
    assert!(fixture.scanner.cancel("alice", &job.id).await.unwrap());
    let cancelled = wait_for_status(&fixture.scanner, "alice", &job.id, "cancelled").await;
    assert!(
        cancelled.visited < FILE_COUNT,
        "cancellation must stop the running pass"
    );
    shutdown.cancel();
    worker.await.unwrap().unwrap();
    assert_no_snapshots(&fixture.directory.path().join("staging")).await;
    let counts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_files WHERE source_id=?")
        .bind(&source)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert!(counts > 0);
    let resumed = fixture.scanner.enqueue("alice", &source).await.unwrap();
    let shutdown = CancellationToken::new();
    let worker = {
        let scanner = fixture.scanner.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { scanner.run(shutdown).await })
    };
    wait_for_progress(&fixture.scanner, "alice", &resumed.id).await;
    shutdown.cancel();
    worker.await.unwrap().unwrap();
    let saved = fixture
        .scanner
        .jobs("alice")
        .await
        .unwrap()
        .into_iter()
        .find(|job| job.id == resumed.id)
        .unwrap();
    assert_eq!(
        saved.status, "queued",
        "shutdown retains a resumable durable job"
    );
    assert_no_snapshots(&fixture.directory.path().join("staging")).await;
    let completed = finish_job(&fixture.scanner, "alice", &resumed.id).await;
    assert_eq!(completed.status, "completed");
    assert_eq!(completed.visited, FILE_COUNT);
    assert_eq!(completed.imported, 0);
    assert_eq!(completed.duplicates + completed.unchanged, FILE_COUNT);
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scan_files WHERE source_id=?")
            .bind(&source)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        FILE_COUNT
    );
    assert_eq!(
        tokio::fs::read(fixture.root.join("song-000.flac"))
            .await
            .unwrap(),
        FLAC
    );
    assert_no_snapshots(&fixture.directory.path().join("staging")).await;
}

async fn wait_for_progress(scanner: &Scanner, owner: &str, id: &str) {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let job = scanner
                .jobs(owner)
                .await
                .unwrap()
                .into_iter()
                .find(|job| job.id == id)
                .unwrap();
            if job.status == "running" && job.visited > 0 {
                return;
            }
            assert_ne!(
                job.status, "completed",
                "test pass must still be active at cancellation"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

async fn wait_for_status(scanner: &Scanner, owner: &str, id: &str, status: &str) -> ScanJob {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let job = scanner
                .jobs(owner)
                .await
                .unwrap()
                .into_iter()
                .find(|job| job.id == id)
                .unwrap();
            if job.status == status {
                break job;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap()
}

async fn assert_no_snapshots(path: &std::path::Path) {
    let mut entries = tokio::fs::read_dir(path).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        assert!(!entry.file_name().to_string_lossy().starts_with("scan-"));
    }
}
