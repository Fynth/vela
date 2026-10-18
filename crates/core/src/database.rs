use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{path::Path, time::Duration};

pub async fn open(data_dir: &Path) -> anyhow::Result<SqlitePool> {
    tokio::fs::create_dir_all(data_dir).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700)).await?;
    }
    let options = SqliteConnectOptions::new()
        .filename(data_dir.join("vela.db"))
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .connect_with(options)
        .await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}

/// Held for the lifetime of one server process; prevents competing startup recovery.
pub struct DataLock {
    _file: std::fs::File,
}
impl DataLock {
    pub async fn acquire(data_dir: &Path) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(data_dir).await?;
        let path = data_dir.join(".vela.lock");
        tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)?;
            fs2::FileExt::try_lock_exclusive(&file).map_err(|_| {
                anyhow::anyhow!("Another Vela process already owns this data directory")
            })?;
            Ok(Self { _file: file })
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn embedded_schema_contains_filter_indexes_and_covering_upgrade() {
        let directory = tempfile::tempdir().unwrap();
        let pool = super::open(directory.path()).await.unwrap();
        let applied: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM _sqlx_migrations WHERE version IN (5,6) AND success=1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(applied, 2, "incremental builds must embed new migrations");
        let columns = sqlx::query("PRAGMA index_info(tracks_owner_genre_filter)")
            .fetch_all(&pool)
            .await
            .unwrap();
        use sqlx::Row;
        assert!(
            columns
                .iter()
                .any(|row| row.get::<Option<String>, _>("name").as_deref() == Some("genre"))
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn catalog_upgrade_preserves_legacy_tracks_and_builds_private_stable_groups() {
        use sqlx::Row;
        let dir = tempfile::tempdir().unwrap();
        let baseline = tempfile::tempdir().unwrap();
        tokio::fs::write(
            baseline.path().join("0001_initial.sql"),
            include_str!("../../../migrations/0001_initial.sql"),
        )
        .await
        .unwrap();
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("vela.db"))
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = sqlx::SqlitePool::connect_with(options).await.unwrap();
        sqlx::migrate::Migrator::new(baseline.path())
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        sqlx::raw_sql("INSERT INTO users(id,username,password_hash) VALUES('owner','owner','fixture'),('other','other','fixture'); INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES('asset','hash','media/hash.audio','audio/flac',100);").execute(&pool).await.unwrap();
        for (id, artist, album_artist, album, year, owner) in [
            ("one", "Solo", "", "First", 2000, "owner"),
            ("two", " solo ", "", " first ", 2000, "owner"),
            ("three", "Solo", "", "First", 2001, "owner"),
            (
                "four",
                "Guest",
                "Various Artists",
                "Collection",
                2000,
                "owner",
            ),
            ("five", "Solo", "", "", 0, "owner"),
            ("six", "Solo", "", "First", 2000, "other"),
        ] {
            // Separate legacy assets allow repeated artist/album tags.
            sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',100)").bind(id).bind(id).bind(format!("media/{id}.audio")).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album_artist,album,year,duration_ms,codec) VALUES(?,?,?,?,?,?,?,?,1000,'FLAC')")
                .bind(id).bind(owner).bind(id).bind(id).bind(artist).bind(album_artist).bind(album).bind(year).execute(&pool).await.unwrap();
        }
        pool.close().await;
        let pool = super::open(dir.path()).await.unwrap();
        let rows = sqlx::query("SELECT id,artist_id,album_id FROM tracks ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 6);
        let ids = |id: &str| {
            let row = rows
                .iter()
                .find(|r| r.get::<String, _>("id") == id)
                .unwrap();
            (
                row.get::<String, _>("artist_id"),
                row.get::<Option<String>, _>("album_id"),
            )
        };
        assert_eq!(ids("one"), ids("two"));
        assert_ne!(ids("one").1, ids("three").1);
        assert_ne!(ids("one"), ids("six"));
        assert!(ids("five").1.is_none());
        let grouping = ids("one");
        // Catalog reassociation and initial inserts must leave FTS intact.
        sqlx::query("INSERT INTO tracks_fts(tracks_fts,rank) VALUES('integrity-check',1)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE tracks SET title='renamed' WHERE id='one'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tracks_fts WHERE tracks_fts MATCH 'renamed'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert!(
            sqlx::query("UPDATE tracks SET album_id=? WHERE id='one'")
                .bind(ids("six").1)
                .execute(&pool)
                .await
                .is_err()
        );
        let artists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artists")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES('new','new','media/new.audio','audio/flac',100)").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,year,duration_ms,codec) VALUES('new','owner','new','New song','SOLO','FIRST',2000,1000,'FLAC')").execute(&pool).await.unwrap();
        let new = sqlx::query("SELECT artist_id,album_id FROM tracks WHERE id='new'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            (
                new.get::<String, _>("artist_id"),
                new.get::<Option<String>, _>("album_id")
            ),
            grouping
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM artists")
                .fetch_one(&pool)
                .await
                .unwrap(),
            artists
        );
        sqlx::query("INSERT INTO tracks_fts(tracks_fts,rank) VALUES('integrity-check',1)")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        super::open(dir.path()).await.unwrap().close().await;
    }
    #[tokio::test]
    async fn schema_seven_upgrade_preserves_catalog_edits_and_playlist_order() {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let directory = tempfile::tempdir().unwrap();
        let legacy = tempfile::tempdir().unwrap();
        let migrations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
        for entry in std::fs::read_dir(migrations).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_str().unwrap();
            if name.ends_with(".sql") && name[..4].parse::<u32>().unwrap() <= 7 {
                std::fs::copy(entry.path(), legacy.path().join(name)).unwrap();
            }
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(directory.path().join("vela.db"))
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        sqlx::migrate::Migrator::new(legacy.path())
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        sqlx::raw_sql("INSERT INTO users(id,username,password_hash) VALUES('owner','owner','fixture');
            INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES('asset','unchanged-hash','media/original','audio/flac',1234);
            INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,duration_ms,codec,metadata_revision) VALUES('track','owner','asset','Edited title','Artist','Album',1000,'FLAC',7);
            INSERT INTO playlists(id,owner_id,name,revision) VALUES('playlist','owner','Old private playlist',4);
            INSERT INTO playlist_items(id,playlist_id,track_id,position) VALUES('entry','playlist','track',12);
            INSERT INTO favorites(user_id,track_id) VALUES('owner','track');").execute(&pool).await.unwrap();
        pool.close().await;
        let upgraded = super::open(directory.path()).await.unwrap();
        let track: (String, String, i64) = sqlx::query_as(
            "SELECT title,media_asset_id,metadata_revision FROM tracks WHERE id='track'",
        )
        .fetch_one(&upgraded)
        .await
        .unwrap();
        assert_eq!(track, ("Edited title".into(), "asset".into(), 7));
        let playlist: (String,i64,i64) = sqlx::query_as("SELECT p.name,p.revision,i.position FROM playlists p JOIN playlist_items i ON i.playlist_id=p.id WHERE p.id='playlist'").fetch_one(&upgraded).await.unwrap();
        assert_eq!(playlist, ("Old private playlist".into(), 4, 12));
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT hash FROM media_assets WHERE id='asset'")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            "unchanged-hash"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM favorites")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM listening_preferences")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            0,
            "upgrade must not opt anyone in"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audio_analysis")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            0,
            "upgrade must not automatically queue analysis"
        );
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&upgraded)
                .await
                .unwrap()
                .is_empty()
        );
        upgraded.close().await;
        super::open(directory.path()).await.unwrap().close().await;
    }
    #[tokio::test]
    async fn schema_nine_upgrade_preserves_private_signals_without_adding_exclusions() {
        use sqlx::{
            Row,
            sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        };
        let directory = tempfile::tempdir().unwrap();
        let legacy = tempfile::tempdir().unwrap();
        let migrations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
        for entry in std::fs::read_dir(migrations).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_str().unwrap();
            if name.ends_with(".sql") && name[..4].parse::<u32>().unwrap() <= 9 {
                std::fs::copy(entry.path(), legacy.path().join(name)).unwrap();
            }
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(directory.path().join("vela.db"))
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        sqlx::migrate::Migrator::new(legacy.path())
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        sqlx::raw_sql("INSERT INTO users(id,username,password_hash) VALUES('owner','owner','fixture');
            INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES('asset','old-hash','media/original','audio/flac',1234);
            INSERT INTO tracks(id,owner_id,media_asset_id,title,duration_ms,codec,metadata_revision) VALUES('track','owner','asset','Preserved',1000,'FLAC',3);
            INSERT INTO playlists(id,owner_id,name,revision) VALUES('playlist','owner','Preserved list',5);
            INSERT INTO playlist_items(id,playlist_id,track_id,position) VALUES('entry','playlist','track',42);
            INSERT INTO listening_preferences(user_id,enabled,retention_days,revision,updated_at) VALUES('owner',1,90,7,111);
            INSERT INTO listening_devices(user_id,device_id,enabled) VALUES('owner','device',1);
            INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES('owner','session','device','track',2,1000,1000,100,111,1,1);
            INSERT INTO audio_analysis(media_asset_id,extractor_version,status,features_json,created_at,updated_at) VALUES('asset','legacy-extractor','complete','{}',100,111);").execute(&pool).await.unwrap();
        let tables = [
            "users",
            "media_assets",
            "tracks",
            "playlists",
            "playlist_items",
            "listening_preferences",
            "listening_devices",
            "listening_sessions",
            "audio_analysis",
        ];
        let mut snapshots = Vec::new();
        for table in tables {
            let columns = sqlx::query(&format!("PRAGMA table_info({table})"))
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| {
                    format!(
                        "quote(\"{}\")",
                        row.get::<String, _>("name").replace('"', "\"\"")
                    )
                })
                .collect::<Vec<_>>()
                .join(" || '|' || ");
            let query = format!("SELECT {columns} FROM {table} ORDER BY rowid");
            let rows = sqlx::query_scalar::<_, String>(&query)
                .fetch_all(&pool)
                .await
                .unwrap();
            snapshots.push((query, rows));
        }
        pool.close().await;
        let upgraded = super::open(directory.path()).await.unwrap();
        for (query, rows) in snapshots {
            assert_eq!(
                sqlx::query_scalar::<_, String>(&query)
                    .fetch_all(&upgraded)
                    .await
                    .unwrap(),
                rows,
                "migration changed existing logical rows"
            );
        }
        let consent: (i64, i64) = sqlx::query_as(
            "SELECT enabled,revision FROM listening_preferences WHERE user_id='owner'",
        )
        .fetch_one(&upgraded)
        .await
        .unwrap();
        assert_eq!(consent, (1, 7));
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT title FROM tracks WHERE id='track'")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            "Preserved"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT position FROM playlist_items WHERE id='entry'")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            42
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM recommendation_exclusions")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            0
        );
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&upgraded)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn schema_ten_upgrade_preserves_all_private_rows_without_publishing_anything() {
        use sqlx::{
            Row,
            sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        };
        let directory = tempfile::tempdir().unwrap();
        let legacy = tempfile::tempdir().unwrap();
        let migrations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
        for entry in std::fs::read_dir(migrations).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_str().unwrap();
            if name.ends_with(".sql") && name[..4].parse::<u32>().unwrap() <= 10 {
                std::fs::copy(entry.path(), legacy.path().join(name)).unwrap();
            }
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(directory.path().join("vela.db"))
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        sqlx::migrate::Migrator::new(legacy.path())
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        sqlx::raw_sql("INSERT INTO users(id,username,password_hash) VALUES('owner','owner','fixture');
            INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES('asset','old-hash','media/original','audio/flac',1234);
            INSERT INTO tracks(id,owner_id,media_asset_id,title,duration_ms,codec,metadata_revision) VALUES('track','owner','asset','Preserved',1000,'FLAC',3);
            INSERT INTO playlists(id,owner_id,name,revision) VALUES('playlist','owner','Preserved list',5);
            INSERT INTO playlist_items(id,playlist_id,track_id,position) VALUES('entry','playlist','track',42);
            INSERT INTO listening_preferences(user_id,enabled,retention_days,revision,updated_at) VALUES('owner',1,90,7,111);
            INSERT INTO listening_devices(user_id,device_id,enabled) VALUES('owner','device',1);
            INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES('owner','session','device','track',2,1000,1000,100,111,1,1);
            INSERT INTO audio_analysis(media_asset_id,extractor_version,status,features_json,created_at,updated_at) VALUES('asset','legacy-extractor','complete','{}',100,111);").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO recommendation_exclusions(user_id,track_id,created_at) VALUES('owner','track',111)").execute(&pool).await.unwrap();
        let tables = [
            "users",
            "media_assets",
            "tracks",
            "playlists",
            "playlist_items",
            "listening_preferences",
            "listening_devices",
            "listening_sessions",
            "audio_analysis",
            "recommendation_exclusions",
        ];
        let mut snapshots = Vec::new();
        for table in tables {
            let columns = sqlx::query(&format!("PRAGMA table_info({table})"))
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| {
                    format!(
                        "quote(\"{}\")",
                        row.get::<String, _>("name").replace('"', "\"\"")
                    )
                })
                .collect::<Vec<_>>()
                .join(" || '|' || ");
            let query = format!("SELECT {columns} FROM {table} ORDER BY rowid");
            let rows = sqlx::query_scalar::<_, String>(&query)
                .fetch_all(&pool)
                .await
                .unwrap();
            snapshots.push((query, rows));
        }
        pool.close().await;
        let upgraded = super::open(directory.path()).await.unwrap();
        for (query, rows) in snapshots {
            assert_eq!(
                sqlx::query_scalar::<_, String>(&query)
                    .fetch_all(&upgraded)
                    .await
                    .unwrap(),
                rows,
                "migration changed existing logical rows"
            );
        }
        let consent: (i64, i64) = sqlx::query_as(
            "SELECT enabled,revision FROM listening_preferences WHERE user_id='owner'",
        )
        .fetch_one(&upgraded)
        .await
        .unwrap();
        assert_eq!(consent, (1, 7));
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT title FROM tracks WHERE id='track'")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            "Preserved"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT position FROM playlist_items WHERE id='entry'")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            42
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM publications")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM network_settings")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            0
        );
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&upgraded)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn migrations_are_repeatable_and_foreign_keys_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let pool = super::open(dir.path()).await.unwrap();
        assert!(
            sqlx::query("INSERT INTO favorites(user_id,track_id) VALUES('missing','missing')")
                .execute(&pool)
                .await
                .is_err()
        );
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal, "wal");
        pool.close().await;
        super::open(dir.path()).await.unwrap().close().await;
    }
}
