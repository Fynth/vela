//! Catalog editing runs against real ingested FLAC and an isolated SQLite file.
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tempfile::TempDir;
use vela_domain::{Track, TrackMetadata, UpdateTrackMetadata};
use vela_media::{CatalogMetadataError, MediaService};

const FLAC: &[u8] = include_bytes!("fixtures/signal.flac");

struct Fixture {
    directory: TempDir,
    pool: SqlitePool,
    media: MediaService,
    track: Track,
    staged: PathBuf,
}

async fn connect(path: &Path) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .unwrap()
}

impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let pool = connect(&directory.path().join("vela.db")).await;
        for migration in [
            include_str!("../../../migrations/0001_initial.sql"),
            include_str!("../../../migrations/0002_catalog_groups.sql"),
            include_str!("../../../migrations/0003_catalog_metadata_revision.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        for owner in ["alice", "bob"] {
            sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,?)")
                .bind(owner)
                .bind(owner)
                .bind("test hash")
                .execute(&pool)
                .await
                .unwrap();
        }
        let media = MediaService::new(pool.clone(), directory.path().to_owned(), 1024 * 1024);
        media.initialize().await.unwrap();
        let staged = directory.path().join("staging/real.flac");
        tokio::fs::write(&staged, FLAC).await.unwrap();
        let track = media
            .ingest("alice", &staged, "real.flac")
            .await
            .unwrap()
            .track;
        Self {
            directory,
            pool,
            media,
            track,
            staged,
        }
    }

    async fn snapshot(&self) -> TrackMetadata {
        self.media
            .metadata("alice", &self.track.id)
            .await
            .unwrap()
            .unwrap()
    }

    async fn audit_count(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_events WHERE action='track.metadata_updated'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }
}

fn edit(snapshot: &TrackMetadata) -> UpdateTrackMetadata {
    let track = &snapshot.track;
    UpdateTrackMetadata {
        revision: snapshot.revision,
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        album_artist: track.album_artist.clone(),
        genre: track.genre.clone(),
        year: track.year,
        track_number: track.track_number,
        disc_number: track.disc_number,
    }
}

