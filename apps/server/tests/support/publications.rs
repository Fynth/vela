use super::*;

#[tokio::test]
async fn publication_http_enforces_explicit_rights_private_ownership_and_every_media_grant() {
    let app = TestApp::new().await;
    let member = app.user("publisher").await;
    let own = app.upload(&app.admin).await.track;
    let foreign = app.upload(&member).await.track;
    for path in [
        "/.well-known/vela",
        "/api/public/catalog",
        "/api/public/catalog?unknown=1",
        "/api/public/catalog?limit=invalid",
        &format!("/api/public/catalog/{}", own.id),
    ] {
        assert_eq!(
            app.client
                .get(format!("{}{path}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let settings: NetworkSettings = app
        .request(Method::GET, "/api/network/settings", &app.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!settings.enabled);
    assert_eq!(settings.revision, 0);
    let change = UpdateNetworkSettings {
        revision: settings.revision,
        enabled: true,
        origin: "https://fixture.vela.example.org".into(),
        name: "Fixture origin".into(),
    };
    assert_eq!(
        app.request(Method::PUT, "/api/network/settings", &member)
            .json(&change)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.client
            .put(format!("{}/api/network/settings", app.base))
            .header("Cookie", &app.admin.cookie)
            .json(&change)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let enabled: NetworkSettings = app
        .request(Method::PUT, "/api/network/settings", &app.admin)
        .json(&change)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(enabled.server_id, settings.server_id);
    assert_eq!(enabled.revision, 1);
    assert_eq!(
        app.request(Method::PUT, "/api/network/settings", &app.admin)
            .json(&change)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let caps = app
        .client
        .get(format!("{}/.well-known/vela", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(caps.status(), StatusCode::OK);
    assert_eq!(caps.headers()["cache-control"], "no-store");
    let caps: NetworkCapabilities = caps.json().await.unwrap();
    assert!(!caps.public_artwork);
    let catalog: PublicCatalog = app
        .client
        .get(format!("{}/api/public/catalog", app.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        catalog.items.is_empty(),
        "enabling publishes no private track"
    );
    let declared = CreatePublication {
        track_id: own.id.clone(),
        license: PublicLicense::Cc0,
        attribution: "Original Vela signal fixture".into(),
        rights_evidence: "https://example.org/fixture-provenance".into(),
        allow_stream: false,
        allow_download: false,
        rights_confirmed: true,
    };
    let mut invalid = declared.clone();
    invalid.rights_confirmed = false;
    assert_eq!(
        app.request(Method::POST, "/api/publications", &app.admin)
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    invalid = declared.clone();
    invalid.allow_download = true;
    assert_eq!(
        app.request(Method::POST, "/api/publications", &app.admin)
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    invalid = declared.clone();
    invalid.track_id = foreign.id.clone();
    assert_eq!(
        app.request(Method::POST, "/api/publications", &app.admin)
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let mut value = serde_json::to_value(&declared).unwrap();
    value["owner_id"] = json!(member.user.id);
    assert_eq!(
        app.request(Method::POST, "/api/publications", &app.admin)
            .json(&value)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let publication: Publication = app
        .request(Method::POST, "/api/publications", &app.admin)
        .json(&declared)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_ne!(publication.id, own.id);
    assert_eq!(
        app.request(Method::POST, "/api/publications", &app.admin)
            .json(&declared)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let public = app
        .client
        .get(format!(
            "{}/api/public/catalog/{}",
            app.base, publication.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(public.status(), StatusCode::OK);
    let value: serde_json::Value = public.json().await.unwrap();
    for private in [
        "track_id",
        "owner_id",
        "user_id",
        "rights_evidence",
        "hash",
        "storage_path",
        "favorite",
        "artwork_id",
        "media_asset_id",
        "listening",
    ] {
        assert!(value.get(private).is_none(), "{private} leaked");
    }
    assert_eq!(value["can_stream"], false);
    for suffix in ["stream", "download"] {
        for method in [Method::GET, Method::HEAD] {
            assert_eq!(
                app.client
                    .request(
                        method,
                        format!("{}/api/public/media/{}/{suffix}", app.base, publication.id)
                    )
                    .header("Range", "bytes=0-15")
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
    }
    assert_eq!(
        app.request(
            Method::POST,
            &format!("/api/publications/{}/revoke", publication.id),
            &member
        )
        .json(&RevokePublication { revision: 1 })
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    let member_records: PublicationPage = app
        .request(Method::GET, "/api/publications", &member)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(member_records.items.is_empty());
    let revoked: Publication = app
        .request(
            Method::POST,
            &format!("/api/publications/{}/revoke", publication.id),
            &app.admin,
        )
        .json(&RevokePublication { revision: 1 })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(revoked.revision, 2);
    let mut permitted = declared.clone();
    permitted.allow_stream = true;
    permitted.allow_download = true;
    let allowed: Publication = app
        .request(Method::POST, "/api/publications", &app.admin)
        .json(&permitted)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_ne!(
        allowed.id, publication.id,
        "republishing cannot resurrect a revoked public ID"
    );
    let url = format!("{}/api/public/media/{}/stream", app.base, allowed.id);
    let response = app
        .client
        .get(&url)
        .header("Range", "bytes=0-15")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    let etag = response.headers()["etag"].clone();
    assert_eq!(response.bytes().await.unwrap(), &AUDIO[..16]);
    assert_eq!(
        app.client
            .head(&url)
            .header("Range", "bytes=0-15")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.client
            .get(&url)
            .header("Range", format!("bytes={}-", AUDIO.len()))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::RANGE_NOT_SATISFIABLE
    );
    assert_eq!(
        app.client
            .get(&url)
            .header("Range", "bytes=0-15")
            .header("If-Range", "\"changed\"")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.client
            .get(&url)
            .header("If-None-Match", etag)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_MODIFIED
    );
    let download = app
        .client
        .get(format!(
            "{}/api/public/media/{}/download",
            app.base, allowed.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(download.status(), StatusCode::OK);
    assert!(
        download.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .starts_with("attachment; filename=\"vela-")
    );
    assert_eq!(download.bytes().await.unwrap().as_ref(), AUDIO);
    assert_eq!(
        app.client
            .get(format!("{}/api/tracks/{}/stream", app.base, own.id))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", own.id),
            &member
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    for q in [
        "limit=0",
        "limit=51",
        "limit=-1",
        "cursor=invalid",
        "q=%00",
        "unknown=1",
    ] {
        assert_eq!(
            app.client
                .get(format!("{}/api/public/catalog?{q}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for origin in [
        "https://127.0.0.1",
        "https://169.254.169.254",
        "https://x.local",
        "https://user:secret@example.org",
        "https://example.org/path",
        "file:///etc/passwd",
    ] {
        let response = app
            .request(Method::POST, "/api/network/preview", &member)
            .json(&PreviewPeer {
                origin: origin.into(),
                cursor: None,
            })
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!response.text().await.unwrap().contains("secret"));
    }
    app.request(
        Method::POST,
        &format!("/api/publications/{}/revoke", allowed.id),
        &app.admin,
    )
    .json(&RevokePublication { revision: 1 })
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
    assert_eq!(
        app.client
            .get(&url)
            .header("Range", "bytes=0-15")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let disabled = UpdateNetworkSettings {
        revision: enabled.revision,
        enabled: false,
        ..change
    };
    app.request(Method::PUT, "/api/network/settings", &app.admin)
        .json(&disabled)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    for path in ["/.well-known/vela", "/api/public/catalog"] {
        assert_eq!(
            app.client
                .get(format!("{}{path}", app.base))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        app.request(
            Method::GET,
            &format!("/api/tracks/{}/stream", own.id),
            &app.admin
        )
        .send()
        .await
        .unwrap()
        .status(),
        StatusCode::OK
    );
}
