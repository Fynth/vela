//! Private catalog access and bounded, content-addressed managed media ingestion.
mod catalog;
mod filters;
mod metadata;
mod publications;
mod range;
pub mod search;
mod validation;

pub use catalog::InvalidCatalogCursor;
pub use filters::InvalidTrackFilter;
pub use metadata::CatalogMetadataError;
pub use publications::PublicationError;
pub use range::{RangeError, select_range};

use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool, sqlite::SqliteRow};
use tokio::sync::Semaphore;
use uuid::Uuid;
use vela_domain::{LibraryStats, Track, UploadResult};

/// Internal streaming descriptor. Never serialize filesystem paths into an API.
#[derive(Debug, Clone)]
pub struct MediaAsset {
    pub path: PathBuf,
    pub content_type: String,
    pub size_bytes: u64,
    pub hash: String,
}

#[derive(Clone)]
pub struct MediaService {
    pool: SqlitePool,
    data_dir: PathBuf,
    max_file_bytes: u64,
    max_storage_bytes: u64,
    ingest_gate: Arc<Semaphore>,
}

const TRACK_SELECT: &str = "SELECT t.id,t.title,t.artist,t.album,t.album_artist,t.genre,t.year,t.track_number,t.disc_number,t.duration_ms,t.codec,t.bitrate,t.sample_rate,t.bit_depth,t.channels,a.size_bytes,t.artwork_id,t.added_at,(f.track_id IS NOT NULL) AS favorite FROM tracks t JOIN media_assets a ON a.id=t.media_asset_id LEFT JOIN favorites f ON f.track_id=t.id AND f.user_id=";

impl MediaService {
    pub fn new(pool: SqlitePool, data_dir: PathBuf, max_file_bytes: u64) -> Self {
        Self {
            pool,
            data_dir,
            max_file_bytes,
            max_storage_bytes: u64::MAX,
            ingest_gate: Arc::new(Semaphore::new(1)),
        }
    }

    /// Global managed-audio quota. Identical assets across owners count once.
    pub fn with_storage_budget(mut self, max_bytes: u64) -> Self {
        self.max_storage_bytes = max_bytes;
        self
    }

