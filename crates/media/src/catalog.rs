//! Bounded owner-scoped browsing of persisted, provisional catalog identities.
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite, sqlite::SqliteRow};
use uuid::Uuid;
use vela_domain::{AlbumPage, AlbumSummary, ArtistPage, ArtistSummary, TrackPage};

use crate::{MediaService, TRACK_SELECT, track_from_row};

/// Invalid, stale or unrelated cursor; its message never reveals a private ID.
#[derive(Debug, thiserror::Error)]
#[error("invalid catalog cursor")]
pub struct InvalidCatalogCursor;

pub(super) const ALBUM_SELECT: &str = "SELECT b.id,b.title,ar.name AS artist,b.artist_id,NULLIF(b.release_year,0) AS year,(SELECT COUNT(*) FROM tracks t WHERE t.owner_id=b.owner_id AND t.album_id=b.id) AS track_count,(SELECT COALESCE(SUM(t.duration_ms),0) FROM tracks t WHERE t.owner_id=b.owner_id AND t.album_id=b.id) AS duration_ms,(SELECT t.artwork_id FROM tracks t WHERE t.owner_id=b.owner_id AND t.album_id=b.id AND t.artwork_id IS NOT NULL ORDER BY COALESCE(t.disc_number,1),COALESCE(t.track_number,2147483647),t.id LIMIT 1) AS artwork_id FROM albums b JOIN artists ar ON ar.id=b.artist_id AND ar.owner_id=b.owner_id";
pub(super) const ALBUM_VISIBLE: &str =
    "EXISTS(SELECT 1 FROM tracks t WHERE t.owner_id=b.owner_id AND t.album_id=b.id)";
pub(super) const ARTIST_VISIBLE: &str = "(EXISTS(SELECT 1 FROM tracks t WHERE t.owner_id=ar.owner_id AND t.artist_id=ar.id) OR EXISTS(SELECT 1 FROM albums b JOIN tracks t ON t.album_id=b.id AND t.owner_id=b.owner_id WHERE b.owner_id=ar.owner_id AND b.artist_id=ar.id))";
// UNION keeps a track credited both directly and as album artist only once.
// Each arm can start from its indexed owner/artist association.
const ARTIST_TRACKS: &str = "SELECT t.id,t.album_id,t.artwork_id FROM tracks t WHERE t.owner_id=ar.owner_id AND t.artist_id=ar.id UNION SELECT t.id,t.album_id,t.artwork_id FROM albums b JOIN tracks t ON t.album_id=b.id AND t.owner_id=b.owner_id WHERE b.owner_id=ar.owner_id AND b.artist_id=ar.id";

