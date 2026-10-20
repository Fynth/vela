use std::{path::PathBuf, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tempfile::TempDir;
use vela_media::MediaService;

const FLAC: &[u8] = include_bytes!("fixtures/signal.flac");
const MP3: &[u8] = include_bytes!("fixtures/beacon.mp3");

struct Fixture {
    directory: TempDir,
    pool: SqlitePool,
    media: MediaService,
}

impl Fixture {
    async fn new(max_file: u64, max_storage: u64) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("vela.db"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../../../migrations/0001_initial.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../../../migrations/0002_catalog_groups.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/0003_catalog_metadata_revision.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/0013_stats_covering_indexes.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        for user in ["alice", "bob"] {
            sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,?)")
                .bind(user)
                .bind(user)
                .bind("test fixture password hash")
                .execute(&pool)
                .await
                .unwrap();
        }
        let media = MediaService::new(pool.clone(), directory.path().to_owned(), max_file)
            .with_storage_budget(max_storage);
        media.initialize().await.unwrap();
        Self {
            directory,
            pool,
            media,
        }
    }

    async fn stage(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.directory.path().join("staging").join(name);
        tokio::fs::write(&path, bytes).await.unwrap();
        path
    }
}

#[tokio::test]
async fn real_flac_extracts_tags_artwork_and_original_playable_bytes() {
    let fixture = Fixture::new(1024 * 1024, 10 * 1024 * 1024).await;
    let path = fixture.stage("upload.bin", FLAC).await;
    let result = fixture
        .media
        .ingest("alice", &path, "../../untrusted.mp3")
        .await
        .unwrap();
    assert!(!result.duplicate);
    let track = result.track;
    assert_eq!(track.title, "Signal");
    assert_eq!(track.artist, "Vela Test");
    assert_eq!(track.album, "Real Fixtures");
    assert_eq!(track.codec, "FLAC");
    assert_eq!(track.sample_rate, Some(44100));
    assert_eq!(track.year, Some(2026));
    assert!((990..=1010).contains(&track.duration_ms));
    assert_eq!(track.size_bytes, FLAC.len() as i64);
    let asset = fixture
        .media
        .asset("alice", &track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(asset.content_type, "audio/flac");
    assert_eq!(tokio::fs::read(asset.path).await.unwrap(), FLAC);
    assert_eq!(
        tokio::fs::read(path).await.unwrap(),
        FLAC,
        "caller retains ownership of staging"
    );
    let artwork_id = track.artwork_id.unwrap();
    let art_path = fixture
        .media
        .artwork("alice", &artwork_id)
        .await
        .unwrap()
        .unwrap();
    let thumbnail = image::open(art_path).unwrap();
    assert_eq!(thumbnail.width(), 640);
    assert!(thumbnail.height() <= 640);
    assert!(
        fixture
            .media
            .asset("bob", &track.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .artwork("bob", &artwork_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .get_track("bob", &track.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn duplicates_share_physical_bytes_without_sharing_catalog_ownership() {
    let fixture = Fixture::new(1024 * 1024, FLAC.len() as u64).await;
    let path = fixture.stage("shared.bin", FLAC).await;
    let first = fixture
        .media
        .ingest("alice", &path, "signal.flac")
        .await
        .unwrap();
    let duplicate = fixture
        .media
        .ingest("alice", &path, "other filename.flac")
        .await
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(first.track.id, duplicate.track.id);
    let second_owner = fixture
        .media
        .ingest("bob", &path, "signal.flac")
        .await
        .unwrap();
    assert!(!second_owner.duplicate);
    assert_ne!(first.track.id, second_owner.track.id);
    let assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media_assets")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(assets, 1);
    let alice = fixture.media.stats("alice").await.unwrap();
    let bob = fixture.media.stats("bob").await.unwrap();
    assert_eq!(alice.tracks, 1);
    assert_eq!(bob.tracks, 1);
    assert_eq!(alice.size_bytes, FLAC.len() as i64);
    assert_eq!(alice.albums, 1);
    assert_eq!(alice.artists, 1);
}

#[tokio::test]
async fn simultaneous_exact_uploads_remain_idempotent() {
    let fixture = Fixture::new(1024 * 1024, 10 * 1024 * 1024).await;
    let path = fixture.stage("concurrent.bin", FLAC).await;
    let (first, second) = tokio::join!(
        fixture.media.ingest("alice", &path, "first.flac"),
        fixture.media.ingest("alice", &path, "second.flac"),
    );
    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(first.track.id, second.track.id);
    assert_ne!(first.duplicate, second.duplicate);
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 1);
}

#[tokio::test]
async fn real_mp3_search_pagination_and_favorites_stay_owner_isolated() {
    let fixture = Fixture::new(1024 * 1024, 10 * 1024 * 1024).await;
    let path = fixture.stage("first", FLAC).await;
    fixture.media.ingest("alice", &path, "first").await.unwrap();
    let path = fixture.stage("second", MP3).await;
    let mp3 = fixture
        .media
        .ingest("alice", &path, "second.flac")
        .await
        .unwrap()
        .track;
    assert_eq!(mp3.title, "Beacon");
    assert_eq!(mp3.codec, "MPEG audio");
    assert_eq!(
        fixture
            .media
            .asset("alice", &mp3.id)
            .await
            .unwrap()
            .unwrap()
            .content_type,
        "audio/mpeg"
    );
    let page = fixture
        .media
        .list_tracks("alice", None, None, 1, false)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    let next = fixture
        .media
        .list_tracks("alice", None, page.next_cursor.as_deref(), 1, false)
        .await
        .unwrap();
    assert_eq!(next.items.len(), 1);
    assert_ne!(page.items[0].id, next.items[0].id);
    assert!(next.next_cursor.is_none());
    let result = fixture
        .media
        .list_tracks("alice", Some("beac"), None, 100, false)
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, mp3.id);
    assert!(
        fixture
            .media
            .list_tracks("bob", Some("beac"), None, 100, false)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        fixture
            .media
            .list_tracks("alice", Some("\" OR *"), None, 100, false)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    sqlx::query("INSERT INTO favorites(user_id,track_id) VALUES(?,?)")
        .bind("alice")
        .bind(&mp3.id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let favorites = fixture
        .media
        .list_tracks("alice", None, None, 100, true)
        .await
        .unwrap();
    assert_eq!(favorites.items.len(), 1);
    assert!(favorites.items[0].favorite);
    assert_eq!(favorites.items[0].id, mp3.id);
    assert!(
        fixture
            .media
            .list_tracks("bob", None, None, 100, true)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        fixture
            .media
            .list_tracks("alice", None, Some("not a uuid"), 100, false)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn validation_file_limits_and_global_quota_prevent_publication() {
    let fixture = Fixture::new(1024 * 1024, FLAC.len() as u64).await;
    let invalid = fixture
        .stage("fake.flac", b"<html>this is not audio</html>")
        .await;
    assert!(
        fixture
            .media
            .ingest("alice", &invalid, "fake.flac")
            .await
            .is_err()
    );
    let valid = fixture.stage("real", FLAC).await;
    fixture.media.ingest("alice", &valid, "real").await.unwrap();
    let exceeds_quota = fixture.stage("mp3", MP3).await;
    assert!(
        fixture
            .media
            .ingest("alice", &exceeds_quota, "mp3")
            .await
            .is_err()
    );
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 1);
    let low_limit = MediaService::new(fixture.pool.clone(), fixture.directory.path().to_owned(), 1);
    assert!(low_limit.ingest("alice", &valid, "real").await.is_err());
    let outside = fixture.directory.path().join("outside.flac");
    tokio::fs::write(&outside, FLAC).await.unwrap();
    assert!(
        fixture
            .media
            .ingest("alice", &outside, "real")
            .await
            .is_err()
    );
    let pending = std::fs::read_dir(fixture.directory.path().join("media"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".vela-pending-")
        })
        .count();
    assert_eq!(
        pending, 0,
        "failed validation removes managed temporary files"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn staging_symlinks_are_rejected() {
    let fixture = Fixture::new(1024 * 1024, 10 * 1024 * 1024).await;
    let original = fixture.stage("real", FLAC).await;
    let link = fixture.directory.path().join("staging/link");
    std::os::unix::fs::symlink(original, &link).unwrap();
    assert!(fixture.media.ingest("alice", &link, "real").await.is_err());
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 0);
}

#[tokio::test]
async fn corrupted_existing_audio_is_never_overwritten_or_republished() {
    let fixture = Fixture::new(1024 * 1024, 10 * 1024 * 1024).await;
    let path = fixture.stage("first", FLAC).await;
    let track = fixture
        .media
        .ingest("alice", &path, "first")
        .await
        .unwrap()
        .track;
    let asset = fixture
        .media
        .asset("alice", &track.id)
        .await
        .unwrap()
        .unwrap();
    tokio::fs::write(&asset.path, vec![0_u8; FLAC.len()])
        .await
        .unwrap();
    assert!(fixture.media.ingest("bob", &path, "second").await.is_err());
    assert_eq!(fixture.media.stats("bob").await.unwrap().tracks, 0);
    assert_eq!(
        tokio::fs::read(asset.path).await.unwrap(),
        vec![0_u8; FLAC.len()]
    );
}

#[tokio::test]
async fn startup_removes_only_owned_unpublished_regular_files() {
    let fixture = Fixture::new(1024 * 1024, 10 * 1024 * 1024).await;
    let path = fixture.stage("first", FLAC).await;
    let track = fixture
        .media
        .ingest("alice", &path, "first")
        .await
        .unwrap()
        .track;
    let asset = fixture
        .media
        .asset("alice", &track.id)
        .await
        .unwrap()
        .unwrap();
    let artwork = fixture
        .media
        .artwork("alice", track.artwork_id.as_ref().unwrap())
        .await
        .unwrap()
        .unwrap();
    for directory in ["media", "artwork"] {
        let root = fixture.directory.path().join(directory);
        tokio::fs::write(root.join(".vela-pending-abandoned"), b"incomplete")
            .await
            .unwrap();
        tokio::fs::write(root.join("unrelated-user-file"), b"preserve this")
            .await
            .unwrap();
        tokio::fs::create_dir(root.join(".vela-pending-directory"))
            .await
            .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            root.join("unrelated-user-file"),
            root.join(".vela-pending-symlink"),
        )
        .unwrap();
    }
    fixture.media.initialize().await.unwrap();
    for directory in ["media", "artwork"] {
        let root = fixture.directory.path().join(directory);
        assert!(!root.join(".vela-pending-abandoned").exists());
        assert_eq!(
            tokio::fs::read(root.join("unrelated-user-file"))
                .await
                .unwrap(),
            b"preserve this"
        );
        assert!(root.join(".vela-pending-directory").is_dir());
        #[cfg(unix)]
        assert!(root.join(".vela-pending-symlink").is_symlink());
    }
    assert_eq!(tokio::fs::read(asset.path).await.unwrap(), FLAC);
    assert!(artwork.exists());
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 1);
}
