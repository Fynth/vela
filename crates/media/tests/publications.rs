//! Actual migrated SQLite policy tests. Synthetic metadata is not decoder or
//! native playback evidence; ingestion is covered by the real FLAC fixture.
use std::time::Duration;

use anyhow::Result;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tempfile::TempDir;
use uuid::Uuid;
use vela_domain::{CreatePublication, PublicLicense, PublicationQuery};
use vela_media::{MediaService, PublicationError};

struct Fixture {
    dir: TempDir,
    pool: SqlitePool,
    media: MediaService,
}

impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(dir.path().join("fixture.db"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(10));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .unwrap();
        for migration in [
            include_str!("../../../migrations/0001_initial.sql"),
            include_str!("../../../migrations/0002_catalog_groups.sql"),
            include_str!("../../../migrations/0003_catalog_metadata_revision.sql"),
            include_str!("../../../migrations/0013_stats_covering_indexes.sql"),
            include_str!("../../../migrations/0004_readonly_scan_sources.sql"),
            include_str!("../../../migrations/0005_catalog_filters.sql"),
            include_str!("../../../migrations/0006_cover_genre_facets.sql"),
            include_str!("../../../migrations/0007_job_history_pages.sql"),
            include_str!("../../../migrations/0008_private_listening.sql"),
            include_str!("../../../migrations/0009_content_audio_analysis.sql"),
            include_str!("../../../migrations/0010_recommendation_exclusions.sql"),
            include_str!("../../../migrations/0011_explicit_publications.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO network_settings(id,server_id,enabled,origin,name,revision) VALUES(1,'f53d2f00-51ac-4560-87eb-257f6cfba190',0,NULL,'Fixture',0)")
            .execute(&pool).await.unwrap();
        let media = MediaService::new(pool.clone(), dir.path().to_owned(), 1024 * 1024);
        media.initialize().await.unwrap();
        let fixture = Self { dir, pool, media };
        for owner in ["alice", "bob"] {
            fixture.user(owner).await;
        }
        fixture
    }

    async fn user(&self, owner: &str) {
        sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,'test-only')")
            .bind(owner)
            .bind(owner)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn enabled(&self, enabled: bool) {
        sqlx::query("UPDATE network_settings SET enabled=?,origin='https://music.example.org',revision=revision+1 WHERE id=1")
            .bind(enabled).execute(&self.pool).await.unwrap();
    }

    async fn track(&self, owner: &str, title: &str, shared: Option<&str>) -> String {
        let id = Uuid::new_v4().to_string();
        let asset = if let Some(asset) = shared {
            asset.to_owned()
        } else {
            let hash = blake3::hash(id.as_bytes()).to_hex().to_string();
            sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',8)")
                .bind(&id).bind(&hash).bind(format!("media/{hash}.audio")).execute(&self.pool).await.unwrap();
            tokio::fs::write(
                self.dir.path().join("media").join(format!("{hash}.audio")),
                b"original",
            )
            .await
            .unwrap();
            id.clone()
        };
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,genre,duration_ms,codec) VALUES(?,?,?,?,'Ensemble','Concert','Chamber',1000,'FLAC')")
            .bind(&id).bind(owner).bind(asset).bind(title).execute(&self.pool).await.unwrap();
        id
    }

    async fn seed_revoked(&self, owner: &str, track: &str, count: i64, offset: i64) {
        // Fixed canonical UUID fixtures, one statement, no active grants.
        sqlx::query("WITH RECURSIVE sequence(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM sequence WHERE n<?) INSERT INTO publications(id,user_id,track_id,license,attribution,rights_evidence,allow_stream,allow_download,revision,created_at,revoked_at) SELECT printf('%08x-0000-4000-8000-%012x',n+?,n+?),?,?,'CC0-1.0','Historical assertion','https://rights.example.org/evidence',0,0,2,1,2 FROM sequence")
            .bind(count).bind(offset).bind(offset).bind(owner).bind(track).execute(&self.pool).await.unwrap();
    }
}

fn input(track: &str) -> CreatePublication {
    CreatePublication {
        track_id: track.to_owned(),
        license: PublicLicense::Attribution,
        attribution: "  Composer, recording 2026\nLicensed by uploader  ".into(),
        rights_evidence: "https://rights.example.org/private-owner-evidence".into(),
        allow_stream: false,
        allow_download: false,
        rights_confirmed: true,
    }
}

