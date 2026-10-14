//! SQL catalog fixtures test grouping semantics; the ingestion test uses real FLAC.
use std::time::Duration;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tempfile::TempDir;
use uuid::Uuid;
use vela_media::{InvalidCatalogCursor, MediaService};

struct Fixture {
    directory: TempDir,
    pool: SqlitePool,
    media: MediaService,
}

struct TrackInput<'a> {
    owner: &'a str,
    artist: &'a str,
    album: &'a str,
    album_artist: &'a str,
    year: Option<i64>,
    disc: Option<i64>,
    number: Option<i64>,
    artwork: Option<&'a str>,
}

impl Default for TrackInput<'_> {
    fn default() -> Self {
        Self {
            owner: "alice",
            artist: "Ensemble",
            album: "Concert",
            album_artist: "",
            year: Some(2026),
            disc: None,
            number: None,
            artwork: None,
        }
    }
}

impl Fixture {
    async fn legacy() -> Self {
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
        Self {
            directory,
            pool,
            media,
        }
    }

    async fn new() -> Self {
        let fixture = Self::legacy().await;
        fixture.migrate_catalog().await;
        fixture
    }

    async fn migrate_catalog(&self) {
        sqlx::raw_sql(include_str!("../../../migrations/0002_catalog_groups.sql"))
            .execute(&self.pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/0003_catalog_metadata_revision.sql"
        ))
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn add(&self, input: TrackInput<'_>) -> String {
        let id = Uuid::new_v4().to_string();
        let hash = blake3::hash(id.as_bytes()).to_hex().to_string();
        // These rows model existing metadata; they do not claim audio validation.
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',8)")
            .bind(&id).bind(&hash).bind(format!("media/{hash}.audio"))
            .execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,album_artist,year,disc_number,track_number,duration_ms,codec,artwork_id) VALUES(?,?,?,'Movement',?,?,?,?,?,?,1000,'FLAC',?)")
            .bind(&id).bind(input.owner).bind(&id).bind(input.artist).bind(input.album)
            .bind(input.album_artist).bind(input.year).bind(input.disc).bind(input.number).bind(input.artwork)
            .execute(&self.pool).await.unwrap();
        id
    }

    async fn cover(&self) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO artwork(id,hash,storage_path) VALUES(?,?,?)")
            .bind(&id)
            .bind(blake3::hash(id.as_bytes()).to_hex().to_string())
            .bind("artwork/test.jpg")
            .execute(&self.pool)
            .await
            .unwrap();
        id
    }
}