impl MediaService {
    pub async fn list_albums(
        &self,
        owner_id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<AlbumPage> {
        let cursor = cursor.map(validate_cursor).transpose()?;
        let limit = limit.clamp(1, 100) as usize;
        let mut builder = QueryBuilder::<Sqlite>::new(ALBUM_SELECT);
        builder
            .push(" WHERE b.owner_id=")
            .push_bind(owner_id)
            .push(" AND ")
            .push(ALBUM_VISIBLE);
        if let Some(cursor) = cursor {
            builder.push(" AND b.id>").push_bind(cursor);
        }
        builder
            .push(" ORDER BY b.id LIMIT ")
            .push_bind((limit + 1) as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = rows
            .iter()
            .map(album_from_row)
            .collect::<Result<Vec<_>>>()?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = has_more.then(|| items.last().unwrap().id.clone());
        Ok(AlbumPage { items, next_cursor })
    }

    pub async fn album(&self, owner_id: &str, id: &str) -> Result<Option<AlbumSummary>> {
        let mut builder = QueryBuilder::<Sqlite>::new(ALBUM_SELECT);
        builder
            .push(" WHERE b.owner_id=")
            .push_bind(owner_id)
            .push(" AND b.id=")
            .push_bind(id)
            .push(" AND ")
            .push(ALBUM_VISIBLE);
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(album_from_row)
            .transpose()
    }

    pub async fn album_tracks(
        &self,
        owner_id: &str,
        id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Option<TrackPage>> {
        let limit = limit.clamp(1, 100) as usize;
        // Keep authorization, cursor lookup and page selection in one read
        // snapshot if an album changes while a request is being handled.
        let mut transaction = self.pool.begin().await?;
        let mut visible = QueryBuilder::<Sqlite>::new("SELECT 1 FROM albums b WHERE b.owner_id=");
        visible
            .push_bind(owner_id)
            .push(" AND b.id=")
            .push_bind(id)
            .push(" AND ")
            .push(ALBUM_VISIBLE);
        if visible
            .build()
            .fetch_optional(&mut *transaction)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let cursor = cursor.map(validate_cursor).transpose()?;
        let sort_key = if let Some(cursor) = &cursor {
            let row = sqlx::query("SELECT COALESCE(disc_number,1) AS disc,COALESCE(track_number,2147483647) AS number FROM tracks WHERE owner_id=? AND album_id=? AND id=?")
                .bind(owner_id).bind(id).bind(cursor)
                .fetch_optional(&mut *transaction).await?
                .ok_or(InvalidCatalogCursor)?;
            Some((
                row.try_get::<i64, _>("disc")?,
                row.try_get::<i64, _>("number")?,
            ))
        } else {
            None
        };
        let mut builder = QueryBuilder::<Sqlite>::new(TRACK_SELECT);
        builder
            .push_bind(owner_id)
            .push(" WHERE t.owner_id=")
            .push_bind(owner_id)
            .push(" AND t.album_id=")
            .push_bind(id);
        if let Some((disc, number)) = sort_key {
            builder
                .push(" AND (COALESCE(t.disc_number,1),COALESCE(t.track_number,2147483647),t.id)>(")
                .push_bind(disc)
                .push(",")
                .push_bind(number)
                .push(",")
                .push_bind(cursor.as_deref().unwrap())
                .push(")");
        }
        builder
            .push(" ORDER BY COALESCE(t.disc_number,1),COALESCE(t.track_number,2147483647),t.id LIMIT ")
            .push_bind((limit + 1) as i64);
        let rows = builder.build().fetch_all(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(track_page(&rows, limit)?))
    }

    pub async fn list_artists(
        &self,
        owner_id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<ArtistPage> {
        let cursor = cursor.map(validate_cursor).transpose()?;
        let limit = limit.clamp(1, 100) as usize;
        let mut builder = QueryBuilder::<Sqlite>::new(artist_select());
        builder
            .push(" WHERE ar.owner_id=")
            .push_bind(owner_id)
            .push(" AND ")
            .push(ARTIST_VISIBLE);
        if let Some(cursor) = cursor {
            builder.push(" AND ar.id>").push_bind(cursor);
        }
        builder
            .push(" ORDER BY ar.id LIMIT ")
            .push_bind((limit + 1) as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = rows
            .iter()
            .map(artist_from_row)
            .collect::<Result<Vec<_>>>()?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = has_more.then(|| items.last().unwrap().id.clone());
        Ok(ArtistPage { items, next_cursor })
    }

    pub async fn artist(&self, owner_id: &str, id: &str) -> Result<Option<ArtistSummary>> {
        let mut builder = QueryBuilder::<Sqlite>::new(artist_select());
        builder
            .push(" WHERE ar.owner_id=")
            .push_bind(owner_id)
            .push(" AND ar.id=")
            .push_bind(id)
            .push(" AND ")
            .push(ARTIST_VISIBLE);
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(artist_from_row)
            .transpose()
    }

    pub async fn artist_tracks(
        &self,
        owner_id: &str,
        id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Option<TrackPage>> {
        let limit = limit.clamp(1, 100) as usize;
        let mut transaction = self.pool.begin().await?;
        let mut visible =
            QueryBuilder::<Sqlite>::new("SELECT 1 FROM artists ar WHERE ar.owner_id=");
        visible
            .push_bind(owner_id)
            .push(" AND ar.id=")
            .push_bind(id)
            .push(" AND ")
            .push(ARTIST_VISIBLE);
        if visible
            .build()
            .fetch_optional(&mut *transaction)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let cursor = cursor.map(validate_cursor).transpose()?;
        // Reuse the same association predicate for membership and page reads.
        let mut builder = artist_track_query(owner_id, id);
        if let Some(cursor) = &cursor {
            builder.push(" AND t.id=").push_bind(cursor);
            if builder
                .build()
                .fetch_optional(&mut *transaction)
                .await?
                .is_none()
            {
                return Err(InvalidCatalogCursor.into());
            }
        }
        let mut builder = artist_track_query(owner_id, id);
        if let Some(cursor) = cursor {
            builder.push(" AND t.id>").push_bind(cursor);
        }
        builder
            .push(" ORDER BY t.id LIMIT ")
            .push_bind((limit + 1) as i64);
        let rows = builder.build().fetch_all(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(track_page(&rows, limit)?))
    }
}

fn validate_cursor(cursor: &str) -> Result<String> {
    Uuid::parse_str(cursor)
        .map(|uuid| uuid.to_string())
        .map_err(|_| InvalidCatalogCursor.into())
}

pub(super) fn artist_select() -> String {
    format!(
        "SELECT ar.id,ar.name,(SELECT COUNT(*) FROM ({ARTIST_TRACKS})) AS track_count,(SELECT COUNT(DISTINCT album_id) FROM ({ARTIST_TRACKS})) AS album_count,(SELECT artwork_id FROM ({ARTIST_TRACKS}) WHERE artwork_id IS NOT NULL ORDER BY id LIMIT 1) AS artwork_id FROM artists ar"
    )
}

fn artist_track_query<'a>(owner_id: &'a str, id: &'a str) -> QueryBuilder<'a, Sqlite> {
    let mut builder = QueryBuilder::<Sqlite>::new(TRACK_SELECT);
    builder
        .push_bind(owner_id)
        .push(" WHERE t.owner_id=")
        .push_bind(owner_id)
        .push(" AND t.id IN (SELECT related.id FROM tracks related WHERE related.owner_id=")
        .push_bind(owner_id)
        .push(" AND related.artist_id=")
        .push_bind(id)
        .push(" UNION SELECT related.id FROM albums b JOIN tracks related ON related.album_id=b.id AND related.owner_id=b.owner_id WHERE b.owner_id=")
        .push_bind(owner_id)
        .push(" AND b.artist_id=")
        .push_bind(id)
        .push(")");
    builder
}

fn track_page(rows: &[SqliteRow], limit: usize) -> Result<TrackPage> {
    let mut items = rows
        .iter()
        .map(track_from_row)
        .collect::<Result<Vec<_>>>()?;
    let has_more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = has_more.then(|| items.last().unwrap().id.clone());
    Ok(TrackPage { items, next_cursor })
}

pub(super) fn album_from_row(row: &SqliteRow) -> Result<AlbumSummary> {
    Ok(AlbumSummary {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        artist: row.try_get("artist")?,
        artist_id: row.try_get("artist_id")?,
        year: row.try_get("year")?,
        track_count: row.try_get("track_count")?,
        duration_ms: row.try_get("duration_ms")?,
        artwork_id: row.try_get("artwork_id")?,
    })
}

pub(super) fn artist_from_row(row: &SqliteRow) -> Result<ArtistSummary> {
    Ok(ArtistSummary {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        track_count: row.try_get("track_count")?,
        album_count: row.try_get("album_count")?,
        artwork_id: row.try_get("artwork_id")?,
    })
}