fn is_error<T: std::fmt::Debug>(result: Result<T>, predicate: fn(&PublicationError) -> bool) {
    let error = result.unwrap_err();
    assert!(
        error
            .downcast_ref::<PublicationError>()
            .is_some_and(predicate),
        "{error:?}"
    );
}
fn missing<T: std::fmt::Debug>(result: Result<T>) {
    is_error(result, |error| matches!(error, PublicationError::NotFound));
}
fn invalid<T: std::fmt::Debug>(result: Result<T>) {
    is_error(result, |error| matches!(error, PublicationError::Invalid));
}
fn conflict<T: std::fmt::Debug>(result: Result<T>) {
    is_error(result, |error| matches!(error, PublicationError::Conflict));
}

#[tokio::test]
async fn migration_and_server_enablement_never_publish_existing_or_ingested_media() {
    let fixture = Fixture::new().await;
    let track = fixture.track("alice", "Private catalog", None).await;
    missing(
        fixture
            .media
            .public_catalog(&PublicationQuery::default())
            .await,
    );
    conflict(
        fixture
            .media
            .create_publication("alice", input(&track))
            .await,
    );
    let staged = fixture.dir.path().join("staging/real.flac");
    tokio::fs::write(&staged, include_bytes!("fixtures/signal.flac"))
        .await
        .unwrap();
    fixture
        .media
        .ingest("alice", &staged, "real.flac")
        .await
        .unwrap();
    fixture.enabled(true).await;
    assert!(
        fixture
            .media
            .public_catalog(&PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        fixture
            .media
            .publications("alice", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    missing(fixture.media.public_track(&track).await);
    missing(fixture.media.public_asset(&track, false).await);
    assert!(
        fixture
            .media
            .get_track("alice", &track)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn owner_authorization_survives_shared_assets_and_public_metadata_grants_no_audio() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let alice = fixture.track("alice", "Owner publication", None).await;
    let bob = fixture
        .track("bob", "Separate private metadata", Some(&alice))
        .await;
    sqlx::query("INSERT INTO favorites(user_id,track_id) VALUES('alice',?)")
        .bind(&alice)
        .execute(&fixture.pool)
        .await
        .unwrap();
    missing(fixture.media.create_publication("bob", input(&alice)).await);
    let publication = fixture
        .media
        .create_publication("alice", input(&alice))
        .await
        .unwrap();
    assert!(publication.track.favorite);
    assert_ne!(publication.id, alice);
    assert_ne!(publication.id, bob);
    assert_eq!(
        publication.attribution,
        "Composer, recording 2026\nLicensed by uploader"
    );
    assert_eq!(publication.rights_evidence, input(&alice).rights_evidence);
    assert_eq!(publication.revision, 1);
    assert!(publication.revoked_at.is_none());
    sqlx::query("UPDATE favorites SET user_id='bob' WHERE user_id='alice' AND track_id=?")
        .bind(&alice)
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert!(
        !fixture
            .media
            .publications("alice", &PublicationQuery::default())
            .await
            .unwrap()
            .items[0]
            .track
            .favorite
    );
    assert!(
        fixture
            .media
            .publications("bob", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    missing(
        fixture
            .media
            .revoke_publication("bob", &publication.id, 1)
            .await,
    );
    let catalog = fixture
        .media
        .public_catalog(&PublicationQuery::default())
        .await
        .unwrap();
    assert_eq!(catalog.items.len(), 1);
    assert_eq!(catalog.items[0].id, publication.id);
    assert!(!catalog.items[0].can_stream);
    assert!(!catalog.items[0].can_download);
    missing(fixture.media.public_asset(&publication.id, false).await);
    missing(fixture.media.public_asset(&publication.id, true).await);
    missing(fixture.media.public_track(&bob).await);
    missing(fixture.media.public_asset(&bob, false).await);
    assert!(fixture.media.asset("bob", &alice).await.unwrap().is_none());
    assert!(fixture.media.asset("bob", &bob).await.unwrap().is_some());
    let value = serde_json::to_value(&catalog.items[0]).unwrap();
    let object = value.as_object().unwrap();
    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "album",
            "artist",
            "attribution",
            "can_download",
            "can_stream",
            "codec",
            "duration_ms",
            "genre",
            "id",
            "license",
            "revision",
            "title",
            "year"
        ]
    );
    let serialized = serde_json::to_string(&catalog).unwrap();
    for private in [
        alice.as_str(),
        bob.as_str(),
        "alice",
        "bob",
        "private-owner-evidence",
        "test-only",
        "media/",
        "favorite",
        "artwork_id",
        "created_at",
        "revoked_at",
        "rights_evidence",
        "hash",
    ] {
        assert!(!serialized.contains(private), "leaked {private}");
    }
}

#[tokio::test]
async fn rights_assertions_closed_inputs_and_limits_reject_adversarial_requests() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let track = fixture.track("alice", "Validation", None).await;
    let mut request = input(&track);
    request.rights_confirmed = false;
    invalid(fixture.media.create_publication("alice", request).await);
    let mut request = input(&track);
    request.allow_download = true;
    invalid(fixture.media.create_publication("alice", request).await);
    for evidence in [
        "http://rights.example.org/evidence",
        "https://user:secret@rights.example.org/evidence",
        "https://@rights.example.org/evidence",
        "https://rights.example.org/evidence?secret=x",
        "https://rights.example.org/evidence#secret",
        "https://localhost/evidence",
        "https://127.0.0.1/evidence",
        "https://rights.example.org:8443/evidence",
        "https://rights.example.org/\nevidence",
        "https:\\rights.example.org\\evidence",
    ] {
        let mut request = input(&track);
        request.rights_evidence = evidence.into();
        invalid(fixture.media.create_publication("alice", request).await);
    }
    let mut request = input(&track);
    request.rights_evidence = format!("https://rights.example.org/{}", "a".repeat(2048));
    invalid(fixture.media.create_publication("alice", request).await);
    for attribution in [
        "   ".to_owned(),
        "a".repeat(2001),
        "license\rforbidden".into(),
        "license\0forbidden".into(),
        "license\tforbidden".into(),
    ] {
        let mut request = input(&track);
        request.attribution = attribution;
        invalid(fixture.media.create_publication("alice", request).await);
    }
    let mut request = input(&track);
    request.track_id = "F53D2F00-51AC-4560-87EB-257F6CFBA190".into();
    invalid(fixture.media.create_publication("alice", request).await);
    let mut value = serde_json::to_value(input(&track)).unwrap();
    value["owner_id"] = "bob".into();
    assert!(serde_json::from_value::<CreatePublication>(value).is_err());
    assert!(serde_json::from_str::<PublicationQuery>(r#"{"offset":0}"#).is_err());
    assert!(serde_json::from_str::<PublicLicense>("\"all-rights-reserved\"").is_err());
    for query in [
        PublicationQuery {
            limit: Some(0),
            ..Default::default()
        },
        PublicationQuery {
            limit: Some(101),
            ..Default::default()
        },
        PublicationQuery {
            cursor: Some("../private".into()),
            ..Default::default()
        },
        PublicationQuery {
            q: Some("a".repeat(129)),
            ..Default::default()
        },
        PublicationQuery {
            q: Some("control\0query".into()),
            ..Default::default()
        },
    ] {
        invalid(fixture.media.publications("alice", &query).await);
    }
    invalid(
        fixture
            .media
            .public_catalog(&PublicationQuery {
                limit: Some(51),
                ..Default::default()
            })
            .await,
    );
    invalid(fixture.media.revoke_publication("alice", &track, 0).await);
    missing(fixture.media.public_track("../private").await);
    let mut accepted = input(&track);
    accepted.attribution = "é".repeat(1000);
    fixture
        .media
        .create_publication("alice", accepted)
        .await
        .unwrap();
}

#[tokio::test]
async fn independent_grants_revoke_cas_and_republication_preserve_original_bytes_and_assertions() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let track = fixture.track("alice", "Licensed audio", None).await;
    let original = fixture.media.asset("alice", &track).await.unwrap().unwrap();
    let original_bytes = tokio::fs::read(&original.path).await.unwrap();
    let mut request = input(&track);
    request.allow_stream = true;
    let first = fixture
        .media
        .create_publication("alice", request)
        .await
        .unwrap();
    assert_eq!(
        fixture
            .media
            .public_asset(&first.id, false)
            .await
            .unwrap()
            .hash,
        original.hash
    );
    missing(fixture.media.public_asset(&first.id, true).await);
    conflict(
        fixture
            .media
            .create_publication("alice", input(&track))
            .await,
    );
    conflict(
        fixture
            .media
            .revoke_publication("alice", &first.id, 2)
            .await,
    );
    let revoked = fixture
        .media
        .revoke_publication("alice", &first.id, 1)
        .await
        .unwrap();
    assert_eq!(revoked.revision, 2);
    assert!(revoked.revoked_at.is_some());
    assert_eq!(revoked.created_at, first.created_at);
    assert_eq!(revoked.attribution, first.attribution);
    assert_eq!(revoked.rights_evidence, first.rights_evidence);
    assert_eq!(revoked.license, first.license);
    conflict(
        fixture
            .media
            .revoke_publication("alice", &first.id, 1)
            .await,
    );
    let repeated = fixture
        .media
        .revoke_publication("alice", &first.id, 2)
        .await
        .unwrap();
    assert_eq!(repeated.revoked_at, revoked.revoked_at);
    assert_eq!(repeated.revision, 2);
    for _ in 0..3 {
        missing(fixture.media.public_track(&first.id).await);
        missing(fixture.media.public_asset(&first.id, false).await);
        missing(fixture.media.public_asset(&first.id, true).await);
    }
    let mut request = input(&track);
    request.allow_stream = true;
    request.allow_download = true;
    request.license = PublicLicense::Cc0;
    let second = fixture
        .media
        .create_publication("alice", request)
        .await
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(
        fixture
            .media
            .public_asset(&second.id, true)
            .await
            .unwrap()
            .hash,
        original.hash
    );
    missing(fixture.media.public_asset(&first.id, false).await);
    assert_eq!(
        fixture
            .media
            .publications("alice", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .len(),
        2
    );
    assert_eq!(
        tokio::fs::read(&original.path).await.unwrap(),
        original_bytes
    );
    assert_eq!(
        fixture
            .media
            .get_track("alice", &track)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Licensed audio"
    );
}

#[tokio::test]
async fn disabling_or_invalidating_current_origin_closes_every_new_public_read() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let track = fixture.track("alice", "Revocable", None).await;
    let mut request = input(&track);
    request.allow_stream = true;
    request.allow_download = true;
    let publication = fixture
        .media
        .create_publication("alice", request)
        .await
        .unwrap();
    assert!(
        fixture
            .media
            .public_asset(&publication.id, true)
            .await
            .is_ok()
    );
    for origin in [
        None,
        Some("https://127.0.0.1"),
        Some("http://music.example.org"),
        Some("https://music.example.org/private"),
        Some("https://Music.Example.org"),
    ] {
        sqlx::query("UPDATE network_settings SET origin=? WHERE id=1")
            .bind(origin)
            .execute(&fixture.pool)
            .await
            .unwrap();
        missing(
            fixture
                .media
                .public_catalog(&PublicationQuery::default())
                .await,
        );
        missing(fixture.media.public_track(&publication.id).await);
        missing(fixture.media.public_asset(&publication.id, false).await);
        missing(fixture.media.public_asset(&publication.id, true).await);
    }
    fixture.enabled(false).await;
    // Disabled policy does not reveal query limits or catalog counts.
    missing(
        fixture
            .media
            .public_catalog(&PublicationQuery {
                limit: Some(10000),
                ..Default::default()
            })
            .await,
    );
    assert_eq!(
        fixture
            .media
            .publications("alice", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    fixture
        .media
        .revoke_publication("alice", &publication.id, 1)
        .await
        .unwrap();
    fixture.enabled(true).await;
    missing(fixture.media.public_track(&publication.id).await);
    assert!(
        fixture
            .media
            .public_catalog(&PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn public_projection_is_utf8_bounded_and_requires_current_positive_actual_asset() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let track = fixture.track("alice", &"界".repeat(600), None).await;
    sqlx::query("UPDATE tracks SET artist=?,album=?,genre=?,codec=? WHERE id=?")
        .bind("é".repeat(600))
        .bind("🎵".repeat(600))
        .bind("界".repeat(300))
        .bind("é".repeat(80))
        .bind(&track)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let mut request = input(&track);
    request.allow_stream = true;
    let publication = fixture
        .media
        .create_publication("alice", request)
        .await
        .unwrap();
    let public = fixture.media.public_track(&publication.id).await.unwrap();
    assert_eq!(public.title.len(), 510);
    assert_eq!(public.artist.len(), 512);
    assert_eq!(public.album.len(), 512);
    assert_eq!(public.genre.len(), 255);
    assert_eq!(public.codec.len(), 64);
    assert_eq!(publication.track.title.len(), 1800);
    sqlx::query("UPDATE tracks SET title=?,artist=?,album=?,genre=?,codec=?,year=0 WHERE id=?")
        .bind("\0\n\t ")
        .bind("En\0semble")
        .bind("Al\rbum")
        .bind("Cham\tber")
        .bind("FL\nAC")
        .bind(&track)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let safe = fixture.media.public_track(&publication.id).await.unwrap();
    assert_eq!(safe.title, "Untitled");
    assert_eq!(safe.artist, "Ensemble");
    assert_eq!(safe.album, "Album");
    assert_eq!(safe.genre, "Chamber");
    assert_eq!(safe.codec, "FLAC");
    assert_eq!(safe.year, None);
    assert_eq!(safe.attribution, publication.attribution);
    assert!(safe.attribution.contains('\n'));
    let owned = fixture
        .media
        .get_track("alice", &track)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(owned.title, "\0\n\t ");
    assert_eq!(owned.artist, "En\0semble");
    assert_eq!(owned.year, Some(0));
    for (year, expected) in [(-1, None), (10000, None), (9999, Some(9999))] {
        sqlx::query("UPDATE tracks SET year=? WHERE id=?")
            .bind(year)
            .bind(&track)
            .execute(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(
            fixture
                .media
                .public_track(&publication.id)
                .await
                .unwrap()
                .year,
            expected
        );
    }
    for duration in [0, 604800001] {
        sqlx::query("UPDATE tracks SET duration_ms=? WHERE id=?")
            .bind(duration)
            .bind(&track)
            .execute(&fixture.pool)
            .await
            .unwrap();
        missing(fixture.media.public_track(&publication.id).await);
        missing(fixture.media.public_asset(&publication.id, false).await);
        assert!(
            fixture
                .media
                .public_catalog(&PublicationQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    let zero = fixture.track("alice", "Zero duration", None).await;
    sqlx::query("UPDATE tracks SET duration_ms=0 WHERE id=?")
        .bind(&zero)
        .execute(&fixture.pool)
        .await
        .unwrap();
    missing(
        fixture
            .media
            .create_publication("alice", input(&zero))
            .await,
    );
    sqlx::query("UPDATE tracks SET duration_ms=604800001 WHERE id=?")
        .bind(&zero)
        .execute(&fixture.pool)
        .await
        .unwrap();
    missing(
        fixture
            .media
            .create_publication("alice", input(&zero))
            .await,
    );
    sqlx::query("UPDATE tracks SET duration_ms=1000 WHERE id=?")
        .bind(&track)
        .execute(&fixture.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE media_assets SET hash='../invalid' WHERE id=?")
        .bind(&track)
        .execute(&fixture.pool)
        .await
        .unwrap();
    missing(fixture.media.public_track(&publication.id).await);
    assert!(
        fixture
            .media
            .public_catalog(&PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn uuid_paging_is_bounded_owner_scoped_and_search_is_literal() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let mut ids = Vec::new();
    for title in [
        "literal 100%_match",
        "Ordinary selection",
        "Third publication",
    ] {
        let track = fixture.track("alice", title, None).await;
        ids.push(
            fixture
                .media
                .create_publication("alice", input(&track))
                .await
                .unwrap()
                .id,
        );
    }
    let bob = fixture.track("bob", "Bob record", None).await;
    fixture
        .media
        .create_publication("bob", input(&bob))
        .await
        .unwrap();
    ids.sort();
    let first = fixture
        .media
        .publications(
            "alice",
            &PublicationQuery {
                limit: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        first.items.iter().map(|p| &p.id).collect::<Vec<_>>(),
        ids[..2].iter().collect::<Vec<_>>()
    );
    assert_eq!(first.next_cursor.as_ref(), Some(&ids[1]));
    let last = fixture
        .media
        .publications(
            "alice",
            &PublicationQuery {
                limit: Some(2),
                cursor: first.next_cursor,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(last.items.len(), 1);
    assert_eq!(last.items[0].id, ids[2]);
    assert!(last.next_cursor.is_none());
    for q in ["%_", "100%", "_match"] {
        let query = PublicationQuery {
            q: Some(q.into()),
            ..Default::default()
        };
        assert_eq!(
            fixture
                .media
                .public_catalog(&query)
                .await
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            fixture
                .media
                .publications("alice", &query)
                .await
                .unwrap()
                .items
                .len(),
            1
        );
        assert!(
            fixture
                .media
                .publications("bob", &query)
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    assert!(
        fixture
            .media
            .public_catalog(&PublicationQuery {
                q: Some("' OR 1=1 --".into()),
                ..Default::default()
            })
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let first_public = fixture
        .media
        .public_catalog(&PublicationQuery {
            limit: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    let second_public = fixture
        .media
        .public_catalog(&PublicationQuery {
            limit: Some(2),
            cursor: first_public.next_cursor.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(first_public.items.len(), 2);
    assert_eq!(second_public.items.len(), 2);
    assert!(second_public.next_cursor.is_none());
    assert!(
        first_public
            .items
            .iter()
            .all(|a| second_public.items.iter().all(|b| a.id < b.id))
    );
    fixture
        .media
        .revoke_publication("alice", &ids[0], 1)
        .await
        .unwrap();
    assert_eq!(
        fixture
            .media
            .public_catalog(&PublicationQuery::default())
            .await
            .unwrap()
            .items
            .len(),
        3
    );
    assert_eq!(
        fixture
            .media
            .publications("alice", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .len(),
        3
    );
}

#[tokio::test]
async fn revoked_records_count_toward_owner_and_global_quotas() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let alice = fixture.track("alice", "Historical quota", None).await;
    fixture.seed_revoked("alice", &alice, 1000, 0).await;
    conflict(
        fixture
            .media
            .create_publication("alice", input(&alice))
            .await,
    );
    assert_eq!(
        fixture
            .media
            .publications("alice", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .len(),
        50
    );
    assert_eq!(
        fixture
            .media
            .publications(
                "alice",
                &PublicationQuery {
                    limit: Some(100),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .len(),
        100
    );
    assert!(
        fixture
            .media
            .public_catalog(&PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    for index in 0..9 {
        let owner = format!("historical-{index}");
        fixture.user(&owner).await;
        let track = fixture.track(&owner, "Quota fixture", None).await;
        fixture
            .seed_revoked(&owner, &track, 1000, (index + 1) * 1000)
            .await;
    }
    let bob = fixture.track("bob", "Global limit", None).await;
    conflict(fixture.media.create_publication("bob", input(&bob)).await);
    assert!(
        fixture
            .media
            .publications("bob", &PublicationQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publications")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(count, 10000);
}

#[tokio::test]
async fn concurrent_creations_cannot_overrun_owner_quota_or_create_two_active_records() {
    let fixture = Fixture::new().await;
    fixture.enabled(true).await;
    let first = fixture.track("alice", "Race one", None).await;
    let second = fixture.track("alice", "Race two", None).await;
    fixture.seed_revoked("alice", &first, 999, 0).await;
    let (a, b) = tokio::join!(
        fixture.media.create_publication("alice", input(&first)),
        fixture.media.create_publication("alice", input(&second))
    );
    assert_ne!(a.is_ok(), b.is_ok());
    conflict(if a.is_err() { a } else { b });
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publications WHERE user_id='alice'")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(count, 1000);
    let bob = fixture.track("bob", "Active race", None).await;
    let (a, b) = tokio::join!(
        fixture.media.create_publication("bob", input(&bob)),
        fixture.media.create_publication("bob", input(&bob))
    );
    assert_ne!(a.is_ok(), b.is_ok());
    conflict(if a.is_err() { a } else { b });
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM publications WHERE user_id='bob' AND revoked_at IS NULL",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&fixture.pool)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        sqlx::query("PRAGMA quick_check")
            .fetch_one(&fixture.pool)
            .await
            .unwrap()
            .get::<String, _>(0),
        "ok"
    );
}