#[tokio::test]
async fn real_ingestion_keeps_group_ids_stable_across_duplicates_and_owners() {
    let fixture = Fixture::new().await;
    let path = fixture.directory.path().join("staging/real.flac");
    tokio::fs::write(&path, include_bytes!("fixtures/signal.flac"))
        .await
        .unwrap();
    let first = fixture
        .media
        .ingest("alice", &path, "real.flac")
        .await
        .unwrap();
    let album = fixture
        .media
        .list_albums("alice", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    let artist = fixture
        .media
        .list_artists("alice", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    assert_eq!(album.title, "Real Fixtures");
    assert_eq!(album.artist, "Vela Test");
    assert_eq!(album.year, Some(2026));
    assert_eq!(album.track_count, 1);
    assert_eq!(album.duration_ms, first.track.duration_ms);
    assert_eq!(album.artwork_id, first.track.artwork_id);
    assert_eq!(artist.track_count, 1);
    assert_eq!(artist.album_count, 1);
    assert_eq!(artist.artwork_id, first.track.artwork_id);

    let duplicate = fixture
        .media
        .ingest("alice", &path, "renamed.flac")
        .await
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.track.id, first.track.id);
    assert_eq!(
        fixture
            .media
            .list_albums("alice", None, 50)
            .await
            .unwrap()
            .items[0]
            .id,
        album.id
    );
    assert_eq!(
        fixture
            .media
            .list_artists("alice", None, 50)
            .await
            .unwrap()
            .items[0]
            .id,
        artist.id
    );

    fixture
        .media
        .ingest("bob", &path, "real.flac")
        .await
        .unwrap();
    let bob_album = fixture
        .media
        .list_albums("bob", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    let bob_artist = fixture
        .media
        .list_artists("bob", None, 50)
        .await
        .unwrap()
        .items
        .remove(0);
    assert_ne!(bob_album.id, album.id);
    assert_ne!(bob_artist.id, artist.id);
    assert!(
        fixture
            .media
            .album("bob", &album.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .artist("bob", &artist.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .album_tracks("bob", &album.id, None, 50)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .artist_tracks("bob", &artist.id, None, 50)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .album_tracks("alice", &bob_album.id, Some("bad cursor"), 50)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn album_pages_follow_musical_disc_order_and_validate_cursor_membership() {
    let fixture = Fixture::new().await;
    let artwork = fixture.cover().await;
    let late = fixture
        .add(TrackInput {
            disc: Some(2),
            number: Some(1),
            ..TrackInput::default()
        })
        .await;
    let unknown = fixture.add(TrackInput::default()).await;
    let second = fixture
        .add(TrackInput {
            disc: Some(1),
            number: Some(2),
            ..TrackInput::default()
        })
        .await;
    let first = fixture
        .add(TrackInput {
            number: Some(1),
            artwork: Some(&artwork),
            ..TrackInput::default()
        })
        .await;
    let other = fixture
        .add(TrackInput {
            album: "Other album",
            ..TrackInput::default()
        })
        .await;
    let private = fixture
        .add(TrackInput {
            owner: "bob",
            ..TrackInput::default()
        })
        .await;
    fixture
        .media
        .set_favorite("alice", &second, true)
        .await
        .unwrap();
    let album = fixture
        .media
        .list_albums("alice", None, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|album| album.title == "Concert")
        .unwrap();
    assert_eq!(album.track_count, 4);
    assert_eq!(album.duration_ms, 4000);
    assert_eq!(album.artwork_id.as_deref(), Some(artwork.as_str()));

    let initial = fixture
        .media
        .album_tracks("alice", &album.id, None, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        initial
            .items
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        vec![first.as_str(), second.as_str()]
    );
    assert!(initial.items[1].favorite);
    assert_eq!(initial.next_cursor.as_deref(), Some(second.as_str()));
    let next = fixture
        .media
        .album_tracks("alice", &album.id, initial.next_cursor.as_deref(), 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        next.items
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        vec![unknown.as_str(), late.as_str()]
    );
    assert!(next.next_cursor.is_none());
    let empty = fixture
        .media
        .album_tracks("alice", &album.id, Some(&late), 100)
        .await
        .unwrap()
        .unwrap();
    assert!(empty.items.is_empty());
    let zero_limit = fixture
        .media
        .album_tracks("alice", &album.id, None, 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(zero_limit.items.len(), 1);
    let missing = Uuid::nil().to_string();
    for cursor in [
        "not a UUID",
        other.as_str(),
        private.as_str(),
        missing.as_str(),
    ] {
        let error = fixture
            .media
            .album_tracks("alice", &album.id, Some(cursor), 2)
            .await
            .unwrap_err();
        assert!(error.is::<InvalidCatalogCursor>());
    }
}

#[tokio::test]
async fn artist_associations_include_album_credit_without_double_counting_tracks() {
    let fixture = Fixture::new().await;
    let cover = fixture.cover().await;
    let guest = fixture
        .add(TrackInput {
            artist: "Guest",
            album_artist: "Various Artists",
            artwork: Some(&cover),
            ..TrackInput::default()
        })
        .await;
    let both = fixture
        .add(TrackInput {
            artist: "Various Artists",
            album_artist: "Various Artists",
            ..TrackInput::default()
        })
        .await;
    let standalone = fixture
        .add(TrackInput {
            artist: "Various Artists",
            album: "",
            ..TrackInput::default()
        })
        .await;
    let unrelated = fixture
        .add(TrackInput {
            artist: "Other",
            album: "Separate",
            ..TrackInput::default()
        })
        .await;
    let private = fixture
        .add(TrackInput {
            owner: "bob",
            artist: "Various Artists",
            album_artist: "Various Artists",
            ..TrackInput::default()
        })
        .await;
    let artists = fixture.media.list_artists("alice", None, 50).await.unwrap();
    let various = artists
        .items
        .iter()
        .find(|artist| artist.name == "Various Artists")
        .unwrap();
    let guest_artist = artists
        .items
        .iter()
        .find(|artist| artist.name == "Guest")
        .unwrap();
    assert_eq!(various.track_count, 3);
    assert_eq!(various.album_count, 1);
    assert_eq!(various.artwork_id.as_deref(), Some(cover.as_str()));
    assert_eq!(guest_artist.track_count, 1);
    assert_eq!(guest_artist.album_count, 1);

    let mut expected = vec![guest, both, standalone];
    expected.sort();
    let first = fixture
        .media
        .artist_tracks("alice", &various.id, None, 1)
        .await
        .unwrap()
        .unwrap();
    let second = fixture
        .media
        .artist_tracks("alice", &various.id, first.next_cursor.as_deref(), 1)
        .await
        .unwrap()
        .unwrap();
    let third = fixture
        .media
        .artist_tracks("alice", &various.id, second.next_cursor.as_deref(), 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        vec![
            first.items[0].id.clone(),
            second.items[0].id.clone(),
            third.items[0].id.clone()
        ],
        expected
    );
    assert!(third.next_cursor.is_none());
    for cursor in [unrelated.as_str(), private.as_str(), "bad"] {
        let error = fixture
            .media
            .artist_tracks("alice", &various.id, Some(cursor), 10)
            .await
            .unwrap_err();
        assert!(error.is::<InvalidCatalogCursor>());
    }
    assert_eq!(fixture.media.stats("alice").await.unwrap().artists, 3);
}

#[tokio::test]
async fn metadata_updates_hide_orphans_and_reuse_persisted_group_identity() {
    let fixture = Fixture::new().await;
    let id = fixture.add(TrackInput::default()).await;
    fixture
        .add(TrackInput {
            artist: "ensemble",
            album: " concert ",
            ..TrackInput::default()
        })
        .await;
    fixture
        .add(TrackInput {
            year: Some(1999),
            ..TrackInput::default()
        })
        .await;
    fixture
        .add(TrackInput {
            album_artist: "Different album artist",
            ..TrackInput::default()
        })
        .await;
    assert_eq!(fixture.media.stats("alice").await.unwrap().albums, 3);
    assert_eq!(fixture.media.stats("alice").await.unwrap().artists, 2);
    let original = fixture
        .media
        .list_albums("alice", None, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|album| album.artist == "Ensemble" && album.year == Some(2026))
        .unwrap();
    assert_eq!(original.track_count, 2);
    sqlx::query("UPDATE tracks SET album='Changed',artist='New artist',album_artist='' WHERE id=?")
        .bind(&id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let changed = fixture
        .media
        .list_albums("alice", None, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|album| album.title == "Changed")
        .unwrap();
    let new_artist = fixture
        .media
        .list_artists("alice", None, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|artist| artist.name == "New artist")
        .unwrap();
    sqlx::query("UPDATE tracks SET album=' Concert ',artist=' ENSEMBLE ' WHERE id=?")
        .bind(&id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(
        fixture
            .media
            .album("alice", &original.id)
            .await
            .unwrap()
            .unwrap()
            .track_count,
        2
    );
    assert!(
        fixture
            .media
            .album("alice", &changed.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .media
            .artist("alice", &new_artist.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(fixture.media.stats("alice").await.unwrap().albums, 3);
    assert_eq!(fixture.media.stats("alice").await.unwrap().artists, 2);
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM albums WHERE owner_id='alice'")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(
        stored, 4,
        "hidden orphan identity remains available for later reuse"
    );
    sqlx::query("UPDATE tracks SET album='Changed',artist='New artist' WHERE id=?")
        .bind(&id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(
        fixture
            .media
            .album("alice", &changed.id)
            .await
            .unwrap()
            .unwrap()
            .track_count,
        1
    );
    assert_eq!(
        fixture
            .media
            .artist("alice", &new_artist.id)
            .await
            .unwrap()
            .unwrap()
            .track_count,
        1
    );
}

#[tokio::test]
async fn catalog_pages_are_bounded_and_database_rejects_cross_owner_relationships() {
    let fixture = Fixture::new().await;
    for index in 0..102 {
        fixture
            .add(TrackInput {
                artist: &format!("Artist {index}"),
                album: &format!("Album {index}"),
                ..TrackInput::default()
            })
            .await;
    }
    let albums = fixture
        .media
        .list_albums("alice", None, u32::MAX)
        .await
        .unwrap();
    assert_eq!(albums.items.len(), 100);
    let remaining = fixture
        .media
        .list_albums("alice", albums.next_cursor.as_deref(), 100)
        .await
        .unwrap();
    assert_eq!(remaining.items.len(), 2);
    assert!(remaining.next_cursor.is_none());
    assert!(albums.items.last().unwrap().id < remaining.items[0].id);
    let artists = fixture
        .media
        .list_artists("alice", None, u32::MAX)
        .await
        .unwrap();
    assert_eq!(artists.items.len(), 100);
    let remaining = fixture
        .media
        .list_artists("alice", artists.next_cursor.as_deref(), 100)
        .await
        .unwrap();
    assert_eq!(remaining.items.len(), 2);
    assert!(remaining.next_cursor.is_none());
    assert_eq!(
        fixture
            .media
            .list_albums("alice", None, 0)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        fixture
            .media
            .list_artists("alice", None, 0)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        fixture
            .media
            .list_albums("bob", None, 50)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        fixture
            .media
            .list_artists("bob", None, 50)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        fixture
            .media
            .list_albums("alice", Some("invalid"), 50)
            .await
            .unwrap_err()
            .is::<InvalidCatalogCursor>()
    );
    assert!(
        fixture
            .media
            .list_artists("alice", Some("invalid"), 50)
            .await
            .unwrap_err()
            .is::<InvalidCatalogCursor>()
    );
    let bob_track = fixture
        .add(TrackInput {
            owner: "bob",
            ..TrackInput::default()
        })
        .await;
    assert!(
        sqlx::query("UPDATE tracks SET artist_id=? WHERE id=?")
            .bind(&artists.items[0].id)
            .bind(&bob_track)
            .execute(&fixture.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE tracks SET album_id=? WHERE id=?")
            .bind(&albums.items[0].id)
            .bind(&bob_track)
            .execute(&fixture.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn migration_backfills_existing_tags_without_exposing_another_owner() {
    let fixture = Fixture::legacy().await;
    let first = fixture
        .add(TrackInput {
            artist: " Guest ",
            album: " Anthology ",
            album_artist: " Various Artists ",
            year: None,
            ..TrackInput::default()
        })
        .await;
    fixture
        .add(TrackInput {
            artist: "guest",
            album: "anthology",
            album_artist: "various artists",
            year: None,
            ..TrackInput::default()
        })
        .await;
    fixture
        .add(TrackInput {
            owner: "bob",
            artist: "Guest",
            album: "Anthology",
            album_artist: "Various Artists",
            year: None,
            ..TrackInput::default()
        })
        .await;
    fixture
        .add(TrackInput {
            artist: "  ",
            album: "  ",
            ..TrackInput::default()
        })
        .await;
    fixture.migrate_catalog().await;
    let albums = fixture.media.list_albums("alice", None, 50).await.unwrap();
    assert_eq!(albums.items.len(), 1);
    let album = &albums.items[0];
    assert_eq!(album.title.to_ascii_lowercase(), "anthology");
    assert_eq!(album.artist.to_ascii_lowercase(), "various artists");
    assert_eq!(album.year, None);
    assert_eq!(album.track_count, 2);
    assert_eq!(fixture.media.stats("alice").await.unwrap().artists, 3);
    let artists = fixture.media.list_artists("alice", None, 50).await.unwrap();
    assert!(
        artists
            .items
            .iter()
            .any(|artist| artist.name == "Unknown artist"
                && artist.track_count == 1
                && artist.album_count == 0)
    );
    assert!(
        fixture
            .media
            .album("bob", &album.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture
            .media
            .album_tracks("alice", &album.id, None, 50)
            .await
            .unwrap()
            .unwrap()
            .items
            .len(),
        2
    );
    assert!(
        fixture
            .media
            .get_track("alice", &first)
            .await
            .unwrap()
            .unwrap()
            .artist
            .starts_with(' '),
        "original source metadata remains intact"
    );
}