    pub async fn initialize(&self) -> Result<()> {
        let _permit = self
            .ingest_gate
            .acquire()
            .await
            .context("media ingestion is unavailable")?;
        for directory in ["staging", "media", "artwork"] {
            tokio::fs::create_dir_all(self.data_dir.join(directory))
                .await
                .context("could not initialize managed storage")?;
            if directory != "staging" {
                let mut entries = tokio::fs::read_dir(self.data_dir.join(directory)).await?;
                while let Some(entry) = entries.next_entry().await? {
                    // This prefix is reserved for our unpublished NamedTempFiles.
                    // Preserve final hashes, unrelated files, symlinks and directories.
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".vela-pending-")
                        && entry.file_type().await?.is_file()
                    {
                        tokio::fs::remove_file(entry.path())
                            .await
                            .context("could not remove abandoned media staging file")?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn ingest(
        &self,
        owner_id: &str,
        staged_path: &Path,
        original_filename: &str,
    ) -> Result<UploadResult> {
        let permit = self
            .ingest_gate
            .clone()
            .acquire_owned()
            .await
            .context("media ingestion is unavailable")?;
        let staging_root = self.data_dir.join("staging");
        let media_root = self.data_dir.join("media");
        let staged_path = staged_path.to_owned();
        let filename = original_filename.to_owned();
        let max_bytes = self.max_file_bytes;
        // The permit moves into the blocking worker so cancellation never starts a
        // second parser while the first parser is still running.
        let (prepared, _permit) = tokio::task::spawn_blocking(move || {
            validation::prepare(
                &staging_root,
                &media_root,
                &staged_path,
                &filename,
                max_bytes,
            )
            .map(|prepared| (prepared, permit))
        })
        .await
        .context("audio validation worker failed")??;

        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id=?")
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await?;
        ensure!(exists == 1, "media owner does not exist");
        let existing_asset: Option<String> =
            sqlx::query_scalar("SELECT id FROM media_assets WHERE hash=?")
                .bind(&prepared.hash)
                .fetch_optional(&self.pool)
                .await?;
        if existing_asset.is_none() {
            let stored: i64 =
                sqlx::query_scalar("SELECT COALESCE(SUM(size_bytes),0) FROM media_assets")
                    .fetch_one(&self.pool)
                    .await?;
            let stored = u64::try_from(stored).context("invalid managed storage accounting")?;
            ensure!(
                stored
                    .checked_add(prepared.size)
                    .is_some_and(|total| total <= self.max_storage_bytes),
                "managed media storage quota exceeded"
            );
        }

        let hash = prepared.hash.clone();
        let size = i64::try_from(prepared.size).context("audio size is too large")?;
        let content_type = prepared.content_type;
        let data_dir = self.data_dir.clone();
        let (metadata, artwork_hash, _permit) = tokio::task::spawn_blocking(move || {
            publish(&data_dir, prepared)
                .map(|(metadata, artwork_hash)| (metadata, artwork_hash, _permit))
        })
        .await
        .context("audio publication worker failed")??;
        let mut transaction = self.pool.begin().await?;
        let candidate_asset = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,?,?) ON CONFLICT(hash) DO NOTHING")
            .bind(&candidate_asset).bind(&hash).bind(format!("media/{hash}.audio"))
            .bind(content_type).bind(size).execute(&mut *transaction).await?;
        let asset_id: String = sqlx::query_scalar("SELECT id FROM media_assets WHERE hash=?")
            .bind(&hash)
            .fetch_one(&mut *transaction)
            .await?;
        let artwork_id = if let Some(artwork_hash) = artwork_hash {
            let candidate_artwork = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO artwork(id,hash,storage_path) VALUES(?,?,?) ON CONFLICT(hash) DO NOTHING")
                .bind(candidate_artwork).bind(&artwork_hash).bind(format!("artwork/{artwork_hash}.jpg"))
                .execute(&mut *transaction).await?;
            Some(
                sqlx::query_scalar::<_, String>("SELECT id FROM artwork WHERE hash=?")
                    .bind(artwork_hash)
                    .fetch_one(&mut *transaction)
                    .await?,
            )
        } else {
            None
        };
        let candidate_track = Uuid::new_v4().to_string();
        let inserted = sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,album_artist,genre,year,track_number,disc_number,duration_ms,codec,bitrate,sample_rate,bit_depth,channels,artwork_id) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(owner_id,media_asset_id) DO NOTHING")
            .bind(candidate_track).bind(owner_id).bind(&asset_id)
            .bind(metadata.title).bind(metadata.artist).bind(metadata.album).bind(metadata.album_artist).bind(metadata.genre)
            .bind(metadata.year).bind(metadata.track_number).bind(metadata.disc_number)
            .bind(metadata.duration_ms).bind(metadata.codec).bind(metadata.bitrate).bind(metadata.sample_rate)
            .bind(metadata.bit_depth).bind(metadata.channels).bind(artwork_id)
            .execute(&mut *transaction).await?.rows_affected();
        let track_id: String =
            sqlx::query_scalar("SELECT id FROM tracks WHERE owner_id=? AND media_asset_id=?")
                .bind(owner_id)
                .bind(asset_id)
                .fetch_one(&mut *transaction)
                .await?;
        transaction.commit().await?;
        let track = self
            .get_track(owner_id, &track_id)
            .await?
            .context("published track is unavailable")?;
        Ok(UploadResult {
            track,
            duplicate: inserted == 0,
        })
    }

    pub async fn get_track(&self, owner_id: &str, id: &str) -> Result<Option<Track>> {
        let mut builder = QueryBuilder::<Sqlite>::new(TRACK_SELECT);
        builder
            .push_bind(owner_id)
            .push(" WHERE t.owner_id=")
            .push_bind(owner_id)
            .push(" AND t.id=")
            .push_bind(id);
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(track_from_row)
            .transpose()
    }

    pub async fn asset(&self, owner_id: &str, id: &str) -> Result<Option<MediaAsset>> {
        let row = sqlx::query("SELECT a.hash,a.content_type,a.size_bytes FROM tracks t JOIN media_assets a ON a.id=t.media_asset_id WHERE t.owner_id=? AND t.id=?")
            .bind(owner_id).bind(id).fetch_optional(&self.pool).await?;
        row.map(|row| {
            let hash: String = row.try_get("hash")?;
            ensure!(valid_hash(&hash), "invalid managed media identifier");
            Ok(MediaAsset {
                path: self.data_dir.join("media").join(format!("{hash}.audio")),
                content_type: row.try_get("content_type")?,
                size_bytes: u64::try_from(row.try_get::<i64, _>("size_bytes")?)
                    .context("invalid media size")?,
                hash,
            })
        })
        .transpose()
    }

    pub async fn artwork(&self, owner_id: &str, id: &str) -> Result<Option<PathBuf>> {
        let hash: Option<String> = sqlx::query_scalar("SELECT a.hash FROM artwork a WHERE a.id=? AND EXISTS(SELECT 1 FROM tracks t WHERE t.artwork_id=a.id AND t.owner_id=?)")
            .bind(id).bind(owner_id).fetch_optional(&self.pool).await?;
        hash.map(|hash| {
            ensure!(valid_hash(&hash), "invalid managed artwork identifier");
            Ok(self.data_dir.join("artwork").join(format!("{hash}.jpg")))
        })
        .transpose()
    }

    pub async fn set_favorite(&self, owner_id: &str, id: &str, favorite: bool) -> Result<bool> {
        if self.get_track(owner_id, id).await?.is_none() {
            return Ok(false);
        }
        if favorite {
            sqlx::query(
                "INSERT INTO favorites(user_id,track_id) VALUES(?,?) ON CONFLICT DO NOTHING",
            )
            .bind(owner_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("DELETE FROM favorites WHERE user_id=? AND track_id=?")
                .bind(owner_id)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(true)
    }

    pub async fn stats(&self, owner_id: &str) -> Result<LibraryStats> {
        // AUD-09: three independent aggregates; covering indexes make the owner
        // scan and the media-asset size lookup index-only, and the groups run in
        // parallel on separate pool connections instead of one serial statement.
        let base=sqlx::query("SELECT COUNT(*) AS tracks,COALESCE(SUM(a.size_bytes),0) AS size_bytes,COALESCE(SUM(t.duration_ms),0) AS duration_ms FROM tracks t INDEXED BY tracks_owner_stats JOIN media_assets a INDEXED BY media_assets_cover ON a.id=t.media_asset_id WHERE t.owner_id=?").bind(owner_id).fetch_one(&self.pool);
        let albums=sqlx::query("SELECT COUNT(*) AS albums FROM albums b WHERE b.owner_id=? AND EXISTS(SELECT 1 FROM tracks bt INDEXED BY tracks_catalog_album WHERE bt.owner_id=b.owner_id AND bt.album_id=b.id)").bind(owner_id).fetch_one(&self.pool);
        let artists=sqlx::query("SELECT COUNT(*) AS artists FROM artists ar WHERE ar.owner_id=? AND (EXISTS(SELECT 1 FROM tracks at INDEXED BY tracks_catalog_artist WHERE at.owner_id=ar.owner_id AND at.artist_id=ar.id) OR EXISTS(SELECT 1 FROM albums ab CROSS JOIN tracks at ON at.album_id=ab.id AND at.owner_id=ab.owner_id WHERE ab.owner_id=ar.owner_id AND ab.artist_id=ar.id))").bind(owner_id).fetch_one(&self.pool);
        let (base, albums, artists) = tokio::try_join!(base, albums, artists)?;
        Ok(LibraryStats {
            tracks: base.try_get("tracks")?,
            albums: albums.try_get("albums")?,
            artists: artists.try_get("artists")?,
            size_bytes: base.try_get("size_bytes")?,
            duration_ms: base.try_get("duration_ms")?,
        })
    }
}

fn track_from_row(row: &SqliteRow) -> Result<Track> {
    Ok(Track {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        artist: row.try_get("artist")?,
        album: row.try_get("album")?,
        album_artist: row.try_get("album_artist")?,
        genre: row.try_get("genre")?,
        year: row.try_get("year")?,
        track_number: row.try_get("track_number")?,
        disc_number: row.try_get("disc_number")?,
        duration_ms: row.try_get("duration_ms")?,
        codec: row.try_get("codec")?,
        bitrate: row.try_get("bitrate")?,
        sample_rate: row.try_get("sample_rate")?,
        bit_depth: row.try_get("bit_depth")?,
        channels: row.try_get("channels")?,
        size_bytes: row.try_get("size_bytes")?,
        artwork_id: row.try_get("artwork_id")?,
        added_at: row.try_get("added_at")?,
        favorite: row.try_get("favorite")?,
    })
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn publish(
    data_dir: &Path,
    prepared: validation::PreparedMedia,
) -> Result<(validation::Metadata, Option<String>)> {
    let audio_path = data_dir
        .join("media")
        .join(format!("{}.audio", prepared.hash));
    persist_content(prepared.file, &audio_path, &prepared.hash, prepared.size)?;
    let artwork_hash = prepared
        .artwork
        .map(|bytes| {
            let hash = blake3::hash(&bytes).to_hex().to_string();
            let directory = data_dir.join("artwork");
            let mut file = tempfile::Builder::new()
                .prefix(".vela-pending-")
                .tempfile_in(&directory)?;
            file.write_all(&bytes)?;
            file.as_file().sync_all()?;
            persist_content(
                file,
                &directory.join(format!("{hash}.jpg")),
                &hash,
                bytes.len() as u64,
            )?;
            Ok::<_, anyhow::Error>(hash)
        })
        .transpose()?;
    Ok((prepared.metadata, artwork_hash))
}

fn persist_content(
    file: tempfile::NamedTempFile,
    target: &Path,
    hash: &str,
    size: u64,
) -> Result<()> {
    match file.persist_noclobber(target) {
        Ok(file) => {
            file.sync_all().context("could not flush published media")?;
        }
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            // Content-addressed races are harmless only when the existing bytes
            // really match. Never replace a corrupted or unexpected object.
            let metadata = std::fs::symlink_metadata(target)?;
            ensure!(
                metadata.is_file() && metadata.len() == size,
                "managed object integrity check failed"
            );
            let mut existing = File::open(target)?;
            let mut hasher = blake3::Hasher::new();
            let mut buffer = [0_u8; 64 * 1024];
            let mut bytes_read = 0_u64;
            loop {
                let read = existing.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                bytes_read = bytes_read
                    .checked_add(read as u64)
                    .context("managed object size overflow")?;
                ensure!(
                    bytes_read <= size,
                    "managed object changed during integrity check"
                );
                hasher.update(&buffer[..read]);
            }
            ensure!(
                hasher.finalize().to_hex().as_str() == hash,
                "managed object integrity check failed"
            );
        }
        Err(_) => anyhow::bail!("could not publish managed object"),
    }
    #[cfg(unix)]
    File::open(
        target
            .parent()
            .context("invalid managed object directory")?,
    )?
    .sync_all()
    .context("could not flush storage directory")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_identifiers_are_fixed_hex_and_cannot_traverse() {
        assert!(valid_hash(&"a".repeat(64)));
        assert!(!valid_hash(&"../".repeat(22)));
        assert!(!valid_hash(&"A".repeat(64)));
    }
}
pub mod analysis;
pub mod analysis_service;
mod discovery;
pub use discovery::DiscoveryError;
mod recommendations;
pub use recommendations::RecommendationError;
