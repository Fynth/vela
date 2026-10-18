//! Real independent HTTP servers; the fixture route pin exists only in unit builds.
use super::*;
use std::sync::{Arc, Mutex};
struct Node {
    base: String,
    state: AppState,
    task: tokio::task::JoinHandle<()>,
    requests: Arc<Mutex<Vec<String>>>,
    _directory: tempfile::TempDir,
}
impl Drop for Node {
    fn drop(&mut self) {
        self.task.abort()
    }
}
impl Node {
    async fn start(origin: &str, fixture: Option<(String, SocketAddr)>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let config = Config {
            bind: "127.0.0.1:0".parse().unwrap(),
            data_dir: directory.path().join("data"),
            web_dir: directory.path().join("web"),
            cookie_secure: false,
            max_file_bytes: 1048576,
            max_storage_bytes: 10485760,
            scan_roots: Vec::new(),
        };
        let mut state = AppState::initialize(config).await.unwrap();
        state.peers.fixture = fixture;
        sqlx::query("UPDATE network_settings SET enabled=1,origin=?,revision=1")
            .bind(origin)
            .execute(&state.pool)
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorder = requests.clone();
        let app = router(state.clone()).layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let recorder = recorder.clone();
                async move {
                    recorder.lock().unwrap().push(request.uri().to_string());
                    next.run(request).await
                }
            },
        ));
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap()
        });
        Self {
            base,
            state,
            task,
            requests,
            _directory: directory,
        }
    }
}
#[tokio::test]
async fn two_independent_origins_preview_only_declared_music_and_revocation_reauthorizes_each_transfer()
 {
    let a = Node::start("https://a.vela.example.org", None).await;
    let b = Node::start(
        "https://b.vela.example.org",
        Some((
            "https://a.vela.example.org".into(),
            a.base.trim_start_matches("http://").parse().unwrap(),
        )),
    )
    .await;
    let client = reqwest::Client::new();
    let setup = client
        .post(format!("{}/api/setup", b.base))
        .json(&Credentials {
            username: "node-b".into(),
            password: "legitimate fixture password".into(),
        })
        .send()
        .await
        .unwrap();
    let cookie = setup.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let auth: AuthResponse = setup.json().await.unwrap();
    // Setup real A owner through the actual authentication service, then real ingest.
    let owner = a
        .state
        .auth
        .setup(Credentials {
            username: "node-a".into(),
            password: "legitimate fixture password".into(),
        })
        .await
        .unwrap();
    let staged = a.state.config.data_dir.join("staging/public-fixture.flac");
    tokio::fs::write(
        &staged,
        include_bytes!("../../../../crates/media/tests/fixtures/signal.flac"),
    )
    .await
    .unwrap();
    let track = a
        .state
        .media
        .ingest(&owner.response.user.id, &staged, "signal.flac")
        .await
        .unwrap()
        .track;
    let before = a
        .state
        .media
        .public_catalog(&PublicationQuery::default())
        .await
        .unwrap();
    assert!(before.items.is_empty());
    let record = a
        .state
        .media
        .create_publication(
            &owner.response.user.id,
            CreatePublication {
                track_id: track.id.clone(),
                license: PublicLicense::Cc0,
                attribution: "Original project-generated signal, Vela fixtures".into(),
                rights_evidence: "https://example.org/generated-fixture".into(),
                allow_stream: true,
                allow_download: false,
                rights_confirmed: true,
            },
        )
        .await
        .unwrap();
    // Test-only hostname pin directs the B-side peer client to the actual A socket.
    let response = client
        .post(format!("{}/api/network/preview", b.base))
        .header("Cookie", &cookie)
        .header("X-CSRF-Token", &auth.csrf_token)
        .json(&PreviewPeer {
            origin: "https://a.vela.example.org".into(),
            cursor: None,
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let preview: PeerPreview = response.json().await.unwrap();
    assert_eq!(preview.catalog.items.len(), 1);
    assert_eq!(preview.catalog.items[0].id, record.id);
    assert_ne!(preview.catalog.items[0].id, track.id);
    assert!(preview.catalog.items[0].can_stream);
    assert!(!preview.catalog.items[0].can_download);
    let second_cursor = record.id.clone();
    let second = b
        .state
        .peers
        .preview("https://a.vela.example.org", Some(&second_cursor))
        .await
        .unwrap_or_else(|e| panic!("Peer cursor preview failed with {}", e.0));
    assert!(second.catalog.items.is_empty());
    assert!(
        a.requests
            .lock()
            .unwrap()
            .iter()
            .any(|uri| uri.contains(&format!("cursor={second_cursor}"))),
        "peer client must forward the validated cursor"
    );
    assert!(
        b.state
            .peers
            .preview("https://a.vela.example.org", Some("not-a-uuid"))
            .await
            .is_err()
    );
    let stream = format!("{}/api/public/media/{}/stream", a.base, record.id);
    let bytes = client
        .get(&stream)
        .header("Range", "bytes=0-15")
        .send()
        .await
        .unwrap();
    assert_eq!(bytes.status(), reqwest::StatusCode::PARTIAL_CONTENT);
    assert_eq!(bytes.bytes().await.unwrap().len(), 16);
    assert_eq!(
        client.head(&stream).send().await.unwrap().status(),
        reqwest::StatusCode::OK
    );
    assert_eq!(
        client
            .get(format!(
                "{}/api/public/media/{}/download",
                a.base, record.id
            ))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    let revoked = a
        .state
        .media
        .revoke_publication(&owner.response.user.id, &record.id, record.revision)
        .await
        .unwrap();
    assert!(revoked.revoked_at.is_some());
    assert!(
        b.state
            .peers
            .preview("https://a.vela.example.org", None)
            .await
            .unwrap_or_else(|e| panic!("Peer preview failed with {}", e.0))
            .catalog
            .items
            .is_empty()
    );
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        assert_eq!(
            client
                .request(method, &stream)
                .header("Range", "bytes=0-15")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::NOT_FOUND
        )
    }
    sqlx::query("UPDATE network_settings SET enabled=0")
        .execute(&a.state.pool)
        .await
        .unwrap();
    assert!(
        b.state
            .peers
            .preview("https://a.vela.example.org", None)
            .await
            .is_err()
    );
    assert_eq!(
        client
            .get(format!("{}/.well-known/vela", a.base))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    assert_ne!(
        a.state.network.settings().await.unwrap().server_id,
        b.state.network.settings().await.unwrap().server_id
    );
    assert!(!auth.user.id.is_empty());
}
#[test]
fn peer_validation_never_admits_private_or_credentialed_origins() {
    for origin in [
        "https://127.0.0.1",
        "https://localhost",
        "https://169.254.169.254",
        "http://public.example.org",
        "https://user:secret@public.example.org",
        "https://public.example.org/path",
    ] {
        assert!(canonical_public_origin(origin).is_err())
    }
}

#[tokio::test]
async fn anonymous_stream_budget_reserves_owner_capacity_and_releases_on_body_drop() {
    let node = Node::start("https://budget.example.org", None).await;
    let path = node._directory.path().join("bounded-audio");
    tokio::fs::write(&path, vec![0u8; 1024]).await.unwrap();
    let asset = || vela_media::MediaAsset {
        path: path.clone(),
        content_type: "audio/flac".into(),
        size_bytes: 1024,
        hash: "a".repeat(64),
    };
    let mut bodies = Vec::new();
    for _ in 0..8 {
        bodies.push(
            serve_asset(&node.state, asset(), Method::GET, HeaderMap::new(), true)
                .await
                .unwrap_or_else(|e| panic!("unexpected {}", e.0)),
        );
    }
    let blocked = serve_asset(&node.state, asset(), Method::GET, HeaderMap::new(), true).await;
    assert!(matches!(
        blocked,
        Err(HttpError(StatusCode::TOO_MANY_REQUESTS, _))
    ));
    let private = serve_asset(&node.state, asset(), Method::GET, HeaderMap::new(), false)
        .await
        .unwrap_or_else(|e| panic!("owner stream blocked {}", e.0));
    assert_eq!(private.status(), StatusCode::OK);
    assert_eq!(node.state.streams.available_permits(), 7);
    drop(private);
    drop(bodies);
    assert_eq!(node.state.public_streams.available_permits(), 8);
    assert_eq!(node.state.streams.available_permits(), 16);
}

#[tokio::test]
async fn hostile_peers_cannot_spoof_origin_inject_grants_or_escape_response_bounds() {
    let origin = "https://hostile.example.org";
    let identity = uuid::Uuid::new_v4().to_string();
    let caps = serde_json::json!({"protocol_version":1,"server_id":identity,"origin":origin,"name":"Fixture","software_version":"0.11.0","public_artwork":false});
    let catalog =
        serde_json::json!({"server_id":identity,"origin":origin,"items":[],"next_cursor":null});
    let track = serde_json::json!({"id":"10000000-0000-4000-8000-000000000001","revision":1,"title":"Generated signal","artist":"Vela fixture","album":"","genre":"","year":null,"duration_ms":1000,"codec":"FLAC","license":"CC0-1.0","attribution":"Original fixture","can_stream":true,"can_download":false});
    let mut cases = Vec::new();
    let mut changed = caps.clone();
    changed["origin"] = serde_json::json!("https://spoof.example.org");
    cases.push((changed.to_string(), catalog.clone()));
    let mut changed = caps.clone();
    changed["name"] = serde_json::json!("secret\u{0}diagnostic");
    cases.push((changed.to_string(), catalog.clone()));
    cases.push(("x".repeat(16 * 1024 + 1), catalog.clone()));
    for change in [
        "identity",
        "download",
        "duplicate",
        "order",
        "cursor",
        "title",
        "license",
    ] {
        let mut changed = catalog.clone();
        changed["items"] = serde_json::json!([track.clone()]);
        match change {
            "identity" => {
                changed["server_id"] = serde_json::json!(uuid::Uuid::new_v4().to_string())
            }
            "download" => {
                changed["items"][0]["can_stream"] = serde_json::json!(false);
                changed["items"][0]["can_download"] = serde_json::json!(true);
            }
            "duplicate" => changed["items"] = serde_json::json!([track.clone(), track.clone()]),
            "order" => {
                let mut lower = track.clone();
                lower["id"] = serde_json::json!("00000000-0000-4000-8000-000000000000");
                changed["items"] = serde_json::json!([track.clone(), lower]);
            }
            "cursor" => {
                changed["next_cursor"] = serde_json::json!("00000000-0000-4000-8000-000000000000")
            }
            "title" => changed["items"][0]["title"] = serde_json::json!("x".repeat(513)),
            "license" => changed["items"][0]["license"] = serde_json::json!("not-a-public-license"),
            _ => unreachable!(),
        }
        cases.push((caps.to_string(), changed));
    }
    for (cap_body, cat_body) in cases {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let service = Router::new()
            .route(
                "/.well-known/vela",
                get(move || {
                    let body = cap_body.clone();
                    async move { body }
                }),
            )
            .route(
                "/api/public/catalog",
                get(move || {
                    let body = cat_body.clone();
                    async move { Json(body) }
                }),
            );
        let server = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
        let peer = PeerClient {
            fixture: Some((origin.into(), address)),
            ..Default::default()
        };
        let result = peer.preview(origin, None).await;
        assert!(matches!(result, Err(HttpError(StatusCode::BAD_GATEWAY, _))));
        assert_eq!(peer.slots.available_permits(), 2);
        server.abort();
    }
    // A stalled real HTTP response is canceled by the outer deadline and releases admission.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let service = Router::new().route(
        "/.well-known/vela",
        get(|| async {
            tokio::time::sleep(Duration::from_secs(20)).await;
            "late"
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    let peer = PeerClient {
        fixture: Some((origin.into(), address)),
        ..Default::default()
    };
    assert!(peer.preview(origin, None).await.is_err());
    assert_eq!(peer.slots.available_permits(), 2);
    server.abort();
}
