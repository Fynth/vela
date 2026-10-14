use super::*;
use axum::{
    Router,
    body::Body,
    http::{Response, StatusCode},
    routing::get,
};
use std::sync::atomic::{AtomicUsize, Ordering};

const HOST: &str = "fixture.vela.example";

fn wav() -> Vec<u8> {
    // Real one-second mono PCM audio, generated locally without external assets.
    let samples = 8000_u32;
    let data_size = samples * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&8000_u32.to_le_bytes());
    bytes.extend_from_slice(&16000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in 0..samples {
        let value = if sample % 32 < 16 {
            1000_i16
        } else {
            -1000_i16
        };
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

struct Fixture {
    service: ImportService,
    _dir: tempfile::TempDir,
    stop_http: CancellationToken,
    http_task: tokio::task::JoinHandle<()>,
}

impl Fixture {
    async fn new(max_file_bytes: u64) -> Self {
        let attempts = Arc::new(AtomicUsize::new(0));
        let audio = wav();
        let router = Router::new()
            .route(
                "/audio.wav",
                get({
                    let bytes = audio.clone();
                    move || {
                        let bytes = bytes.clone();
                        async move {
                            Response::builder()
                                .header("Content-Type", "audio/wav")
                                .body(Body::from(bytes))
                                .unwrap()
                        }
                    }
                }),
            )
            .route(
                "/temporarily-unavailable.wav",
                get({
                    let bytes = audio.clone();
                    move || {
                        let bytes = bytes.clone();
                        let attempts = attempts.clone();
                        async move {
                            if attempts.fetch_add(1, Ordering::SeqCst) < 2 {
                                Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .body(Body::empty())
                                    .unwrap()
                            } else {
                                Response::builder()
                                    .header("Content-Type", "audio/wav")
                                    .body(Body::from(bytes))
                                    .unwrap()
                            }
                        }
                    }
                }),
            )
            .route(
                "/html.mp3",
                get(|| async { ([("Content-Type", "text/html")], "<html>not audio</html>") }),
            )
            .route(
                "/invalid.wav",
                get(|| async { ([("Content-Type", "audio/wav")], "invalid audio bytes") }),
            )
            .route(
                "/private.wav",
                get(|| async {
                    Response::builder()
                        .status(StatusCode::FOUND)
                        .header("Location", "http://169.254.169.254/latest/meta-data")
                        .body(Body::empty())
                        .unwrap()
                }),
            )
            .route(
                "/loop.wav",
                get(|| async {
                    Response::builder()
                        .status(StatusCode::FOUND)
                        .header("Location", "/loop.wav")
                        .body(Body::empty())
                        .unwrap()
                }),
            )
            .route(
                "/redirect.wav",
                get(|| async {
                    Response::builder()
                        .status(StatusCode::FOUND)
                        .header("Location", "/audio.wav")
                        .body(Body::empty())
                        .unwrap()
                }),
            )
            .route(
                "/unknown-size.wav",
                get({
                    let bytes = audio.clone();
                    move || {
                        let bytes = bytes.clone();
                        async move {
                            let chunks = futures_util::stream::iter(
                                bytes
                                    .chunks(1024)
                                    .map(|chunk| Ok::<_, std::io::Error>(chunk.to_vec()))
                                    .collect::<Vec<_>>(),
                            );
                            Response::builder()
                                .header("Content-Type", "audio/wav")
                                .body(Body::from_stream(chunks))
                                .unwrap()
                        }
                    }
                }),
            )
            .route(
                "/slow.wav",
                get(|| async {
                    let chunks = futures_util::stream::unfold(0_u32, |index| async move {
                        if index >= 10 {
                            return None;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        Some((Ok::<_, std::io::Error>(vec![0_u8; 1024]), index + 1))
                    });
                    Response::builder()
                        .header("Content-Type", "audio/wav")
                        .body(Body::from_stream(chunks))
                        .unwrap()
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stop_http = CancellationToken::new();
        let stop = stop_http.clone();
        let http_task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        let dir = tempfile::tempdir().unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        for owner in ["owner", "other"] {
            sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,'test-only')")
                .bind(owner)
                .bind(owner)
                .execute(&pool)
                .await
                .unwrap();
        }
        let media = MediaService::new(pool.clone(), dir.path().to_path_buf(), max_file_bytes);
        media.initialize().await.unwrap();
        let mut service = ImportService::new(pool, media, dir.path().to_path_buf(), max_file_bytes);
        service.http = ControlledHttp::for_fixture(HOST, address);
        Self {
            service,
            _dir: dir,
            stop_http,
            http_task,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{HOST}/{path}")
    }

    fn worker(&self) -> (CancellationToken, tokio::task::JoinHandle<Result<()>>) {
        let shutdown = CancellationToken::new();
        let service = self.service.clone();
        let token = shutdown.clone();
        (
            shutdown,
            tokio::spawn(async move { service.run(token).await }),
        )
    }

    async fn wait_status(&self, id: &str, status: &str) -> ImportJob {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let job = self
                    .service
                    .list("owner")
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|job| job.id == id)
                    .unwrap();
                if job.status == status {
                    return job;
                }
                assert!(
                    !matches!(job.status.as_str(), "failed" | "cancelled" | "completed"),
                    "unexpected job state: {} {:?}",
                    job.status,
                    job.error
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap()
    }

    async fn assert_staging_empty(&self) {
        assert!(
            fs::read_dir(&self.service.staging_dir)
                .await
                .unwrap()
                .next_entry()
                .await
                .unwrap()
                .is_none()
        );
    }

    async fn wait_download_bytes(&self, id: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let job = self
                    .service
                    .list("owner")
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|job| job.id == id)
                    .unwrap();
                if job.downloaded_bytes > 0 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }

    async fn close(self) {
        self.stop_http.cancel();
        self.http_task.await.unwrap();
        self.service.pool.close().await;
    }
}

#[tokio::test]
async fn real_http_download_ingests_audio_and_deduplicates_privately() {
    let fixture = Fixture::new(256 * 1024).await;
    let preview = fixture
        .service
        .preview(&fixture.url("redirect.wav"))
        .await
        .unwrap();
    assert_eq!(preview.filename, "audio.wav");
    assert_eq!(preview.size_bytes, Some(wav().len() as i64));
    assert!(
        fixture
            .service
            .preview(&fixture.url("html.mp3"))
            .await
            .is_err()
    );
    assert!(
        fixture
            .service
            .preview(&fixture.url("private.wav"))
            .await
            .is_err()
    );
    assert!(
        fixture
            .service
            .preview(&fixture.url("loop.wav"))
            .await
            .is_err()
    );
    let job = fixture
        .service
        .enqueue("owner", &fixture.url("audio.wav"))
        .await
        .unwrap();
    assert!(fixture.service.list("other").await.unwrap().is_empty());
    assert!(!fixture.service.cancel("other", &job.id).await.unwrap());
    assert!(!fixture.service.retry("other", &job.id).await.unwrap());
    let (stop, worker) = fixture.worker();
    let completed = fixture.wait_status(&job.id, "completed").await;
    assert_eq!(completed.downloaded_bytes, wav().len() as i64);
    let track = fixture
        .service
        .media
        .get_track("owner", completed.track_id.as_ref().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(track.codec, "WAV audio");
    assert_eq!(track.duration_ms, 1000);
    assert!(
        fixture
            .service
            .media
            .get_track("other", &track.id)
            .await
            .unwrap()
            .is_none()
    );
    let duplicate = fixture
        .service
        .enqueue("owner", &fixture.url("audio.wav"))
        .await
        .unwrap();
    assert_eq!(
        fixture
            .wait_status(&duplicate.id, "completed")
            .await
            .track_id,
        completed.track_id
    );
    assert_eq!(
        fixture.service.media.stats("owner").await.unwrap().tracks,
        1
    );
    stop.cancel();
    worker.await.unwrap().unwrap();
    fixture.assert_staging_empty().await;
    fixture.close().await;
}

#[tokio::test]
async fn cancellation_restart_recovery_and_retry_are_durable() {
    let fixture = Fixture::new(256 * 1024).await;
    let cancelled = fixture
        .service
        .enqueue("owner", &fixture.url("audio.wav"))
        .await
        .unwrap();
    assert!(
        fixture
            .service
            .cancel("owner", &cancelled.id)
            .await
            .unwrap()
    );
    assert!(fixture.service.retry("owner", &cancelled.id).await.unwrap());
    sqlx::query(
        "UPDATE import_jobs SET status='downloading',attempt=2,downloaded_bytes=100 WHERE id=?",
    )
    .bind(&cancelled.id)
    .execute(&fixture.service.pool)
    .await
    .unwrap();
    fs::create_dir_all(&fixture.service.staging_dir)
        .await
        .unwrap();
    fs::write(
        fixture
            .service
            .staging_dir
            .join(format!("vela-import-{}.part", cancelled.id)),
        b"stale partial",
    )
    .await
    .unwrap();
    let (stop, worker) = fixture.worker();
    fixture.wait_status(&cancelled.id, "completed").await;
    let slow = fixture
        .service
        .enqueue("owner", &fixture.url("slow.wav"))
        .await
        .unwrap();
    fixture.wait_status(&slow.id, "downloading").await;
    fixture.wait_download_bytes(&slow.id).await;
    assert!(fixture.service.cancel("owner", &slow.id).await.unwrap());
    fixture.wait_status(&slow.id, "cancelled").await;
    stop.cancel();
    worker.await.unwrap().unwrap();
    fixture.assert_staging_empty().await;
    fixture.close().await;
}

#[tokio::test]
async fn streamed_size_limit_and_real_audio_validation_fail_safely() {
    let fixture = Fixture::new(1024).await;
    assert!(
        fixture
            .service
            .preview(&fixture.url("audio.wav"))
            .await
            .is_err()
    );
    let size_job = fixture
        .service
        .enqueue("owner", &fixture.url("unknown-size.wav"))
        .await
        .unwrap();
    let invalid_job = fixture
        .service
        .enqueue("owner", &fixture.url("invalid.wav"))
        .await
        .unwrap();
    let (stop, worker) = fixture.worker();
    let oversized = fixture.wait_status(&size_job.id, "failed").await;
    assert_eq!(
        oversized.error.as_deref(),
        Some("Audio file exceeds the configured file-size limit")
    );
    let invalid = fixture.wait_status(&invalid_job.id, "failed").await;
    assert_eq!(
        invalid.error.as_deref(),
        Some("Imported file is not valid supported audio")
    );
    assert_eq!(
        fixture.service.media.stats("owner").await.unwrap().tracks,
        0
    );
    stop.cancel();
    worker.await.unwrap().unwrap();
    fixture.assert_staging_empty().await;
    fixture.close().await;
}

#[tokio::test]
async fn transient_failures_retry_with_a_bounded_attempt_count() {
    let fixture = Fixture::new(256 * 1024).await;
    let job = fixture
        .service
        .enqueue("owner", &fixture.url("temporarily-unavailable.wav"))
        .await
        .unwrap();
    let (stop, worker) = fixture.worker();
    fixture.wait_status(&job.id, "completed").await;
    let attempts: i64 = sqlx::query_scalar("SELECT attempt FROM import_jobs WHERE id=?")
        .bind(&job.id)
        .fetch_one(&fixture.service.pool)
        .await
        .unwrap();
    assert_eq!(attempts, 3);
    stop.cancel();
    worker.await.unwrap().unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_enqueue_respects_owner_queue_quota() {
    let fixture = Fixture::new(256 * 1024).await;
    let url = fixture.url("audio.wav");
    let results =
        futures_util::future::join_all((0..16).map(|_| fixture.service.enqueue("owner", &url)))
            .await;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 8);
    assert_eq!(fixture.service.list("owner").await.unwrap().len(), 8);
    fixture.close().await;
}

#[tokio::test]
async fn shutdown_requeues_active_download_and_cleans_partial_file() {
    let fixture = Fixture::new(256 * 1024).await;
    let job = fixture
        .service
        .enqueue("owner", &fixture.url("slow.wav"))
        .await
        .unwrap();
    let (stop, worker) = fixture.worker();
    fixture.wait_download_bytes(&job.id).await;
    stop.cancel();
    worker.await.unwrap().unwrap();
    let queued = fixture.service.list("owner").await.unwrap().remove(0);
    assert_eq!(queued.status, "queued");
    assert_eq!(queued.downloaded_bytes, 0);
    fixture.assert_staging_empty().await;
    fixture.close().await;
}

#[tokio::test]
async fn managed_storage_quota_is_checked_before_download() {
    let mut fixture = Fixture::new(256 * 1024).await;
    fixture.service.limits.max_storage_bytes = 1024;
    let job = fixture
        .service
        .enqueue("owner", &fixture.url("audio.wav"))
        .await
        .unwrap();
    let (stop, worker) = fixture.worker();
    let failed = fixture.wait_status(&job.id, "failed").await;
    assert_eq!(failed.downloaded_bytes, 0);
    assert_eq!(
        failed.error.as_deref(),
        Some("Import storage quota or free-space reserve exceeded")
    );
    stop.cancel();
    worker.await.unwrap().unwrap();
    fixture.assert_staging_empty().await;
    fixture.close().await;
}

#[tokio::test]
async fn staging_recovery_removes_only_owned_regular_files() {
    let fixture = Fixture::new(256 * 1024).await;
    fs::create_dir_all(&fixture.service.staging_dir)
        .await
        .unwrap();
    let owned = fixture
        .service
        .staging_dir
        .join(format!("vela-import-{}.part", Uuid::new_v4()));
    let unrelated = fixture.service.staging_dir.join("my-important-audio.part");
    let invalid_id = fixture
        .service
        .staging_dir
        .join("vela-import-user-file.part");
    let outside = fixture._dir.path().join("preserved.wav");
    fs::write(&owned, b"abandoned download").await.unwrap();
    fs::write(&unrelated, b"preserve this file").await.unwrap();
    fs::write(&invalid_id, b"also preserve this file")
        .await
        .unwrap();
    fs::write(&outside, b"outside staging").await.unwrap();
    #[cfg(unix)]
    let symlink = {
        let symlink = fixture
            .service
            .staging_dir
            .join(format!("vela-import-{}.part", Uuid::new_v4()));
        std::os::unix::fs::symlink(&outside, &symlink).unwrap();
        symlink
    };
    fixture.service.cleanup_staging().await.unwrap();
    assert!(!owned.exists());
    assert_eq!(fs::read(&unrelated).await.unwrap(), b"preserve this file");
    assert_eq!(
        fs::read(&invalid_id).await.unwrap(),
        b"also preserve this file"
    );
    assert_eq!(fs::read(&outside).await.unwrap(), b"outside staging");
    #[cfg(unix)]
    assert!(fs::symlink_metadata(&symlink).await.unwrap().is_symlink());
    fixture.close().await;
}

#[tokio::test]
async fn existing_published_content_is_verified_and_never_overwritten() {
    let fixture = Fixture::new(256 * 1024).await;
    let audio = wav();
    let hash = blake3::hash(&audio).to_hex().to_string();
    let published = fixture
        ._dir
        .path()
        .join("media")
        .join(format!("{hash}.audio"));
    let corrupt = vec![1_u8; audio.len()];
    fs::write(&published, &corrupt).await.unwrap();
    let job = fixture
        .service
        .enqueue("owner", &fixture.url("audio.wav"))
        .await
        .unwrap();
    let (stop, worker) = fixture.worker();
    fixture.wait_status(&job.id, "failed").await;
    assert_eq!(fs::read(&published).await.unwrap(), corrupt);
    assert_eq!(
        fixture.service.media.stats("owner").await.unwrap().tracks,
        0
    );
    // Simulate an administrator repairing the fixture to a valid orphan from a prior publish.
    fs::write(&published, &audio).await.unwrap();
    assert!(fixture.service.retry("owner", &job.id).await.unwrap());
    fixture.wait_status(&job.id, "completed").await;
    assert_eq!(fs::read(&published).await.unwrap(), audio);
    assert_eq!(
        fixture.service.media.stats("owner").await.unwrap().tracks,
        1
    );
    stop.cancel();
    worker.await.unwrap().unwrap();
    fixture.assert_staging_empty().await;
    fixture.close().await;
}