#[tokio::test]
async fn actual_catalog_edit_regroups_search_preserves_assets_and_survives_restart() {
    let fixture = Fixture::new().await;
    let initial = fixture.snapshot().await;
    assert_eq!(initial.revision, 0);
    let asset = fixture
        .media
        .asset("alice", &fixture.track.id)
        .await
        .unwrap()
        .unwrap();
    let old_album = fixture
        .media
        .list_albums("alice", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    let old_artist = fixture
        .media
        .list_artists("alice", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    let bob = fixture
        .media
        .ingest("bob", &fixture.staged, "same.flac")
        .await
        .unwrap()
        .track;
    fixture
        .media
        .set_favorite("alice", &fixture.track.id, true)
        .await
        .unwrap();
    let mut input = edit(&initial);
    input.title = "  Renamed movement  ".into();
    input.artist = "  Guest Soloist  ".into();
    input.album = "  Chamber anthology  ".into();
    input.album_artist = "  Chamber Ensemble  ".into();
    input.genre = "  Classical  ".into();
    input.year = Some(2001);
    input.track_number = Some(3);
    input.disc_number = Some(2);
    let updated = fixture
        .media
        .update_metadata("alice", &fixture.track.id, input)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.revision, 1);
    assert_eq!(updated.track.title, "Renamed movement");
    assert_eq!(updated.track.artist, "Guest Soloist");
    assert_eq!(updated.track.album, "Chamber anthology");
    assert_eq!(updated.track.album_artist, "Chamber Ensemble");
    assert_eq!(updated.track.genre, "Classical");
    assert_eq!(updated.track.year, Some(2001));
    assert_eq!(updated.track.track_number, Some(3));
    assert_eq!(updated.track.disc_number, Some(2));
    assert!(updated.track.favorite);
    assert_eq!(updated.track.id, initial.track.id);
    assert_eq!(updated.track.codec, initial.track.codec);
    assert_eq!(updated.track.duration_ms, initial.track.duration_ms);
    assert_eq!(updated.track.sample_rate, initial.track.sample_rate);
    assert_eq!(updated.track.bitrate, initial.track.bitrate);
    assert_eq!(updated.track.bit_depth, initial.track.bit_depth);
    assert_eq!(updated.track.channels, initial.track.channels);
    assert_eq!(updated.track.size_bytes, initial.track.size_bytes);
    assert_eq!(updated.track.artwork_id, initial.track.artwork_id);
    assert_eq!(updated.track.added_at, initial.track.added_at);
    assert!(
        fixture
            .media
            .list_tracks("alice", Some("Signal"), None, 50, false)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let search = fixture
        .media
        .list_tracks("alice", Some("Renamed"), None, 50, false)
        .await
        .unwrap();
    assert_eq!(search.items.len(), 1);
    assert_eq!(search.items[0].id, fixture.track.id);
    assert!(
        fixture
            .media
            .album("alice", &old_album.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .artist("alice", &old_artist.id)
            .await
            .unwrap()
            .is_none()
    );
    let new_album = fixture
        .media
        .list_albums("alice", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    assert_eq!(new_album.title, "Chamber anthology");
    assert_eq!(new_album.artist, "Chamber Ensemble");
    assert_eq!(new_album.year, Some(2001));
    assert_eq!(fixture.media.stats("alice").await.unwrap().artists, 2);
    assert_eq!(
        fixture
            .media
            .metadata("bob", &bob.id)
            .await
            .unwrap()
            .unwrap()
            .track
            .title,
        "Signal"
    );
    let current_asset = fixture
        .media
        .asset("alice", &fixture.track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current_asset.hash, asset.hash);
    assert_eq!(current_asset.path, asset.path);
    assert_eq!(tokio::fs::read(&asset.path).await.unwrap(), FLAC);
    assert_eq!(tokio::fs::read(&fixture.staged).await.unwrap(), FLAC);
    let repeated = fixture
        .media
        .ingest("alice", &fixture.staged, "again.flac")
        .await
        .unwrap();
    assert!(repeated.duplicate);
    assert_eq!(
        repeated.track.title, "Renamed movement",
        "embedded tags never overwrite a catalog edit during deduplication"
    );
    assert_eq!(fixture.snapshot().await.revision, 1);
    let audit = sqlx::query("SELECT actor_id,action,object_id FROM audit_events")
        .fetch_all(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].get::<String, _>("actor_id"), "alice");
    assert_eq!(
        audit[0].get::<String, _>("action"),
        "track.metadata_updated"
    );
    assert_eq!(audit[0].get::<String, _>("object_id"), fixture.track.id);

    fixture.pool.close().await;
    let reopened_pool = connect(&fixture.directory.path().join("vela.db")).await;
    let reopened = MediaService::new(
        reopened_pool.clone(),
        fixture.directory.path().to_owned(),
        1024 * 1024,
    );
    let persisted = reopened
        .metadata("alice", &fixture.track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted.revision, 1);
    assert_eq!(persisted.track.title, "Renamed movement");
    assert_eq!(persisted.track.album_artist, "Chamber Ensemble");
    assert!(persisted.track.favorite);
    assert_eq!(
        reopened
            .album("alice", &new_album.id)
            .await
            .unwrap()
            .unwrap()
            .track_count,
        1
    );
    let persisted_asset = reopened
        .asset("alice", &fixture.track.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted_asset.hash, asset.hash);
    assert_eq!(tokio::fs::read(persisted_asset.path).await.unwrap(), FLAC);
    reopened_pool.close().await;
}

#[tokio::test]
async fn competing_actual_metadata_edits_conflict_without_losing_the_winner() {
    let fixture = Fixture::new().await;
    let initial = fixture.snapshot().await;
    let mut first_input = edit(&initial);
    first_input.title = "First edit".into();
    let mut second_input = edit(&initial);
    second_input.title = "Second edit".into();
    let (first, second) = tokio::join!(
        fixture
            .media
            .update_metadata("alice", &fixture.track.id, first_input),
        fixture
            .media
            .update_metadata("alice", &fixture.track.id, second_input),
    );
    let (winner, rejected, later_title) = match (first, second) {
        (Ok(Some(winner)), Err(error)) => (winner, error, "Second edit"),
        (Err(error), Ok(Some(winner))) => (winner, error, "First edit"),
        results => panic!("expected one success and one conflict, got {results:?}"),
    };
    assert!(matches!(
        rejected.downcast_ref::<CatalogMetadataError>(),
        Some(CatalogMetadataError::Conflict)
    ));
    assert_eq!(winner.revision, 1);
    assert_eq!(fixture.snapshot().await.track.title, winner.track.title);
    assert_eq!(fixture.audit_count().await, 1);
    let mut retry = edit(&fixture.snapshot().await);
    retry.title = later_title.into();
    let updated = fixture
        .media
        .update_metadata("alice", &fixture.track.id, retry)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.track.title, later_title);
    assert_eq!(fixture.audit_count().await, 2);
    assert_eq!(
        tokio::fs::read(
            fixture
                .media
                .asset("alice", &fixture.track.id)
                .await
                .unwrap()
                .unwrap()
                .path
        )
        .await
        .unwrap(),
        FLAC
    );
}

#[tokio::test]
async fn ownership_precedes_validation_and_playlist_editor_cannot_change_source_metadata() {
    let fixture = Fixture::new().await;
    let initial = fixture.snapshot().await;
    let playlist_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO playlists(id,owner_id,name) VALUES(?,'alice','Private playlist')")
        .bind(&playlist_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO playlist_collaborators(playlist_id,user_id,can_edit) VALUES(?,'bob',1)",
    )
    .bind(&playlist_id)
    .execute(&fixture.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO playlist_items(id,playlist_id,track_id,position) VALUES(?,?,?,0)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&playlist_id)
        .bind(&fixture.track.id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let mut invalid = edit(&initial);
    invalid.title = "\n".into();
    invalid.revision = -1;
    assert!(
        fixture
            .media
            .metadata("bob", &fixture.track.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .update_metadata("bob", &fixture.track.id, invalid.clone())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .update_metadata("alice", &uuid::Uuid::new_v4().to_string(), invalid.clone())
            .await
            .unwrap()
            .is_none()
    );
    let error = fixture
        .media
        .update_metadata("alice", &fixture.track.id, invalid)
        .await
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<CatalogMetadataError>(),
        Some(CatalogMetadataError::Invalid(_))
    ));
    assert_eq!(fixture.snapshot().await.revision, 0);
    assert_eq!(fixture.snapshot().await.track.title, "Signal");
    assert_eq!(fixture.audit_count().await, 0);
}

#[tokio::test]
async fn invalid_metadata_is_atomic_and_utf8_byte_limits_and_unknown_fields_work() {
    let fixture = Fixture::new().await;
    let initial = fixture.snapshot().await;
    let base = edit(&initial);
    let mut invalid = Vec::new();
    let mut value = base.clone();
    value.title = "  ".into();
    invalid.push(value);
    let mut value = base.clone();
    value.title = "😀".repeat(129);
    invalid.push(value);
    let mut value = base.clone();
    value.title = "\u{0000}Title".into();
    invalid.push(value);
    let mut value = base.clone();
    value.artist = "Artist\n".into();
    invalid.push(value);
    let mut value = base.clone();
    value.album = "😀".repeat(257);
    invalid.push(value);
    let mut value = base.clone();
    value.album_artist = "\u{0085}Artist".into();
    invalid.push(value);
    let mut value = base.clone();
    value.genre = "\rGenre".into();
    invalid.push(value);
    let mut value = base.clone();
    value.revision = -1;
    invalid.push(value);
    for number in [0, -1, 10000, i64::MAX] {
        let mut value = base.clone();
        value.year = Some(number);
        invalid.push(value);
        let mut value = base.clone();
        value.track_number = Some(number);
        invalid.push(value);
        let mut value = base.clone();
        value.disc_number = Some(number);
        invalid.push(value);
    }
    for input in invalid {
        let error = fixture
            .media
            .update_metadata("alice", &fixture.track.id, input)
            .await
            .unwrap_err();
        assert!(
            matches!(
                error.downcast_ref::<CatalogMetadataError>(),
                Some(CatalogMetadataError::Invalid(_))
            ),
            "unexpected error: {error:#}"
        );
    }
    assert_eq!(fixture.snapshot().await.revision, 0);
    assert_eq!(fixture.snapshot().await.track.title, "Signal");
    assert_eq!(fixture.audit_count().await, 0);
    let mut allowed = base;
    allowed.title = "😀".repeat(128);
    allowed.artist = "😀".repeat(256);
    allowed.album.clear();
    allowed.album_artist.clear();
    allowed.genre.clear();
    allowed.year = None;
    allowed.track_number = None;
    allowed.disc_number = None;
    let updated = fixture
        .media
        .update_metadata("alice", &fixture.track.id, allowed)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.revision, 1);
    assert_eq!(updated.track.title.len(), 512);
    assert_eq!(updated.track.artist.len(), 1024);
    assert_eq!(updated.track.year, None);
    assert_eq!(updated.track.track_number, None);
    assert_eq!(updated.track.disc_number, None);
    assert!(
        fixture
            .media
            .list_albums("alice", None, 50)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(fixture.media.stats("alice").await.unwrap().artists, 1);
    assert_eq!(tokio::fs::read(&fixture.staged).await.unwrap(), FLAC);
}
