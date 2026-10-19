//! Real TCP/HTTP tests against isolated SQLite and filesystem stores.
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vela_core::config::Config;
use vela_domain::*;
use vela_server::{AppState, router};

const AUDIO: &[u8] = include_bytes!("../../../crates/media/tests/fixtures/signal.flac");
const OTHER_AUDIO: &[u8] = include_bytes!("../../../crates/media/tests/fixtures/beacon.mp3");

#[tokio::test]
async fn personal_mixes_use_only_current_consented_history_and_owned_feedback() {
    let app = TestApp::new().await;
    let other = app.user("mix_listener").await;
    let own = app.upload(&app.admin).await.track;
    // Same physical audio does not give the other account our track identity.
    let foreign = app.upload(&other).await.track;
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(OTHER_AUDIO.to_vec()).file_name("beacon.mp3"),
    );
    let second: UploadResult = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(form)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let endpoint = "/api/recommendations?limit=10";
    let response = app
        .request(Method::GET, endpoint, &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let cold: Recommendations = response.json().await.unwrap();
    assert!(!cold.history_enabled);
    assert_eq!(cold.anchor_count, 0);
    assert_eq!(cold.candidate_count, 2);
    assert_eq!(cold.discovery.len(), 2);
    assert!(cold.rotation.is_empty());
    assert!(cold.discovery.iter().all(|x| x.track.id != foreign.id));
    for suffix in ["?limit=0", "?limit=31", "?limit=-1", "?unknown=1"] {
        assert_eq!(
            app.request(
                Method::GET,
                &format!("/api/recommendations{suffix}"),
                &app.admin
            )
            .send()
            .await
            .unwrap()
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        app.client
            .get(format!("{}{endpoint}", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let feedback = RecommendationFeedback {
        track_id: second.track.id.clone(),
        hidden: true,
    };
    assert_eq!(
        app.client
            .put(format!("{}/api/recommendations/feedback", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&feedback)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.request(Method::PUT, "/api/recommendations/feedback", &app.admin)
            .json(&json!({"track_id":own.id,"hidden":true,"owner_id":other.user.id}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.request(Method::PUT, "/api/recommendations/feedback", &app.admin)
            .json(&RecommendationFeedback {
                track_id: foreign.id.clone(),
                hidden: true
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    for _ in 0..2 {
        let receipt: RecommendationFeedbackReceipt = app
            .request(Method::PUT, "/api/recommendations/feedback", &app.admin)
            .json(&feedback)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            receipt.hidden_count, 1,
            "desired-state feedback must be idempotent"
        );
    }
    let hidden: Recommendations = app
        .request(Method::GET, endpoint, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hidden.hidden_count, 1);
    assert!(
        hidden
            .discovery
            .iter()
            .all(|x| x.track.id != second.track.id)
    );
    let exported: RecommendationExclusions = app
        .request(Method::GET, "/api/recommendations/feedback", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(exported.track_ids, vec![second.track.id.clone()]);
    let foreign_export: RecommendationExclusions = app
        .request(Method::GET, "/api/recommendations/feedback", &other)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(foreign_export.track_ids.is_empty());
    app.request(Method::PUT, "/api/recommendations/feedback", &app.admin)
        .json(&RecommendationFeedback {
            track_id: second.track.id.clone(),
            hidden: false,
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let device = uuid::Uuid::new_v4().to_string();
    let enabled: ListeningPreferences = app
        .request(Method::PUT, "/api/listening/preferences", &app.admin)
        .json(&UpdateListeningPreferences {
            enabled: true,
            device_enabled: true,
            device_id: device.clone(),
            retention_days: 90,
            revision: 0,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let batch = ListeningBatch {
        device_id: device.clone(),
        revision: enabled.revision,
        events: vec![ListeningProgress {
            session_id: uuid::Uuid::new_v4().to_string(),
            sequence: 1,
            track_id: own.id.clone(),
            listened_ms: own.duration_ms.min(30000),
            position_ms: own.duration_ms.min(30000),
            started_at: vela_core::now() - 60,
            ended: true,
        }],
    };
    for _ in 0..2 {
        app.request(Method::POST, "/api/listening/events", &app.admin)
            .json(&batch)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let personal: Recommendations = app
        .request(Method::GET, endpoint, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(personal.history_enabled);
    assert_eq!(personal.privacy_revision, enabled.revision);
    assert_eq!(personal.rotation.len(), 1);
    assert_eq!(personal.rotation[0].track.id, own.id);
    assert!(personal.discovery.iter().all(|x| x.track.id != own.id));
    let disabled: ListeningPreferences = app
        .request(Method::PUT, "/api/listening/preferences", &app.admin)
        .json(&UpdateListeningPreferences {
            enabled: false,
            device_enabled: true,
            device_id: device.clone(),
            retention_days: 90,
            revision: enabled.revision,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let retained: ListeningExport = app
        .request(Method::GET, "/api/listening/export", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        retained.events.len(),
        1,
        "opting out preserves history until explicit erase"
    );
    let private: Recommendations = app
        .request(Method::GET, endpoint, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!private.history_enabled);
    assert_eq!(private.privacy_revision, disabled.revision);
    assert_eq!(
        private.anchor_count, 0,
        "retained history must not influence opted-out recommendations"
    );
    assert!(private.rotation.is_empty());
    assert_eq!(private.discovery.len(), 2);
    // Hiding affects recommendations only, never the catalog or stream ACL.
    app.request(Method::PUT, "/api/recommendations/feedback", &app.admin)
        .json(&feedback)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let stream = app
        .request(
            Method::GET,
            &format!("/api/tracks/{}/stream", second.track.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    assert_eq!(stream.bytes().await.unwrap().as_ref(), OTHER_AUDIO);
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", foreign.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    let cleared: RecommendationFeedbackReceipt = app
        .request(Method::DELETE, "/api/recommendations/feedback", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cleared.hidden_count, 0);
    let response = app
        .request(Method::GET, endpoint, &other)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!response.contains(&own.id) && !response.contains(&second.track.id));
    assert!(
        !response.contains("storage_path")
            && !response.contains("password_hash")
            && !response.contains("listening_sessions")
    );
}

#[tokio::test]
async fn actual_analysis_worker_and_radio_remain_private_and_preserve_original_streams() {
    let app = TestApp::new().await;
    let other = app.user("analysis_other").await;
    let seed = app.upload(&app.admin).await.track;
    let foreign = app.upload(&other).await.track;
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(OTHER_AUDIO.to_vec()).file_name("beacon.mp3"),
    );
    let upload: UploadResult = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(form)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let detail_path = format!("/api/tracks/{}/analysis", seed.id);
    let initial: TrackAnalysis = app
        .request(Method::GET, &detail_path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(initial.status, AnalysisStatus::NotAnalyzed);
    let radio_path = format!("/api/tracks/{}/radio?limit=10", seed.id);
    let initial: TrackRadio = app
        .request(Method::GET, &radio_path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!initial.uses_audio_features);
    assert_eq!(initial.items.len(), 1);
    assert_eq!(initial.items[0].track.id, upload.track.id);
    let mixed = EnqueueAnalysis {
        track_ids: vec![seed.id.clone(), foreign.id],
    };
    assert_eq!(
        app.request(Method::POST, "/api/audio-analysis", &app.admin)
            .json(&mixed)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let empty: AnalysisLibraryStatus = app
        .request(Method::GET, "/api/audio-analysis", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(empty.queued, 0);
    let selected = EnqueueAnalysis {
        track_ids: vec![seed.id.clone(), upload.track.id.clone()],
    };
    assert_eq!(
        app.client
            .post(format!("{}/api/audio-analysis", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&selected)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let receipt: AnalysisReceipt = app
        .request(Method::POST, "/api/audio-analysis", &app.admin)
        .json(&selected)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(receipt.queued, 2);
    let token = tokio_util::sync::CancellationToken::new();
    let worker = tokio::spawn({
        let service = app.state.analysis.clone();
        let token = token.clone();
        async move { service.run(token).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let status: AnalysisLibraryStatus = app
                .request(Method::GET, "/api/audio-analysis", &app.admin)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if status.complete == 2 {
                break;
            }
            assert_eq!(status.failed, 0);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let analysis: TrackAnalysis = app
        .request(Method::GET, &detail_path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let features = analysis.features.unwrap();
    assert_eq!(features.sample_rate, 44100);
    assert_eq!(features.channels, 1);
    assert!(features.rms > 0.0 && features.rms < 1.0 && features.peak > features.rms);
    assert!(features.spectral_centroid_hz.is_some());
    assert!(features.tempo_bpm.is_none());
    assert!(features.key.is_none() && features.key_confidence.is_none());
    let chroma = features.chroma.expect("tonal fixture exposes chroma");
    assert!((chroma.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    let centroid = features.spectral_centroid_hz.unwrap();
    let expected =
        ((69.0 + 12.0 * (centroid / 440.0).log2()).round() as i64).rem_euclid(12) as usize;
    assert_eq!(
        chroma
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index),
        Some(expected)
    );
    let radio: TrackRadio = app
        .request(Method::GET, &radio_path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(radio.uses_audio_features);
    assert_eq!(radio.items.len(), 1);
    assert_eq!(radio.items[0].track.id, upload.track.id);
    let duplicate: AnalysisReceipt = app
        .request(Method::POST, "/api/audio-analysis", &app.admin)
        .json(&selected)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(duplicate.queued, 0);
    for path in [&detail_path, &radio_path] {
        assert_eq!(
            app.request(Method::GET, path, &other)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            app.client
                .get(format!("{}{path}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let original = app
        .request(
            Method::GET,
            &format!("/api/tracks/{}/stream", seed.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(&original[..], AUDIO);
    let bad_query = format!("/api/tracks/{}/radio?limit=51", seed.id);
    assert_eq!(
        app.request(Method::GET, &bad_query, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let extra_query = format!("/api/tracks/{}/radio?owner=other", seed.id);
    assert_eq!(
        app.request(Method::GET, &extra_query, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    token.cancel();
    worker.await.unwrap().unwrap();
}

#[tokio::test]
async fn private_listening_consent_idempotence_export_and_erasure_use_real_http() {
    let app = TestApp::new().await;
    let other = app.user("history_other").await;
    let own = app.upload(&app.admin).await.track;
    let foreign = app.upload(&other).await.track;
    let device = uuid::Uuid::new_v4().to_string();
    let path = format!("/api/listening/preferences?device_id={device}");
    let response = app
        .request(Method::GET, &path, &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let initial: ListeningPreferences = response.json().await.unwrap();
    assert!(!initial.enabled && !initial.device_enabled);
    assert_eq!(initial.revision, 0);
    let mut batch = ListeningBatch {
        device_id: device.clone(),
        revision: 0,
        events: vec![ListeningProgress {
            session_id: uuid::Uuid::new_v4().to_string(),
            sequence: 1,
            track_id: own.id.clone(),
            listened_ms: 1000,
            position_ms: 1000,
            started_at: vela_core::now() - 2,
            ended: true,
        }],
    };
    assert_eq!(
        app.request(Method::POST, "/api/listening/events", &app.admin)
            .json(&batch)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let prefs = UpdateListeningPreferences {
        revision: 0,
        enabled: true,
        retention_days: 90,
        device_id: device.clone(),
        device_enabled: true,
    };
    assert_eq!(
        app.client
            .put(format!("{}/api/listening/preferences", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&prefs)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let enabled: ListeningPreferences = app
        .request(Method::PUT, "/api/listening/preferences", &app.admin)
        .json(&prefs)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    batch.revision = enabled.revision;
    let accepted: ListeningReceipt = app
        .request(Method::POST, "/api/listening/events", &app.admin)
        .json(&batch)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(accepted.accepted, 1);
    let duplicate: ListeningReceipt = app
        .request(Method::POST, "/api/listening/events", &app.admin)
        .json(&batch)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(duplicate.duplicates, 1);
    let history: ListeningHistoryPage = app
        .request(Method::GET, "/api/listening/history", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(history.items.len(), 1);
    assert_eq!(history.items[0].track.id, own.id);
    assert_eq!(history.total_listens, 1);
    assert_eq!(history.total_listened_ms, 1000);
    let empty: ListeningHistoryPage = app
        .request(Method::GET, "/api/listening/history", &other)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(empty.items.is_empty());
    // An owned prefix followed by another account's track must roll back as a whole.
    let mut mixed = batch.clone();
    mixed.events[0].session_id = uuid::Uuid::new_v4().to_string();
    let mut second = mixed.events[0].clone();
    second.session_id = uuid::Uuid::new_v4().to_string();
    second.track_id = foreign.id;
    mixed.events.push(second);
    assert_eq!(
        app.request(Method::POST, "/api/listening/events", &app.admin)
            .json(&mixed)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let export = app
        .request(Method::GET, "/api/listening/export", &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(
        export.headers()["content-disposition"],
        "attachment; filename=\"vela-listening-history.json\""
    );
    let exported: ListeningExport = export.json().await.unwrap();
    assert_eq!(exported.events.len(), 1);
    assert_eq!(exported.events[0].progress, batch.events[0]);
    let clear_path = format!("/api/listening/history?device_id={device}");
    let cleared: ListeningPreferences = app
        .request(Method::DELETE, &clear_path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!cleared.enabled && !cleared.device_enabled);
    assert!(cleared.revision > batch.revision);
    let reenable = UpdateListeningPreferences {
        revision: cleared.revision,
        ..prefs
    };
    assert_eq!(
        app.request(Method::PUT, "/api/listening/preferences", &app.admin)
            .json(&reenable)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.request(Method::POST, "/api/listening/events", &app.admin)
            .json(&batch)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let empty: ListeningExport = app
        .request(Method::GET, "/api/listening/export", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(empty.events.is_empty());
    for path in [
        "/api/listening/history",
        "/api/listening/export",
        "/api/listening/preferences?device_id=invalid",
    ] {
        assert_eq!(
            app.client
                .get(format!("{}{path}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    for path in [
        "/api/listening/history?limit=1&owner=other",
        "/api/listening/history?cursor=secret",
        "/api/listening/preferences?device_id=invalid",
    ] {
        assert_eq!(
            app.request(Method::GET, path, &app.admin)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}
struct Actor {
    cookie: String,
    csrf: String,
    user: User,
}
struct TestApp {
    base: String,
    client: Client,
    state: AppState,
    _dir: Option<tempfile::TempDir>,
    task: tokio::task::JoinHandle<()>,
    admin: Actor,
}
impl Drop for TestApp {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl TestApp {
    async fn new() -> Self {
        Self::with_file_limit(1024 * 1024).await
    }
    async fn with_file_limit(max_file_bytes: u64) -> Self {
        Self::with_scanner(max_file_bytes, false).await
    }
    async fn with_scanner(max_file_bytes: u64, enabled: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        let scan_roots = if enabled {
            tokio::fs::create_dir_all(music.join("album"))
                .await
                .unwrap();
            tokio::fs::write(music.join("album/signal.flac"), AUDIO)
                .await
                .unwrap();
            tokio::fs::write(music.join("beacon.mp3"), OTHER_AUDIO)
                .await
                .unwrap();
            vec![vela_core::config::ScanRootConfig {
                id: "fixture".into(),
                name: "Fixture folder".into(),
                path: music,
            }]
        } else {
            Vec::new()
        };
        let config = Config {
            scan_roots,
            bind: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().join("data"),
            web_dir: dir.path().join("web"),
            cookie_secure: false,
            max_file_bytes,
            max_storage_bytes: 20 * 1024 * 1024,
        };
        let state = AppState::initialize(config).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap()
        });
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        let r = client
            .post(format!("{base}/api/setup"))
            .json(&Credentials {
                username: "admin".into(),
                password: "fixture password 12345".into(),
            })
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let admin = actor(r).await;
        Self {
            base,
            client,
            state,
            _dir: Some(dir),
            task,
            admin,
        }
    }
    fn request(&self, method: Method, path: &str, actor: &Actor) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.base))
            .header("Cookie", &actor.cookie)
            .header("X-CSRF-Token", &actor.csrf)
    }
    async fn user(&self, name: &str) -> Actor {
        let c = Credentials {
            username: name.into(),
            password: "fixture password 12345".into(),
        };
        let r = self
            .request(Method::POST, "/api/users", &self.admin)
            .json(&c)
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let r = self
            .client
            .post(format!("{}/api/login", self.base))
            .json(&c)
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        actor(r).await
    }
    async fn upload(&self, actor: &Actor) -> UploadResult {
        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(AUDIO.to_vec())
                .file_name("../../signal.flac")
                .mime_str("audio/flac")
                .unwrap(),
        );
        let r = self
            .request(Method::POST, "/api/upload", actor)
            .multipart(form)
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK, "{}", r.text().await.unwrap());
        r.json().await.unwrap()
    }
}
async fn actor(r: Response) -> Actor {
    let cookie = r.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let body: AuthResponse = r.json().await.unwrap();
    Actor {
        cookie,
        csrf: body.csrf_token,
        user: body.user,
    }
}

#[tokio::test]
async fn global_search_finds_owned_music_and_granted_metadata_without_audio_rights() {
    let app = TestApp::new().await;
    let listener = app.user("searchlistener").await;
    let outsider = app.user("searchoutsider").await;
    let original = app.upload(&app.admin).await.track;
    let private_copy = app.upload(&listener).await.track;
    let list: PlaylistSummary = app
        .request(Method::POST, "/api/playlists", &app.admin)
        .json(&CreatePlaylist {
            name: "Signal sessions".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.request(Method::POST, &format!("/api/playlists/{}/items", list.id), &app.admin)
        .json(&json!({"revision":0,"mutation_id":uuid::Uuid::new_v4().to_string(),"track_id":original.id}))
        .send().await.unwrap().error_for_status().unwrap();
    let r = app
        .request(Method::GET, "/api/search?q=Signal", &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["cache-control"], "no-store");
    let matches: SearchResults = r.json().await.unwrap();
    assert_eq!(matches.tracks[0].id, original.id);
    assert_eq!(matches.playlists[0].id, list.id);
    assert!(matches.albums.is_empty() && matches.artists.is_empty());
    let names: SearchResults = app
        .request(Method::GET, "/api/search?q=Vela", &listener)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(names.artists.len(), 1);
    assert_eq!(names.albums.len(), 1);
    assert_eq!(names.tracks[0].id, private_copy.id);
    app.request(
        Method::POST,
        &format!("/api/playlists/{}/collaborators", list.id),
        &app.admin,
    )
    .json(&CollaboratorRequest {
        username: listener.user.username.clone(),
        can_edit: false,
    })
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
    let granted: SearchResults = app
        .request(Method::GET, "/api/search?q=Signal", &listener)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(granted.playlists[0].id, list.id);
    assert!(!granted.playlists[0].can_edit);
    assert_eq!(granted.tracks.len(), 1);
    assert_eq!(granted.tracks[0].id, private_copy.id);
    assert_ne!(granted.tracks[0].id, original.id);
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", original.id),
            &listener
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    let absent: SearchResults = app
        .request(Method::GET, "/api/search?q=Signal", &outsider)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        absent.tracks.is_empty()
            && absent.albums.is_empty()
            && absent.artists.is_empty()
            && absent.playlists.is_empty()
    );
    app.request(
        Method::DELETE,
        &format!(
            "/api/playlists/{}/collaborators/{}",
            list.id, listener.user.id
        ),
        &app.admin,
    )
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
    let revoked: SearchResults = app
        .request(Method::GET, "/api/search?q=Signal", &listener)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(revoked.playlists.is_empty());
}

#[tokio::test]
async fn global_search_bounds_each_category_hides_empty_groups_and_rejects_invalid_queries() {
    let app = TestApp::new().await;
    app.upload(&app.admin).await;
    // Explicit metadata fixtures exercise query bounds, not fabricated audio or UI.
    for at in 0..12 {
        let asset = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',99)")
            .bind(&asset).bind(&asset).bind(format!("media/query-fixture-{at}.audio"))
            .execute(&app.state.pool).await.unwrap();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,album_artist,duration_ms,codec) VALUES(?,?,?,?,?,?,?,1000,'FLAC')")
            .bind(uuid::Uuid::new_v4().to_string()).bind(&app.admin.user.id).bind(&asset)
            .bind(format!("Matching {at:02}")).bind(format!("Matching artist {at:02}"))
            .bind(format!("Matching album {at:02}")).bind(format!("Matching artist {at:02}"))
            .execute(&app.state.pool).await.unwrap();
        app.request(Method::POST, "/api/playlists", &app.admin)
            .json(&CreatePlaylist {
                name: format!("Matching playlist {at:02}"),
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let result: SearchResults = app
        .request(Method::GET, "/api/search?q=matching", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        (
            result.tracks.len(),
            result.albums.len(),
            result.artists.len(),
            result.playlists.len()
        ),
        (8, 8, 8, 8)
    );
    assert_eq!(result.albums[0].title, "Matching album 00");
    assert_eq!(result.artists[0].name, "Matching artist 00");
    sqlx::query("DELETE FROM tracks WHERE owner_id=? AND title='Matching 00'")
        .bind(&app.admin.user.id)
        .execute(&app.state.pool)
        .await
        .unwrap();
    for query in ["Matching album 00", "Matching artist 00", "%_' OR 1=1 --"] {
        let matches: SearchResults = app
            .request(Method::GET, "/api/search", &app.admin)
            .query(&[("q", query)])
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(matches.albums.is_empty() && matches.artists.is_empty());
    }
    for path in [
        "/api/search",
        "/api/search?q=",
        "/api/search?q=x&extra=1",
        "/api/search?q=a&q=b",
        "/api/search?q=Signal%00",
    ] {
        let response = app
            .request(Method::GET, path, &app.admin)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        let error: ApiError = response.json().await.unwrap();
        assert_eq!(error.error, "Invalid search query");
    }
    let long = "я".repeat(129);
    assert_eq!(
        app.request(Method::GET, "/api/search", &app.admin)
            .query(&[("q", long)])
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.client
            .get(format!("{}/api/search?q=Signal", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn job_center_is_private_validated_and_controls_real_import_jobs() {
    let app = TestApp::new().await;
    let member = app.user("tasklistener").await;
    let unauthenticated = app
        .client
        .get(format!("{}/api/jobs", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        app.request(Method::GET, "/api/jobs?kind=scan", &member)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for query in [
        "kind=other",
        "state=running",
        "limit=-1",
        "limit=4294967296",
        "unknown=1",
        "cursor=broken",
        "cursor=v1:0:00000000000000000000000000000000:import",
    ] {
        let r = app
            .request(Method::GET, &format!("/api/jobs?{query}"), &app.admin)
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST, "accepted {query}");
        assert!(
            r.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("application/json")
        );
        let body: ApiError = r.json().await.unwrap();
        assert!(body.error.starts_with("Invalid task"));
    }
    // No import worker runs in this HTTP fixture. Public literal destinations
    // validate without DNS or any request to an external audio server.
    let mut created = Vec::new();
    for actor in [&app.admin, &member] {
        let r = app
            .request(Method::POST, "/api/imports", actor)
            .json(&ImportRequest {
                url: "https://8.8.8.8/audio.flac?secret=fixture-only".into(),
            })
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        created.push(r.json::<ImportJob>().await.unwrap());
    }
    let id = &created[0].id;
    let r = app
        .request(Method::GET, "/api/jobs", &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["cache-control"], "no-store");
    let text = r.text().await.unwrap();
    assert!(
        !text.contains("secret") && !text.contains("8.8.8.8") && !text.contains(&created[1].id)
    );
    let page: JobPage = serde_json::from_str(&text).unwrap();
    assert!(page.has_active);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].title, "Audio import");
    assert_eq!(page.items[0].status, JobStatus::Queued);
    assert!(page.items[0].can_cancel && !page.items[0].can_retry);
    let path = format!("/api/imports/{id}/cancel");
    assert_eq!(
        app.client
            .post(format!("{}{path}", app.base))
            .header("Cookie", &app.admin.cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_ne!(
        app.request(Method::POST, &path, &member)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.request(Method::POST, &path, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let page: JobPage = app
        .request(
            Method::GET,
            "/api/jobs?state=cancelled&kind=import",
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!page.has_active);
    assert_eq!(page.items[0].status, JobStatus::Cancelled);
    assert!(!page.items[0].can_cancel && page.items[0].can_retry);
    let path = format!("/api/imports/{id}/retry");
    assert_eq!(
        app.request(Method::POST, &path, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let filtered: JobPage = app
        .request(Method::GET, "/api/jobs?state=completed", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(filtered.items.is_empty());
    assert!(
        filtered.has_active,
        "filter must not conceal activity requiring polling"
    );
    let private: JobPage = app
        .request(Method::GET, "/api/jobs", &member)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(private.items.len(), 1);
    assert_eq!(private.items[0].id, created[1].id);
    assert!(private.has_active);
}

#[tokio::test]
async fn job_center_follows_real_scan_progress_cancel_and_new_pass_history() {
    let app = TestApp::with_scanner(1024 * 1024, true).await;
    let source: ScanSource = app
        .request(Method::POST, "/api/scan-sources", &app.admin)
        .json(&CreateScanSource {
            name: "Background archive".into(),
            root_id: "fixture".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let path = format!("/api/scan-sources/{}/scan", source.id);
    let first: ScanJob = app
        .request(Method::POST, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let queued: JobPage = app
        .request(Method::GET, "/api/jobs?kind=scan", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(queued.has_active && queued.items[0].can_cancel);
    assert_eq!(
        queued.items[0].source_id.as_deref(),
        Some(source.id.as_str())
    );
    assert_eq!(queued.items[0].title, "Background archive");
    async fn finish_scan(app: &TestApp, id: &str) -> BackgroundJob {
        let shutdown = tokio_util::sync::CancellationToken::new();
        let scanner = app.state.scanner.clone();
        let token = shutdown.clone();
        let worker = tokio::spawn(async move { scanner.run(token).await });
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
        let completed = loop {
            let page: JobPage = app
                .request(Method::GET, "/api/jobs?kind=scan", &app.admin)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let job = page.items.into_iter().find(|j| j.id == id).unwrap();
            if job.status == JobStatus::Completed {
                assert!(!page.has_active);
                break job;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "scan timed out: {job:?}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        };
        shutdown.cancel();
        worker.await.unwrap().unwrap();
        completed
    }
    let completed = finish_scan(&app, &first.id).await;
    assert_eq!(
        completed.progress,
        JobProgress::Scan {
            visited: 3,
            imported: 2,
            duplicates: 0,
            unchanged: 0,
            skipped: 0,
            failed_files: 0,
        }
    );
    assert!(completed.can_rescan && !completed.can_cancel && !completed.can_retry);
    let second: ScanJob = app
        .request(Method::POST, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_ne!(second.id, first.id);
    let page: JobPage = app
        .request(Method::GET, "/api/jobs?state=completed", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(page.has_active);
    assert_eq!(page.items[0].id, first.id);
    assert!(
        !page.items[0].can_rescan,
        "an active pass suppresses all same-source rescan actions"
    );
    assert_eq!(
        app.request(
            Method::POST,
            &format!("/api/scans/{}/cancel", second.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NO_CONTENT
    );
    let third: ScanJob = app
        .request(Method::POST, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let completed = finish_scan(&app, &third.id).await;
    assert!(matches!(
        completed.progress,
        JobProgress::Scan {
            unchanged: 2,
            imported: 0,
            ..
        }
    ));
    let first_page: JobPage = app
        .request(Method::GET, "/api/jobs?limit=1", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut found = vec![first_page.items[0].id.clone()];
    let mut cursor = first_page.next_cursor;
    while let Some(next) = cursor {
        let page: JobPage = app
            .request(Method::GET, "/api/jobs", &app.admin)
            .query(&[("limit", "1"), ("cursor", &next)])
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        found.extend(page.items.into_iter().map(|job| job.id));
        cursor = page.next_cursor;
    }
    found.sort();
    let mut expected = vec![first.id, second.id, third.id];
    expected.sort();
    assert_eq!(found, expected);
    let tracks: TrackPage = app
        .request(Method::GET, "/api/tracks", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        tracks.items.len(),
        2,
        "new scan passes do not duplicate the library"
    );
}
#[tokio::test]
async fn upload_extract_browse_artwork_stream_and_deduplicate() {
    let app = TestApp::new().await;
    let u = app.upload(&app.admin).await;
    assert!(!u.duplicate);
    assert!(u.track.duration_ms > 0);
    assert_eq!(u.track.codec.to_lowercase(), "flac");
    assert!(u.track.artwork_id.is_some());
    let second = app.upload(&app.admin).await;
    assert!(second.duplicate);
    assert_eq!(u.track.id, second.track.id);
    let r = app
        .request(Method::GET, "/api/tracks", &app.admin)
        .send()
        .await
        .unwrap();
    let page: TrackPage = r.json().await.unwrap();
    assert_eq!(page.items.len(), 1);
    let r = app
        .request(
            Method::GET,
            &format!("/api/artwork/{}", u.track.artwork_id.unwrap()),
            &app.admin,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["content-type"], "image/jpeg");
    assert!(r.bytes().await.unwrap().len() > 100);
    let r = app
        .request(
            Method::GET,
            &format!("/api/tracks/{}/stream", u.track.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(&r.bytes().await.unwrap()[..], AUDIO);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_assets")
            .fetch_one(&app.state.pool)
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn http_range_suffix_open_head_conditionals_and_unsatisfiable() {
    let app = TestApp::new().await;
    let u = app.upload(&app.admin).await;
    let path = format!("/api/tracks/{}/stream", u.track.id);
    for (range, start, end) in [
        ("bytes=0-15", 0, 15),
        ("bytes=16-", 16, AUDIO.len() - 1),
        ("bytes=-12", AUDIO.len() - 12, AUDIO.len() - 1),
    ] {
        let r = app
            .request(Method::GET, &path, &app.admin)
            .header("Range", range)
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            r.headers()["content-range"],
            format!("bytes {start}-{end}/{}", AUDIO.len())
        );
        assert_eq!(&r.bytes().await.unwrap()[..], &AUDIO[start..=end]);
    }
    let r = app
        .request(Method::GET, &path, &app.admin)
        .header("Range", "bytes=99999999-")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        r.headers()["content-range"],
        format!("bytes */{}", AUDIO.len())
    );
    let r = app
        .request(Method::HEAD, &path, &app.admin)
        .header("Range", "bytes=0-15")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["content-length"], AUDIO.len().to_string());
    assert_eq!(r.bytes().await.unwrap().len(), 0);
    let r = app
        .request(Method::GET, &path, &app.admin)
        .send()
        .await
        .unwrap();
    let etag = r.headers()["etag"].to_str().unwrap().to_string();
    drop(r);
    let r = app
        .request(Method::GET, &path, &app.admin)
        .header("If-None-Match", &etag)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
    let r = app
        .request(Method::GET, &path, &app.admin)
        .header("If-None-Match", format!("W/{etag}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
    let r = app
        .request(Method::GET, &path, &app.admin)
        .header("Range", "seconds=0-15")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let r = app
        .request(Method::GET, &path, &app.admin)
        .header("If-Range", "\"old\"")
        .header("Range", "bytes=0-15")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let r = app
        .request(Method::GET, &path, &app.admin)
        .header("Range", "bytes=0-1,3-4")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}
#[tokio::test]
async fn authentication_csrf_private_catalog_and_artwork_are_enforced() {
    let app = TestApp::new().await;
    let u = app.upload(&app.admin).await;
    let other = app.user("listener").await;
    let page: TrackPage = app
        .request(Method::GET, "/api/tracks", &other)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(page.items.is_empty());
    for path in [
        format!("/api/tracks/{}/stream", u.track.id),
        format!("/api/artwork/{}", u.track.artwork_id.unwrap()),
    ] {
        assert_eq!(
            app.request(Method::GET, &path, &other)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            app.client
                .get(format!("{}{path}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let path = format!("/api/tracks/{}/favorite", u.track.id);
    let r = app
        .client
        .post(format!("{}{path}", app.base))
        .header("Cookie", &app.admin.cookie)
        .json(&json!({"favorite":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = app
        .request(Method::POST, &path, &app.admin)
        .header("Origin", "https://evil.example")
        .json(&json!({"favorite":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = app
        .request(Method::POST, "/api/users", &other)
        .json(&Credentials {
            username: "intruder".into(),
            password: "fixture password 12345".into(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = app
        .request(Method::POST, "/api/logout", &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        app.request(Method::GET, "/api/me", &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn favorites_search_and_shared_media_identity_remain_owner_isolated() {
    let app = TestApp::new().await;
    let u = app.upload(&app.admin).await;
    let other = app.user("listener").await;
    let other_track = app.upload(&other).await;
    assert_ne!(u.track.id, other_track.track.id);
    assert!(!other_track.duplicate);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_assets")
            .fetch_one(&app.state.pool)
            .await
            .unwrap(),
        1
    );
    let r = app
        .request(
            Method::POST,
            &format!("/api/tracks/{}/favorite", u.track.id),
            &app.admin,
        )
        .json(&json!({"favorite":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    let page: TrackPage = app
        .request(Method::GET, "/api/tracks?favorites=true", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.items[0].favorite);
    let page: TrackPage = app
        .request(Method::GET, "/api/tracks?favorites=true", &other)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(page.items.is_empty());
    for q in ["Signal", "\" OR 1=1 --", "NEAR(\"anything\")"] {
        let r = app
            .request(Method::GET, "/api/tracks", &app.admin)
            .query(&[("q", q)])
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }
}
#[tokio::test]
async fn playlist_rename_and_delete_are_owner_cas_idempotent_and_revoke_metadata() {
    let app = TestApp::new().await;
    let track = app.upload(&app.admin).await.track;
    let editor = app.user("playlist_editor").await;
    let outsider = app.user("playlist_outsider").await;
    let playlist: PlaylistSummary = app
        .request(Method::POST, "/api/playlists", &app.admin)
        .json(&CreatePlaylist {
            name: "Unnamed session".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let path = format!("/api/playlists/{}", playlist.id);
    let added: PlaylistDetail = app
        .request(Method::POST, &format!("{path}/items"), &app.admin)
        .json(&json!({"revision":0,"mutation_id":uuid::Uuid::new_v4().to_string(),"track_id":track.id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(added.playlist.revision, 1);
    app.request(Method::POST, &format!("{path}/collaborators"), &app.admin)
        .json(&CollaboratorRequest {
            username: editor.user.username.clone(),
            can_edit: true,
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let share: ShareResponse = app
        .request(Method::POST, &format!("{path}/shares"), &app.admin)
        .json(&ShareRequest {
            expires_in_seconds: 3600,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        app.client
            .get(format!("{}/api/shares/{}", app.base, share.token))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let rename = UpdatePlaylist {
        revision: 1,
        mutation_id: uuid::Uuid::new_v4().to_string(),
        name: "  Renamed session  ".into(),
    };
    assert_eq!(
        app.client
            .patch(format!("{}{path}", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&rename)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN,
        "cookie-authenticated rename requires the per-session CSRF header"
    );
    for actor in [&editor, &outsider] {
        assert_eq!(
            app.request(Method::PATCH, &path, actor)
                .json(&rename)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND,
            "rename is owner-only even for an editor grant"
        );
    }
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&json!({"revision":1,"mutation_id":uuid::Uuid::new_v4().to_string(),"name":"X","owner_id":outsider.user.id}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&UpdatePlaylist {
                revision: 1,
                mutation_id: uuid::Uuid::new_v4().to_string(),
                name: "   ".into(),
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&UpdatePlaylist {
                revision: 0,
                mutation_id: uuid::Uuid::new_v4().to_string(),
                name: "Stale".into(),
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let renamed: PlaylistSummary = app
        .request(Method::PATCH, &path, &app.admin)
        .json(&rename)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(renamed.name, "Renamed session");
    assert_eq!(renamed.revision, 2);
    assert_eq!(renamed.item_count, 1);
    assert!(renamed.can_edit);
    let replay: PlaylistSummary = app
        .request(Method::PATCH, &path, &app.admin)
        .json(&rename)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        replay.revision, 2,
        "identical replay must not increment again"
    );
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&UpdatePlaylist {
                revision: 2,
                mutation_id: rename.mutation_id.clone(),
                name: "Different input".into(),
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let editor_view: PlaylistDetail = app
        .request(Method::GET, &path, &editor)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(editor_view.playlist.name, "Renamed session");
    assert!(editor_view.playlist.can_edit);
    assert!(
        !editor_view.items[0].can_stream,
        "metadata access still grants no audio"
    );

    let delete = format!("{path}?revision=1&mutation_id={}", uuid::Uuid::new_v4());
    assert_eq!(
        app.request(Method::DELETE, &delete, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let delete = format!("{path}?revision=2&mutation_id={}", uuid::Uuid::new_v4());
    assert_eq!(
        app.request(Method::DELETE, &delete, &outsider)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.request(Method::DELETE, &delete, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    for actor in [&app.admin, &editor, &outsider] {
        assert_eq!(
            app.request(Method::GET, &path, actor)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND,
            "deleted playlist must not be readable through any grant"
        );
    }
    assert_eq!(
        app.client
            .get(format!("{}/api/shares/{}", app.base, share.token))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND,
        "deletion revokes existing metadata share links"
    );
    assert_eq!(
        app.request(Method::DELETE, &delete, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND,
        "a committed delete is not resurrected by a replay"
    );
    let page: TrackPage = app
        .request(Method::GET, "/api/tracks?limit=10", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        page.items.len(),
        1,
        "deleting a playlist must not delete owned tracks"
    );
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", track.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::OK,
        "owned original streaming is independent of playlist metadata"
    );
}
#[tokio::test]
async fn collaboration_conflicts_idempotency_reordering_sharing_and_revocation() {
    let app = TestApp::new().await;
    let u = app.upload(&app.admin).await;
    let other = app.user("listener").await;
    let p: PlaylistSummary = app
        .request(Method::POST, "/api/playlists", &app.admin)
        .json(&CreatePlaylist {
            name: "Evening signals".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let path = format!("/api/playlists/{}", p.id);
    assert_eq!(
        app.request(Method::GET, &path, &other)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let m = PlaylistMutation {
        revision: 0,
        mutation_id: uuid::Uuid::new_v4().to_string(),
        track_id: Some(u.track.id.clone()),
        item_ids: None,
    };
    let added: PlaylistDetail = app
        .request(Method::POST, &format!("{path}/items"), &app.admin)
        .json(&m)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(added.playlist.revision, 1);
    let again: PlaylistDetail = app
        .request(Method::POST, &format!("{path}/items"), &app.admin)
        .json(&m)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(again.items.len(), 1);
    assert_eq!(again.playlist.revision, 1);
    let r = app
        .request(Method::POST, &format!("{path}/collaborators"), &app.admin)
        .json(&CollaboratorRequest {
            username: "listener".into(),
            can_edit: true,
        })
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    let list: Vec<PlaylistCollaborator> = app
        .request(Method::GET, &format!("{path}/collaborators"), &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].username, "listener");
    assert!(list[0].can_edit);
    assert_eq!(
        app.request(Method::GET, &format!("{path}/collaborators"), &other)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let detail: PlaylistDetail = app
        .request(Method::GET, &path, &other)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(detail.playlist.can_edit);
    assert!(!detail.items[0].can_stream);
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", u.track.id),
            &other
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    let other_track = app.upload(&other).await;
    let a = PlaylistMutation {
        revision: 1,
        mutation_id: uuid::Uuid::new_v4().to_string(),
        track_id: Some(u.track.id.clone()),
        item_ids: None,
    };
    let b = PlaylistMutation {
        revision: 1,
        mutation_id: uuid::Uuid::new_v4().to_string(),
        track_id: Some(other_track.track.id),
        item_ids: None,
    };
    let (ra, rb) = tokio::join!(
        app.request(Method::POST, &format!("{path}/items"), &app.admin)
            .json(&a)
            .send(),
        app.request(Method::POST, &format!("{path}/items"), &other)
            .json(&b)
            .send()
    );
    let codes = [ra.unwrap().status(), rb.unwrap().status()];
    assert!(codes.contains(&StatusCode::OK));
    assert!(codes.contains(&StatusCode::CONFLICT));
    let detail: PlaylistDetail = app
        .request(Method::GET, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail.items.len(), 2);
    assert_eq!(detail.playlist.revision, 2);
    let (actor, mut retry) = if codes[0] == StatusCode::CONFLICT {
        (&app.admin, a)
    } else {
        (&other, b)
    };
    retry.revision = 2;
    assert_eq!(
        app.request(Method::POST, &format!("{path}/items"), actor)
            .json(&retry)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let detail: PlaylistDetail = app
        .request(Method::GET, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail.items.len(), 3);
    assert_eq!(detail.playlist.revision, 3);
    let order = PlaylistMutation {
        revision: 3,
        mutation_id: uuid::Uuid::new_v4().to_string(),
        track_id: None,
        item_ids: Some(detail.items.iter().rev().map(|i| i.id.clone()).collect()),
    };
    let reordered: PlaylistDetail = app
        .request(Method::PUT, &format!("{path}/order"), &other)
        .json(&order)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reordered.items[0].id, detail.items[2].id);
    let share: ShareResponse = app
        .request(Method::POST, &format!("{path}/shares"), &app.admin)
        .json(&ShareRequest {
            expires_in_seconds: 60,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let expired = app
        .state
        .playlists
        .share(
            &app.admin.user,
            &p.id,
            ShareRequest {
                expires_in_seconds: 60,
            },
        )
        .await
        .unwrap();
    sqlx::query("UPDATE playlist_shares SET expires_at=0 WHERE id=?")
        .bind(&expired.id)
        .execute(&app.state.pool)
        .await
        .unwrap();
    assert_eq!(
        app.client
            .get(format!("{}/api/shares/{}", app.base, expired.token))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let share_path = format!("/api/shares/{}", share.token);
    let r = app
        .client
        .get(format!("{}{share_path}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let public: PlaylistDetail = r.json().await.unwrap();
    assert!(public.items.iter().all(|i| !i.can_stream));
    let own_ids = detail
        .items
        .iter()
        .filter(|i| i.can_stream)
        .map(|i| &i.track.id)
        .collect::<Vec<_>>();
    assert!(public.items.iter().all(|i| own_ids.contains(&&i.track.id)));
    let r = app
        .request(Method::DELETE, &format!("{path}/shares"), &app.admin)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        app.client
            .get(format!("{}{share_path}", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    app.request(
        Method::DELETE,
        &format!("{path}/collaborators/{}", other.user.id),
        &app.admin,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(
        app.request(Method::GET, &path, &other)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn malicious_imports_and_invalid_uploads_are_rejected() {
    let app = TestApp::new().await;
    for url in [
        "http://127.0.0.1/secret",
        "http://[::1]/",
        "http://169.254.169.254/latest/meta-data/",
        "http://2130706433/",
        "http://[::ffff:127.0.0.1]/",
        "http://user:pass@example.org/audio.mp3",
        "ftp://example.org/audio.mp3",
        "http://example.org:8080/audio.mp3",
    ] {
        let r = app
            .request(Method::POST, "/api/imports/preview", &app.admin)
            .json(&ImportRequest { url: url.into() })
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST, "URL {url}");
        let body: ApiError = r.json().await.unwrap();
        assert!(!body.error.contains("user:pass"));
    }
    let fake = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(b"this is not FLAC".to_vec()).file_name("music.flac"),
    );
    let r = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(fake)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let large = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(vec![0u8; 1024 * 1024 + 1]).file_name("large.flac"),
    );
    let r = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(large)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let stats: LibraryStats = app
        .request(Method::GET, "/api/stats", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stats.tracks, 0);
    let mut entries = tokio::fs::read_dir(app.state.config.data_dir.join("staging"))
        .await
        .unwrap();
    while let Some(e) = entries.next_entry().await.unwrap() {
        assert!(!e.file_name().to_string_lossy().starts_with("upload-"));
    }
}
#[tokio::test]
async fn login_rate_limit_is_bounded_and_bootstrap_is_single_use() {
    let app = TestApp::new().await;
    let c = Credentials {
        username: "admin".into(),
        password: "fixture password 12345".into(),
    };
    assert_eq!(
        app.client
            .post(format!("{}/api/setup", app.base))
            .json(&c)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let bad = Credentials {
        username: "admin".into(),
        password: "x".repeat(129),
    };
    let mut last = StatusCode::OK;
    for _ in 0..16 {
        last = app
            .client
            .post(format!("{}/api/login", app.base))
            .json(&bad)
            .send()
            .await
            .unwrap()
            .status();
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}
#[tokio::test]
async fn database_and_media_survive_reopen_and_abandoned_uploads_are_cleaned() {
    let mut app = TestApp::new().await;
    let u = app.upload(&app.admin).await;
    tokio::fs::write(
        app.state
            .config
            .data_dir
            .join("staging/upload-abandoned.part"),
        b"unfinished",
    )
    .await
    .unwrap();
    app.task.abort();
    app.state.pool.close().await;
    let config = app.state.config.clone();
    let owner = app.admin.user.id.clone();
    let _dir = app._dir.take();
    drop(app);
    tokio::task::yield_now().await;
    let reopened = AppState::initialize(config.clone()).await.unwrap();
    let track = reopened
        .media
        .get_track(&owner, &u.track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(track.id, u.track.id);
    let asset = reopened
        .media
        .asset(&owner, &track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tokio::fs::read(asset.path).await.unwrap(), AUDIO);
    assert!(
        !config
            .data_dir
            .join("staging/upload-abandoned.part")
            .exists()
    );
    assert!(!reopened.auth.needs_setup().await.unwrap());
}

#[tokio::test]
async fn backup_restores_real_audio_metadata_accounts_and_sessions() {
    let app = TestApp::new().await;
    let uploaded = app.upload(&app.admin).await;
    let backup_dir = tempfile::tempdir().unwrap();
    let archive = backup_dir.path().join("backup.tar.gz");
    let restored = backup_dir.path().join("restored");
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/backup.py");
    let output = tokio::process::Command::new("python3")
        .arg(&script)
        .args(["create", "--data-dir"])
        .arg(&app.state.config.data_dir)
        .arg("--output")
        .arg(&archive)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = tokio::process::Command::new("python3")
        .arg(&script)
        .args(["restore", "--archive"])
        .arg(&archive)
        .arg("--data-dir")
        .arg(&restored)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = Config {
        scan_roots: Vec::new(),
        data_dir: restored,
        ..app.state.config.clone()
    };
    let restored = AppState::initialize(config).await.unwrap();
    let track = restored
        .media
        .get_track(&app.admin.user.id, &uploaded.track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(track.title, uploaded.track.title);
    let asset = restored
        .media
        .asset(&app.admin.user.id, &track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tokio::fs::read(asset.path).await.unwrap(), AUDIO);
    let session = restored
        .auth
        .session(app.admin.cookie.split_once('=').unwrap().1)
        .await
        .unwrap();
    assert_eq!(session.response.user, app.admin.user);
}

#[tokio::test]
async fn data_directory_lock_prevents_competing_recovery() {
    let app = TestApp::new().await;
    assert!(
        AppState::initialize(app.state.config.clone())
            .await
            .is_err()
    );
    assert!(app.state.media.stats(&app.admin.user.id).await.is_ok());
}

#[tokio::test]
async fn viewer_cannot_edit_manage_access_or_play_and_can_leave() {
    let app = TestApp::new().await;
    let viewer = app.user("viewer").await;
    let playlist = app
        .state
        .playlists
        .create(
            &app.admin.user,
            CreatePlaylist {
                name: "Read only".into(),
            },
        )
        .await
        .unwrap();
    app.state
        .playlists
        .collaborate(
            &app.admin.user,
            &playlist.id,
            CollaboratorRequest {
                username: "viewer".into(),
                can_edit: false,
            },
        )
        .await
        .unwrap();
    let path = format!("/api/playlists/{}", playlist.id);
    assert_eq!(
        app.request(Method::GET, &path, &viewer)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let mutation = PlaylistMutation {
        revision: 0,
        mutation_id: uuid::Uuid::new_v4().to_string(),
        track_id: None,
        item_ids: Some(Vec::new()),
    };
    assert_eq!(
        app.request(Method::PUT, &format!("{path}/order"), &viewer)
            .json(&mutation)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.request(Method::POST, &format!("{path}/collaborators"), &viewer)
            .json(&CollaboratorRequest {
                username: "admin".into(),
                can_edit: true
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.request(Method::POST, &format!("{path}/shares"), &viewer)
            .json(&ShareRequest {
                expires_in_seconds: 60
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.request(
            Method::DELETE,
            &format!("{path}/collaborators/{}", app.admin.user.id),
            &viewer
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.request(
            Method::DELETE,
            &format!("{path}/collaborators/{}", viewer.user.id),
            &viewer
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.request(Method::GET, &path, &viewer)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.request(Method::GET, &path, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn cancelling_large_streams_releases_capacity_while_uploads_and_scans_continue() {
    let app = TestApp::with_scanner(12 * 1024 * 1024, true).await;
    let payload_bytes = 10 * 1024 * 1024_u32;
    let mut wave = Vec::with_capacity(payload_bytes as usize + 44);
    wave.extend_from_slice(b"RIFF");
    wave.extend_from_slice(&(payload_bytes + 36).to_le_bytes());
    wave.extend_from_slice(b"WAVEfmt ");
    wave.extend_from_slice(&16u32.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes());
    wave.extend_from_slice(&44100u32.to_le_bytes());
    wave.extend_from_slice(&88200u32.to_le_bytes());
    wave.extend_from_slice(&2u16.to_le_bytes());
    wave.extend_from_slice(&16u16.to_le_bytes());
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&payload_bytes.to_le_bytes());
    wave.resize(payload_bytes as usize + 44, 0);
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(wave).file_name("long.wav"),
    );
    let r = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let uploaded: UploadResult = r.json().await.unwrap();
    let path = format!("/api/tracks/{}/stream", uploaded.track.id);
    let mut held = Vec::new();
    for _ in 0..16 {
        let r = app
            .request(Method::GET, &path, &app.admin)
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        held.push(r);
    }
    assert_eq!(
        app.request(Method::GET, &path, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let source: ScanSource = app
        .request(Method::POST, "/api/scan-sources", &app.admin)
        .json(&CreateScanSource {
            name: "Concurrent folder".into(),
            root_id: "fixture".into(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let scan: ScanJob = app
        .request(
            Method::POST,
            &format!("/api/scan-sources/{}/scan", source.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let scanner = app.state.scanner.clone();
    let shutdown = app.state.shutdown.clone();
    let worker = tokio::spawn(async move { scanner.run(shutdown).await });
    let (a, b) = tokio::join!(app.upload(&app.admin), app.upload(&app.admin));
    assert_eq!(a.track.id, b.track.id);
    assert!(a.duplicate || b.duplicate);
    // Finish real scanning/ingestion while all sixteen large response bodies
    // still hold stream permits. Shared parsing must not wait on playback.
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let jobs: Vec<ScanJob> = app
                .request(Method::GET, "/api/scans", &app.admin)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let current = jobs.iter().find(|job| job.id == scan.id).unwrap();
            assert_ne!(current.status, "failed", "{current:?}");
            if current.status == "completed" {
                assert_eq!(current.imported + current.duplicates, 2);
                assert_eq!(current.failed_files, 0);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    drop(held);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let r = app
                .request(Method::GET, &path, &app.admin)
                .header("Range", "bytes=0-15")
                .send()
                .await
                .unwrap();
            if r.status() == StatusCode::PARTIAL_CONTENT {
                assert_eq!(r.bytes().await.unwrap().len(), 16);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    app.state.shutdown.cancel();
    worker.await.unwrap().unwrap();
}

#[tokio::test]
async fn static_assets_never_serve_arbitrary_database_files() {
    let app = TestApp::new().await;
    tokio::fs::create_dir_all(&app.state.config.web_dir)
        .await
        .unwrap();
    tokio::fs::write(
        app.state.config.web_dir.join("private.db"),
        b"private fixture bytes",
    )
    .await
    .unwrap();
    tokio::fs::write(
        app.state.config.web_dir.join("now-playing.css"),
        b".now-playing-dialog{color:inherit}",
    )
    .await
    .unwrap();
    tokio::fs::write(
        app.state.config.web_dir.join("private.css"),
        b"private fixture bytes",
    )
    .await
    .unwrap();
    let stylesheet = app
        .client
        .get(format!("{}/now-playing.css", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(stylesheet.status(), StatusCode::OK);
    assert!(
        stylesheet.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/css")
    );
    assert!(
        stylesheet
            .text()
            .await
            .unwrap()
            .contains(".now-playing-dialog")
    );
    assert_eq!(
        app.client
            .get(format!("{}/private.css", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    tokio::fs::create_dir_all(app.state.config.web_dir.join("api"))
        .await
        .unwrap();
    tokio::fs::write(
        app.state.config.web_dir.join("api/private.db"),
        b"private fixture bytes",
    )
    .await
    .unwrap();
    assert_eq!(
        app.client
            .get(format!("{}/api/private.db", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    tokio::fs::write(
        app.state.config.web_dir.join("index.html"),
        b"<h1>Public asset fixture</h1>",
    )
    .await
    .unwrap();
    let r = app
        .client
        .get(format!("{}/private.db", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert!(!r.text().await.unwrap().contains("private fixture"));
    assert_eq!(
        app.client.get(&app.base).send().await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        app.client
            .get(format!("{}/pkg/%2e%2e/private.db", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn brand_assets_are_exactly_allowlisted_and_never_expose_their_folder() {
    let app = TestApp::new().await;
    let brand = app.state.config.web_dir.join("assets/brand");
    tokio::fs::create_dir_all(&brand).await.unwrap();
    tokio::fs::write(brand.join("vela-coast.png"), b"coast fixture bytes")
        .await
        .unwrap();
    tokio::fs::write(brand.join("vela-wave.png"), b"wave fixture bytes")
        .await
        .unwrap();
    tokio::fs::write(brand.join("SOURCE.md"), b"private provenance fixture")
        .await
        .unwrap();
    tokio::fs::write(brand.join("other.png"), b"unlisted fixture bytes")
        .await
        .unwrap();

    for name in ["vela-coast.png", "vela-wave.png"] {
        let response = app
            .client
            .get(format!("{}/assets/brand/{name}", app.base))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{name} must be served");
        assert!(!response.text().await.unwrap().is_empty());
    }
    // The provenance record and every unlisted neighbour stay denied: the
    // allowlist names exact files, never a directory.
    for denied in [
        "/assets/brand/SOURCE.md",
        "/assets/brand/other.png",
        "/assets/brand/",
    ] {
        let response = app
            .client
            .get(format!("{}{denied}", app.base))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{denied} must stay denied"
        );
        assert!(!response.text().await.unwrap().contains("fixture"));
    }
    // Traversal probes go over a raw socket: an HTTP client normalizes `..`
    // segments away before they reach the server, which would test the client
    // instead of the allowlist.
    for raw in [
        "/assets/brand/%2e%2e/brand/vela-coast.png",
        "/assets/brand%2fvela-coast.png",
        "/assets/brand/..%2fbrand/vela-coast.png",
    ] {
        let response = raw_request(&app.base, raw).await;
        assert!(
            response.starts_with("HTTP/1.1 404"),
            "{raw} must stay denied, got: {}",
            response.lines().next().unwrap_or_default()
        );
        assert!(!response.contains("fixture"));
    }
}

/// Send one request line verbatim so the path is not normalized by a client.
async fn raw_request(base: &str, path: &str) -> String {
    let authority = base.trim_start_matches("http://");
    let mut stream = tokio::net::TcpStream::connect(authority).await.unwrap();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

#[tokio::test]
async fn multipart_headers_and_additional_parts_are_rejected_before_publication() {
    let app = TestApp::new().await;
    let header = format!(
        "--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\r\naudio\r\n--boundary--\r\n",
        "x".repeat(100000)
    );
    let response = app
        .request(Method::POST, "/api/upload", &app.admin)
        .header("Content-Type", "multipart/form-data; boundary=boundary")
        .body(header)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // A preamble cannot trick the guard into ending headers before Multer starts them.
    let attack = format!(
        "preamble\r\n\r\nx--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\r\nignored\r\n--boundary--\r\n",
        "x".repeat(100_000)
    );
    let response = app
        .request(Method::POST, "/api/upload", &app.admin)
        .header("Content-Type", "multipart/form-data; boundary=boundary")
        .body(attack)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let mut body=b"--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"signal.flac\"\r\n\r\n".to_vec();
    body.extend_from_slice(AUDIO);
    body.extend_from_slice(b"\r\n--boundary\r\nContent-Disposition: form-data; name=\"other\"\r\n\r\nextra\r\n--boundary--\r\n");
    let response = app
        .request(Method::POST, "/api/upload", &app.admin)
        .header("Content-Type", "multipart/form-data; boundary=boundary")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        app.state
            .media
            .stats(&app.admin.user.id)
            .await
            .unwrap()
            .tracks,
        0
    );
}

#[tokio::test]
async fn album_and_artist_http_catalog_is_paginated_private_and_playable() {
    let app = TestApp::new().await;
    let first = app.upload(&app.admin).await.track;
    let second = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(OTHER_AUDIO.to_vec()).file_name("beacon.mp3"),
        ))
        .send()
        .await
        .unwrap()
        .json::<UploadResult>()
        .await
        .unwrap()
        .track;
    // Fixture MP3 has no date/number; exercise catalog regrouping without rewriting audio.
    sqlx::query("UPDATE tracks SET year=2026,track_number=2 WHERE id=?")
        .bind(&second.id)
        .execute(&app.state.pool)
        .await
        .unwrap();
    let albums: AlbumPage = app
        .request(Method::GET, "/api/albums?limit=1", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(albums.items.len(), 1);
    let album = &albums.items[0];
    assert_eq!(album.title, "Real Fixtures");
    assert_eq!(album.track_count, 2);
    assert_eq!(album.year, Some(2026));
    assert!(albums.next_cursor.is_none());
    assert!(album.artwork_id.is_some());
    let page: TrackPage = app
        .request(
            Method::GET,
            &format!("/api/albums/{}/tracks?limit=1", album.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, first.id);
    let next = page.next_cursor.unwrap();
    let page: TrackPage = app
        .request(
            Method::GET,
            &format!("/api/albums/{}/tracks?limit=1&cursor={next}", album.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, second.id);
    assert!(page.next_cursor.is_none());
    let artists: ArtistPage = app
        .request(Method::GET, "/api/artists?limit=1", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(artists.items.len(), 1);
    assert_eq!(artists.items[0].name, "Vela Test");
    assert_eq!(artists.items[0].track_count, 2);
    assert_eq!(artists.items[0].album_count, 1);
    let artist = &artists.items[0];
    let page: TrackPage = app
        .request(
            Method::GET,
            &format!("/api/artists/{}/tracks?limit=1", artist.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.next_cursor.is_some());
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", first.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap(),
        AUDIO
    );
    let other = app.user("otherlistener").await;
    for (kind, id) in [
        ("albums", album.id.as_str()),
        ("artists", artist.id.as_str()),
    ] {
        let response = app
            .request(Method::GET, &format!("/api/{kind}"), &other)
            .send()
            .await
            .unwrap();
        assert!(
            response.json::<serde_json::Value>().await.unwrap()["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        for path in [
            format!("/api/{kind}/{id}"),
            format!("/api/{kind}/{id}/tracks"),
        ] {
            assert_eq!(
                app.request(Method::GET, &path, &other)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
            assert_eq!(
                app.client
                    .get(format!("{}{path}", app.base))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::UNAUTHORIZED
            );
        }
        for cursor in ["not-a-uuid".to_string(), uuid::Uuid::new_v4().to_string()] {
            assert_eq!(
                app.request(
                    Method::GET,
                    &format!("/api/{kind}/{id}/tracks?cursor={cursor}"),
                    &app.admin
                )
                .send()
                .await
                .unwrap()
                .status(),
                StatusCode::BAD_REQUEST
            );
        }
    }
    let duplicate = app.upload(&app.admin).await;
    assert!(duplicate.duplicate);
    let detail: AlbumSummary = app
        .request(
            Method::GET,
            &format!("/api/albums/{}", album.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail.id, album.id);
    assert_eq!(detail.track_count, 2);
}

#[tokio::test]
async fn metadata_edits_require_owner_csrf_validate_input_and_preserve_original_audio() {
    let app = TestApp::new().await;
    let track = app.upload(&app.admin).await.track;
    let path = format!("/api/tracks/{}/metadata", track.id);
    let before: TrackMetadata = app
        .request(Method::GET, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(before.revision, 0);
    let update = UpdateTrackMetadata {
        revision: 0,
        title: "  Ocean signal  ".into(),
        artist: "Vela Test".into(),
        album: "Edited album".into(),
        album_artist: "Vela Test".into(),
        genre: "Ambient".into(),
        year: Some(2025),
        track_number: Some(2),
        disc_number: Some(1),
    };
    assert_eq!(
        app.client
            .patch(format!("{}{path}", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&update)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let listener = app.user("metadatalistener").await;
    for method in [Method::GET, Method::PATCH] {
        assert_eq!(
            app.request(method, &path, &listener)
                .json(&update)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let changed = app
        .request(Method::PATCH, &path, &app.admin)
        .json(&update)
        .send()
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::OK);
    let changed: TrackMetadata = changed.json().await.unwrap();
    assert_eq!(changed.revision, 1);
    assert_eq!(changed.track.title, "Ocean signal");
    assert_eq!(changed.track.album, "Edited album");
    assert_eq!(changed.track.codec, before.track.codec);
    assert_eq!(changed.track.duration_ms, before.track.duration_ms);
    assert_eq!(changed.track.artwork_id, before.track.artwork_id);
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&update)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let mut valid = serde_json::to_value(&update).unwrap();
    valid["revision"] = json!(1);
    for (field, value) in [
        ("title", json!("  ")),
        ("artist", json!("x".repeat(1025))),
        ("genre", json!("bad\nvalue")),
        ("year", json!(0)),
        ("disc_number", json!(-1)),
        ("track_number", json!(10000)),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        assert_eq!(
            app.request(Method::PATCH, &path, &app.admin)
                .json(&invalid)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST,
            "{field}"
        );
    }
    let mut unknown = valid.clone();
    unknown["storage_path"] = json!("/etc/passwd");
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&unknown)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("year");
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&missing)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    valid["title"] = json!("x".repeat(20_000));
    assert_eq!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&valid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let searched: TrackPage = app
        .request(Method::GET, "/api/tracks?q=Ocean", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(searched.items[0].id, track.id);
    let albums: AlbumPage = app
        .request(Method::GET, "/api/albums", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(albums.items.len(), 1);
    assert_eq!(albums.items[0].title, "Edited album");
    let bytes = app
        .request(
            Method::GET,
            &format!("/api/tracks/{}/stream", track.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(bytes, AUDIO);
    let audit:i64=sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE actor_id=? AND object_id=? AND action='track.metadata_updated'").bind(&app.admin.user.id).bind(&track.id).fetch_one(&app.state.pool).await.unwrap();
    assert_eq!(audit, 1);
}

#[tokio::test]
async fn simultaneous_metadata_edits_conflict_and_do_not_overwrite_each_other() {
    let app = TestApp::new().await;
    let track = app.upload(&app.admin).await.track;
    let path = format!("/api/tracks/{}/metadata", track.id);
    let edit = UpdateTrackMetadata {
        revision: 0,
        title: "One editor".into(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        album_artist: track.album_artist.clone(),
        genre: track.genre.clone(),
        year: track.year,
        track_number: track.track_number,
        disc_number: track.disc_number,
    };
    let mut another = edit.clone();
    another.title = "Another editor".into();
    let (first, second) = tokio::join!(
        app.request(Method::PATCH, &path, &app.admin)
            .json(&edit)
            .send(),
        app.request(Method::PATCH, &path, &app.admin)
            .json(&another)
            .send()
    );
    let first = first.unwrap();
    let second = second.unwrap();
    assert!(matches!(
        (first.status(), second.status()),
        (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::OK)
    ));
    let accepted: TrackMetadata = if first.status() == StatusCode::OK {
        first
    } else {
        second
    }
    .json()
    .await
    .unwrap();
    let final_snapshot: TrackMetadata = app
        .request(Method::GET, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(final_snapshot.revision, 1);
    assert_eq!(final_snapshot.track, accepted.track);
}

#[tokio::test]
async fn admin_folder_scan_imports_real_audio_without_paths_and_respects_private_permissions() {
    let app = TestApp::with_scanner(1024 * 1024, true).await;
    let member = app.user("scanlistener").await;
    for path in ["/api/storage/roots", "/api/scan-sources", "/api/scans"] {
        assert_eq!(
            app.request(Method::GET, path, &member)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            app.client
                .get(format!("{}{path}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let roots = app
        .request(Method::GET, "/api/storage/roots", &app.admin)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        !roots.contains("path")
            && !roots.contains(app._dir.as_ref().unwrap().path().to_str().unwrap())
    );
    let roots: Vec<StorageRoot> = serde_json::from_str(&roots).unwrap();
    assert_eq!(roots.len(), 1);
    assert!(roots[0].available);
    let source_input = CreateScanSource {
        name: "Archive collection".into(),
        root_id: "fixture".into(),
    };
    assert_eq!(
        app.client
            .post(format!("{}/api/scan-sources", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&source_input)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.request(Method::POST, "/api/scan-sources", &member)
            .json(&source_input)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut forged = serde_json::to_value(&source_input).unwrap();
    forged["path"] = json!("/etc");
    assert_eq!(
        app.request(Method::POST, "/api/scan-sources", &app.admin)
            .json(&forged)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let source: ScanSource = app
        .request(Method::POST, "/api/scan-sources", &app.admin)
        .json(&source_input)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let path = format!("/api/scan-sources/{}/scan", source.id);
    let job: ScanJob = app
        .request(Method::POST, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        app.request(Method::POST, &path, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    for path in [
        format!("/api/scans/{}/cancel", job.id),
        format!("/api/scan-sources/{}/scan", source.id),
    ] {
        assert_eq!(
            app.request(Method::POST, &path, &member)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        app.request(
            Method::DELETE,
            &format!("/api/scan-sources/{}", source.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::CONFLICT
    );
    let shutdown = app.state.shutdown.clone();
    let scanner = app.state.scanner.clone();
    let worker = tokio::spawn(async move { scanner.run(shutdown).await });
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let jobs: Vec<ScanJob> = app
            .request(Method::GET, "/api/scans", &app.admin)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let current = jobs.iter().find(|j| j.id == job.id).unwrap();
        if current.status == "completed" {
            assert_eq!(current.imported, 2);
            assert_eq!(current.failed_files, 0);
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "scan failed or timed out: {current:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let tracks: TrackPage = app
        .request(Method::GET, "/api/tracks", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tracks.items.len(), 2);
    let track = tracks.items.iter().find(|t| t.title == "Signal").unwrap();
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", track.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap(),
        AUDIO
    );
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", track.id),
            &member
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    let metadata: TrackMetadata = app
        .request(
            Method::GET,
            &format!("/api/tracks/{}/metadata", track.id),
            &app.admin,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let update = UpdateTrackMetadata {
        revision: metadata.revision,
        title: "Owner curated title".into(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        album_artist: track.album_artist.clone(),
        genre: track.genre.clone(),
        year: track.year,
        track_number: track.track_number,
        disc_number: track.disc_number,
    };
    assert_eq!(
        app.request(
            Method::PATCH,
            &format!("/api/tracks/{}/metadata", track.id),
            &app.admin
        )
        .json(&update)
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::OK
    );
    let next: ScanJob = app
        .request(Method::POST, &path, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    loop {
        let jobs: Vec<ScanJob> = app
            .request(Method::GET, "/api/scans", &app.admin)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let current = jobs.iter().find(|j| j.id == next.id).unwrap();
        if current.status == "completed" {
            assert_eq!(current.unchanged, 2);
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let source_audio = app
        ._dir
        .as_ref()
        .unwrap()
        .path()
        .join("music/album/signal.flac");
    assert_eq!(tokio::fs::read(source_audio).await.unwrap(), AUDIO);
    let tracks: TrackPage = app
        .request(Method::GET, "/api/tracks", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        tracks
            .items
            .iter()
            .any(|t| t.title == "Owner curated title")
    );
    assert_eq!(
        app.request(
            Method::DELETE,
            &format!("/api/scan-sources/{}", source.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NO_CONTENT
    );
    let tracks: TrackPage = app
        .request(Method::GET, "/api/tracks", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tracks.items.len(), 2);
    app.state.shutdown.cancel();
    worker.await.unwrap().unwrap();
}

#[tokio::test]
async fn typed_library_filters_facets_and_sorting_are_private_and_compose_with_search() {
    let app = TestApp::new().await;
    let member = app.user("filterlistener").await;
    let signal = app.upload(&app.admin).await.track;
    let private_signal = app.upload(&member).await.track;
    let beacon: UploadResult = app
        .request(Method::POST, "/api/upload", &app.admin)
        .multipart(reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(OTHER_AUDIO.to_vec()).file_name("beacon.mp3"),
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    for (owner, track, genre, year) in [
        (&app.admin, &signal, "Downtempo", 1998),
        (&app.admin, &beacon.track, "Rock", 2021),
        (&member, &private_signal, "Private-only genre", 1984),
    ] {
        let update = UpdateTrackMetadata {
            revision: 0,
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            album_artist: track.album_artist.clone(),
            genre: genre.into(),
            year: Some(year),
            track_number: track.track_number,
            disc_number: track.disc_number,
        };
        assert_eq!(
            app.request(
                Method::PATCH,
                &format!("/api/tracks/{}/metadata", track.id),
                owner
            )
            .json(&update)
            .send()
            .await
            .unwrap()
            .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        app.request(
            Method::POST,
            &format!("/api/tracks/{}/favorite", signal.id),
            &app.admin
        )
        .json(&json!({"favorite":true}))
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.client
            .get(format!("{}/api/catalog/filters", app.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .request(Method::GET, "/api/catalog/filters", &app.admin)
        .send()
        .await
        .unwrap();
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let facets: CatalogFacets = response.json().await.unwrap();
    assert_eq!(facets.genres, vec!["Downtempo", "Rock"]);
    assert!(!facets.genres_truncated && !facets.codecs_truncated);
    assert!(facets.codecs.contains(&signal.codec) && facets.codecs.contains(&beacon.track.codec));
    let facets: CatalogFacets = app
        .request(Method::GET, "/api/catalog/filters", &member)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(facets.genres, vec!["Private-only genre"]);
    let filtered: TrackPage = app
        .request(Method::GET, "/api/tracks", &app.admin)
        .query(&[
            ("q", "Signal"),
            ("genre", "downtempo"),
            ("codec", "flac"),
            ("year_from", "1990"),
            ("year_to", "2000"),
            ("min_bitrate", "1"),
            ("min_sample_rate", "44100"),
            ("min_bit_depth", "16"),
            ("favorites", "true"),
            ("sort", "title"),
        ])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].id, signal.id);
    assert!(filtered.next_cursor.is_none());
    let unknowns: TrackPage = app
        .request(Method::GET, "/api/tracks?min_bit_depth=1", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        unknowns.items.len(),
        1,
        "unknown MPEG bit depth must not match a quality threshold"
    );
    assert_eq!(unknowns.items[0].id, signal.id);
    for genre in ["Private-only genre", "' OR 1=1 --", "Rock%", "Rock_"] {
        let page: TrackPage = app
            .request(Method::GET, "/api/tracks", &app.admin)
            .query(&[("genre", genre)])
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(
            page.items.is_empty(),
            "genre is an owner-scoped literal, never SQL or LIKE syntax"
        );
    }
    let first: TrackPage = app
        .request(Method::GET, "/api/tracks?sort=title&limit=1", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first.items[0].id, beacon.track.id);
    let second: TrackPage = app
        .request(Method::GET, "/api/tracks?sort=title&limit=1", &app.admin)
        .query(&[("cursor", first.next_cursor.as_deref().unwrap())])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second.items[0].id, signal.id);
    assert!(second.next_cursor.is_none());
    for sort in ["title", "newest"] {
        let mut refusals = Vec::new();
        for cursor in [private_signal.id.clone(), uuid::Uuid::new_v4().to_string()] {
            let response = app
                .request(Method::GET, "/api/tracks", &app.admin)
                .query(&[
                    ("sort", sort),
                    ("q", "DefinitelyAbsent"),
                    ("cursor", cursor.as_str()),
                ])
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            refusals.push(response.json::<serde_json::Value>().await.unwrap());
        }
        assert_eq!(
            refusals[0], refusals[1],
            "foreign/missing cursor errors are indistinguishable"
        );
    }
    // Known equal timestamps exercise the UUID tie breaker through real HTTP.
    sqlx::query("UPDATE tracks SET added_at=1234 WHERE owner_id=?")
        .bind(&app.admin.user.id)
        .execute(&app.state.pool)
        .await
        .unwrap();
    let first: TrackPage = app
        .request(Method::GET, "/api/tracks?sort=newest&limit=1", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let second: TrackPage = app
        .request(Method::GET, "/api/tracks?sort=newest&limit=1", &app.admin)
        .query(&[("cursor", first.next_cursor.as_deref().unwrap())])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(first.items[0].id > second.items[0].id);
    assert!(second.next_cursor.is_none());
    assert_ne!(first.items[0].id, private_signal.id);
    assert_ne!(second.items[0].id, private_signal.id);
}

#[tokio::test]
async fn malformed_library_filter_queries_are_bounded_json_errors() {
    let app = TestApp::new().await;
    for query in [
        "sort=owner_id",
        "year_from=0",
        "year_to=10000",
        "year_from=2025&year_to=1990",
        "year_from=not-a-number",
        "min_bitrate=0",
        "min_bitrate=1000001",
        "min_sample_rate=-1",
        "min_sample_rate=1536001",
        "min_bit_depth=65",
        "genre=%00",
        "genre=",
        "codec=%0A",
        "codec=",
        "cursor=not-a-uuid",
        "filter_sql=1",
        "limit=4294967296",
        "sort=title&sort=newest",
    ] {
        let response = app
            .request(Method::GET, &format!("/api/tracks?{query}"), &app.admin)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
        let error: serde_json::Value = response.json().await.unwrap();
        assert!(
            error["error"]
                .as_str()
                .is_some_and(|message| message.len() < 256)
        );
    }
    for (field, size) in [("q", 257), ("genre", 1025), ("codec", 65), ("cursor", 129)] {
        let value = "a".repeat(size);
        let response = app
            .request(Method::GET, "/api/tracks", &app.admin)
            .query(&[(field, value)])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.json::<serde_json::Value>().await.unwrap()["error"].is_string());
    }
}

#[path = "support/publications.rs"]
mod publication_tests;

#[tokio::test]
async fn listening_device_grants_are_listed_and_revoked_with_revision_cas() {
    let app = TestApp::new().await;
    let owned = app.upload(&app.admin).await.track;
    let device_a = uuid::Uuid::new_v4().to_string();
    let device_b = uuid::Uuid::new_v4().to_string();
    let enable_a = UpdateListeningPreferences {
        revision: 0,
        enabled: true,
        retention_days: 90,
        device_id: device_a.clone(),
        device_enabled: true,
    };
    let rev1: ListeningPreferences = app
        .request(Method::PUT, "/api/listening/preferences", &app.admin)
        .json(&enable_a)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let batch_a = ListeningBatch {
        device_id: device_a.clone(),
        revision: rev1.revision,
        events: vec![ListeningProgress {
            session_id: uuid::Uuid::new_v4().to_string(),
            sequence: 1,
            track_id: owned.id.clone(),
            listened_ms: 1500,
            position_ms: 1500,
            started_at: vela_core::now() - 2,
            ended: true,
        }],
    };
    assert_eq!(
        app.request(Method::POST, "/api/listening/events", &app.admin)
            .json(&batch_a)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let enable_b = UpdateListeningPreferences {
        revision: rev1.revision,
        enabled: true,
        retention_days: 30,
        device_id: device_b.clone(),
        device_enabled: true,
    };
    let rev2: ListeningPreferences = app
        .request(Method::PUT, "/api/listening/preferences", &app.admin)
        .json(&enable_b)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let batch_b = ListeningBatch {
        device_id: device_b.clone(),
        revision: rev2.revision,
        events: vec![ListeningProgress {
            session_id: uuid::Uuid::new_v4().to_string(),
            sequence: 1,
            track_id: owned.id,
            listened_ms: 2500,
            position_ms: 2500,
            started_at: vela_core::now() - 1,
            ended: true,
        }],
    };
    assert_eq!(
        app.request(Method::POST, "/api/listening/events", &app.admin)
            .json(&batch_b)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let listed: ListeningDevices = app
        .request(Method::GET, "/api/listening/devices", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed.revision, rev2.revision);
    assert_eq!(listed.devices.len(), 2);
    assert!(
        listed
            .devices
            .iter()
            .all(|device| device.session_count == 1)
    );
    assert!(
        listed
            .devices
            .iter()
            .all(|device| device.last_activity_at.is_some())
    );
    let stale = format!(
        "/api/listening/devices/{device_a}?revision={}",
        rev1.revision
    );
    assert_eq!(
        app.request(Method::DELETE, &stale, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let current = format!(
        "/api/listening/devices/{device_a}?revision={}",
        rev2.revision
    );
    let after: ListeningDevices = app
        .request(Method::DELETE, &current, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(after.revision > rev2.revision);
    assert_eq!(after.devices.len(), 1);
    assert_eq!(after.devices[0].device_id, device_b);
    let export: ListeningExport = app
        .request(Method::GET, "/api/listening/export", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(export.events.len(), 1);
    assert_eq!(export.events[0].device_id, device_b);
    let missing = format!(
        "/api/listening/devices/{device_a}?revision={}",
        after.revision
    );
    assert_eq!(
        app.request(Method::DELETE, &missing, &app.admin)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn playlist_folders_are_owner_scoped_and_move_playlists_with_cas() {
    let app = TestApp::new().await;
    let folder_a: PlaylistFolder = app
        .request(Method::POST, "/api/playlist-folders", &app.admin)
        .json(&CreatePlaylistFolder {
            name: "Evening".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(folder_a.playlist_count, 0);
    assert_eq!(folder_a.revision, 0);
    assert_eq!(
        app.request(Method::POST, "/api/playlist-folders", &app.admin)
            .json(&CreatePlaylistFolder {
                name: "evening".into(),
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let folder_b: PlaylistFolder = app
        .request(Method::POST, "/api/playlist-folders", &app.admin)
        .json(&CreatePlaylistFolder {
            name: "Morning".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let playlist: PlaylistSummary = app
        .request(Method::POST, "/api/playlists", &app.admin)
        .json(&CreatePlaylist {
            name: "Folder trip".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let moved: PlaylistSummary = app
        .request(
            Method::PATCH,
            &format!("/api/playlists/{}/folder", playlist.id),
            &app.admin,
        )
        .json(&MovePlaylist {
            revision: 0,
            folder_id: Some(folder_a.id.clone()),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(moved.folder_id.as_deref(), Some(folder_a.id.as_str()));
    assert_eq!(moved.revision, 1);
    assert_eq!(
        app.request(
            Method::PATCH,
            &format!("/api/playlists/{}/folder", playlist.id),
            &app.admin
        )
        .json(&MovePlaylist {
            revision: 0,
            folder_id: Some(folder_b.id.clone()),
        })
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::CONFLICT
    );
    let renamed: PlaylistFolder = app
        .request(
            Method::PATCH,
            &format!("/api/playlist-folders/{}", folder_b.id),
            &app.admin,
        )
        .json(&UpdatePlaylistFolder {
            revision: 0,
            name: "Early".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(renamed.name, "Early");
    assert_eq!(renamed.revision, 1);
    let before_delete = format!("/api/playlists/{}", playlist.id);
    let _: PlaylistSummary = app
        .request(
            Method::PATCH,
            &format!("/api/playlists/{}/folder", playlist.id),
            &app.admin,
        )
        .json(&MovePlaylist {
            revision: 1,
            folder_id: Some(folder_b.id.clone()),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        app.request(
            Method::DELETE,
            &format!("/api/playlist-folders/{}?revision=1", folder_b.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NO_CONTENT
    );
    let detail: PlaylistDetail = app
        .request(Method::GET, &before_delete, &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail.playlist.folder_id, None);
    assert!(detail.playlist.revision >= 3);
    let folders: Vec<PlaylistFolder> = app
        .request(Method::GET, "/api/playlist-folders", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0].id, folder_a.id);
    let other = app.user("folder_other").await;
    let foreign: Vec<PlaylistFolder> = app
        .request(Method::GET, "/api/playlist-folders", &other)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(foreign.is_empty());
}
