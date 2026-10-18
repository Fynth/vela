//! Explicit uploader declarations and separately authorized public media.
//! Enabling a server never creates a publication or changes a private ACL.
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite, SqliteConnection, sqlite::SqliteRow};
use url::Url;
use uuid::Uuid;
use vela_domain::{
    CreatePublication, PublicCatalog, PublicLicense, PublicTrack, Publication, PublicationPage,
    PublicationQuery, canonical_public_origin,
};

use crate::{MediaAsset, MediaService, track_from_row, valid_hash};

const OWNER_QUOTA: i64 = 1000;
const GLOBAL_QUOTA: i64 = 10000;
const MAX_PUBLIC_DURATION_MS: i64 = 604800000;
const OWNER_SELECT: &str = "SELECT p.id AS publication_id,p.license,p.attribution,p.rights_evidence,p.allow_stream,p.allow_download,p.revision AS publication_revision,p.created_at,p.revoked_at,t.id,t.title,t.artist,t.album,t.album_artist,t.genre,t.year,t.track_number,t.disc_number,t.duration_ms,t.codec,t.bitrate,t.sample_rate,t.bit_depth,t.channels,a.size_bytes,t.artwork_id,t.added_at,(f.track_id IS NOT NULL) AS favorite FROM publications p JOIN tracks t ON t.id=p.track_id AND t.owner_id=p.user_id JOIN media_assets a ON a.id=t.media_asset_id LEFT JOIN favorites f ON f.track_id=t.id AND f.user_id=p.user_id";
// Bound raw byte extraction before decoding: SQLite TEXT substr stops at NUL
// and would hide the suffix before control filtering. The four-byte prefix
// budget also accommodates UTF-8 before the final byte bound. No private fields
// are selected into the public DTO's construction path.
const PUBLIC_SELECT: &str = "SELECT p.id,p.revision,substr(CAST(t.title AS BLOB),1,2048) AS title,substr(CAST(t.artist AS BLOB),1,2048) AS artist,substr(CAST(t.album AS BLOB),1,2048) AS album,substr(CAST(t.genre AS BLOB),1,1024) AS genre,t.year,t.duration_ms,substr(CAST(t.codec AS BLOB),1,256) AS codec,p.license,p.attribution,p.allow_stream,p.allow_download FROM publications p JOIN tracks t ON t.id=p.track_id AND t.owner_id=p.user_id JOIN media_assets a ON a.id=t.media_asset_id";
const PUBLIC_ELIGIBLE: &str = "p.revoked_at IS NULL AND t.duration_ms>0 AND t.duration_ms<=604800000 AND a.size_bytes>0 AND length(a.hash)=64 AND a.hash NOT GLOB '*[^0-9a-f]*'";

/// Fixed safe policy errors. Worker/database diagnostics never become DTOs.
#[derive(Debug, thiserror::Error)]
pub enum PublicationError {
    #[error("Invalid publication request")]
    Invalid,
    #[error("Publication not found")]
    NotFound,
    #[error("Publication state changed or publication limit reached")]
    Conflict,
}

struct PageInput<'a> {
    cursor: Option<&'a str>,
    limit: usize,
    q: Option<&'a str>,
}

impl MediaService {
    pub async fn create_publication(
        &self,
        owner: &str,
        input: CreatePublication,
    ) -> Result<Publication> {
        let attribution = input.attribution.trim();
        let evidence = input.rights_evidence.trim();
        if !canonical_uuid(&input.track_id)
            || !input.rights_confirmed
            || (input.allow_download && !input.allow_stream)
            || attribution.is_empty()
            || attribution.len() > 2000
            || attribution.chars().any(|c| c.is_control() && c != '\n')
            || input.rights_evidence.len() > 2048
            || input.rights_evidence.chars().any(char::is_control)
            || !valid_evidence(evidence)
        {
            return Err(PublicationError::Invalid.into());
        }
        // Serialize ownership, enablement, both durable record quotas and the
        // active-record constraint with every other publication mutation.
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if public_settings(&mut tx).await?.is_none() {
            return Err(PublicationError::Conflict.into());
        }
        let asset_hash: Option<String> = sqlx::query_scalar("SELECT a.hash FROM tracks t JOIN media_assets a ON a.id=t.media_asset_id WHERE t.owner_id=? AND t.id=? AND t.duration_ms>0 AND t.duration_ms<=? AND a.size_bytes>0")
            .bind(owner).bind(&input.track_id).bind(MAX_PUBLIC_DURATION_MS).fetch_optional(&mut *tx).await?;
        if !asset_hash.as_deref().is_some_and(valid_hash) {
            return Err(PublicationError::NotFound.into());
        }
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM publications WHERE user_id=? AND track_id=? AND revoked_at IS NULL)")
            .bind(owner).bind(&input.track_id).fetch_one(&mut *tx).await?;
        let counts = sqlx::query(
            "SELECT COUNT(*) AS total,COALESCE(SUM(user_id=?),0) AS owned FROM publications",
        )
        .bind(owner)
        .fetch_one(&mut *tx)
        .await?;
        if active
            || counts.try_get::<i64, _>("owned")? >= OWNER_QUOTA
            || counts.try_get::<i64, _>("total")? >= GLOBAL_QUOTA
        {
            return Err(PublicationError::Conflict.into());
        }
        // Neither the track nor physical asset identifier is a public identity.
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO publications(id,user_id,track_id,license,attribution,rights_evidence,allow_stream,allow_download,revision,created_at) VALUES(?,?,?,?,?,?,?,?,1,unixepoch())")
            .bind(&id).bind(owner).bind(&input.track_id).bind(input.license.id())
            .bind(attribution).bind(evidence).bind(input.allow_stream).bind(input.allow_download)
            .execute(&mut *tx).await?;
        let publication = owned_publication(&mut tx, owner, &id).await?;
        tx.commit().await?;
        Ok(publication)
    }

    pub async fn publications(
        &self,
        owner: &str,
        query: &PublicationQuery,
    ) -> Result<PublicationPage> {
        let input = page_input(query, 50, 100)?;
        let mut tx = self.pool.begin().await?;
        let mut select = QueryBuilder::<Sqlite>::new(OWNER_SELECT);
        select.push(" WHERE p.user_id=").push_bind(owner);
        page_filter(&mut select, &input);
        let rows = select.build().fetch_all(&mut *tx).await?;
        let mut items = rows
            .iter()
            .map(publication_from_row)
            .collect::<Result<Vec<_>>>()?;
        let has_more = items.len() > input.limit;
        items.truncate(input.limit);
        let next_cursor = has_more.then(|| items.last().unwrap().id.clone());
        tx.commit().await?;
        Ok(PublicationPage { items, next_cursor })
    }

    pub async fn revoke_publication(
        &self,
        owner: &str,
        id: &str,
        revision: i64,
    ) -> Result<Publication> {
        if !canonical_uuid(id) || revision < 1 {
            return Err(PublicationError::Invalid.into());
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current = owned_publication(&mut tx, owner, id).await?;
        if current.revision != revision {
            return Err(PublicationError::Conflict.into());
        }
        if current.revoked_at.is_some() {
            tx.commit().await?;
            return Ok(current);
        }
        if revision == i64::MAX {
            return Err(PublicationError::Conflict.into());
        }
        let changed = sqlx::query("UPDATE publications SET revoked_at=unixepoch(),revision=revision+1 WHERE id=? AND user_id=? AND revision=? AND revoked_at IS NULL")
            .bind(id).bind(owner).bind(revision).execute(&mut *tx).await?.rows_affected();
        if changed != 1 {
            return Err(PublicationError::Conflict.into());
        }
        let publication = owned_publication(&mut tx, owner, id).await?;
        tx.commit().await?;
        Ok(publication)
    }

    pub async fn public_catalog(&self, query: &PublicationQuery) -> Result<PublicCatalog> {
        let mut tx = self.pool.begin().await?;
        let (server_id, origin) = public_settings(&mut tx)
            .await?
            .ok_or(PublicationError::NotFound)?;
        let input = page_input(query, 20, 50)?;
        let mut select = QueryBuilder::<Sqlite>::new(PUBLIC_SELECT);
        select.push(" WHERE ").push(PUBLIC_ELIGIBLE);
        page_filter(&mut select, &input);
        let rows = select.build().fetch_all(&mut *tx).await?;
        let mut items = rows
            .iter()
            .map(public_track_from_row)
            .collect::<Result<Vec<_>>>()?;
        let has_more = items.len() > input.limit;
        items.truncate(input.limit);
        let next_cursor = has_more.then(|| items.last().unwrap().id.clone());
        tx.commit().await?;
        Ok(PublicCatalog {
            server_id,
            origin,
            items,
            next_cursor,
        })
    }

    pub async fn public_track(&self, id: &str) -> Result<PublicTrack> {
        let mut tx = self.pool.begin().await?;
        if public_settings(&mut tx).await?.is_none() || !canonical_uuid(id) {
            return Err(PublicationError::NotFound.into());
        }
        let mut select = QueryBuilder::<Sqlite>::new(PUBLIC_SELECT);
        select
            .push(" WHERE ")
            .push(PUBLIC_ELIGIBLE)
            .push(" AND p.id=")
            .push_bind(id);
        let row = select
            .build()
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(PublicationError::NotFound)?;
        let track = public_track_from_row(&row)?;
        tx.commit().await?;
        Ok(track)
    }

    /// Internal descriptor only. Every new stream/download request must invoke
    /// this policy check, including HEAD and range requests; metadata is no grant.
    pub async fn public_asset(&self, id: &str, download: bool) -> Result<MediaAsset> {
        let mut tx = self.pool.begin().await?;
        if public_settings(&mut tx).await?.is_none() || !canonical_uuid(id) {
            return Err(PublicationError::NotFound.into());
        }
        let mut select = QueryBuilder::<Sqlite>::new(
            "SELECT a.hash,a.content_type,a.size_bytes FROM publications p JOIN tracks t ON t.id=p.track_id AND t.owner_id=p.user_id JOIN media_assets a ON a.id=t.media_asset_id WHERE ",
        );
        select
            .push(PUBLIC_ELIGIBLE)
            .push(" AND p.id=")
            .push_bind(id)
            .push(" AND p.allow_stream=1");
        if download {
            select.push(" AND p.allow_download=1");
        }
        let row = select
            .build()
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(PublicationError::NotFound)?;
        let hash: String = row.try_get("hash")?;
        let size = row.try_get::<i64, _>("size_bytes")?;
        if !valid_hash(&hash) || size <= 0 {
            return Err(PublicationError::NotFound.into());
        }
        let asset = MediaAsset {
            path: self.data_dir.join("media").join(format!("{hash}.audio")),
            content_type: row.try_get("content_type")?,
            size_bytes: size as u64,
            hash,
        };
        tx.commit().await?;
        Ok(asset)
    }
}

async fn owned_publication(
    connection: &mut SqliteConnection,
    owner: &str,
    id: &str,
) -> Result<Publication> {
    let mut select = QueryBuilder::<Sqlite>::new(OWNER_SELECT);
    select
        .push(" WHERE p.user_id=")
        .push_bind(owner)
        .push(" AND p.id=")
        .push_bind(id);
    let row = select
        .build()
        .fetch_optional(connection)
        .await?
        .ok_or(PublicationError::NotFound)?;
    publication_from_row(&row)
}

async fn public_settings(connection: &mut SqliteConnection) -> Result<Option<(String, String)>> {
    let row = sqlx::query("SELECT server_id,origin FROM network_settings WHERE id=1 AND enabled=1")
        .fetch_optional(connection)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let server_id: String = row.try_get("server_id")?;
    let origin: Option<String> = row.try_get("origin")?;
    Ok(origin
        .filter(|origin| {
            canonical_public_origin(origin).is_ok_and(|canonical| canonical == *origin)
                && canonical_uuid(&server_id)
        })
        .map(|origin| (server_id, origin)))
}

fn page_input(query: &PublicationQuery, default: u32, maximum: u32) -> Result<PageInput<'_>> {
    let limit = query.limit.unwrap_or(default);
    let q = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    if !(1..=maximum).contains(&limit)
        || query
            .cursor
            .as_deref()
            .is_some_and(|id| !canonical_uuid(id))
        || q.is_some_and(|q| q.len() > 128 || q.chars().any(char::is_control))
    {
        return Err(PublicationError::Invalid.into());
    }
    Ok(PageInput {
        cursor: query.cursor.as_deref(),
        limit: limit as usize,
        q,
    })
}

fn page_filter<'a>(select: &mut QueryBuilder<'a, Sqlite>, input: &PageInput<'a>) {
    if let Some(cursor) = input.cursor {
        select.push(" AND p.id>").push_bind(cursor);
    }
    if let Some(q) = input.q {
        // instr is literal: %, _, quotes and SQL syntax are ordinary text.
        select
            .push(" AND (instr(lower(t.title),lower(")
            .push_bind(q)
            .push("))>0 OR instr(lower(t.artist),lower(")
            .push_bind(q)
            .push("))>0 OR instr(lower(t.album),lower(")
            .push_bind(q)
            .push("))>0)");
    }
    select
        .push(" ORDER BY p.id ASC LIMIT ")
        .push_bind((input.limit + 1) as i64);
}

fn publication_from_row(row: &SqliteRow) -> Result<Publication> {
    Ok(Publication {
        id: row.try_get("publication_id")?,
        track: track_from_row(row)?,
        license: license(row.try_get("license")?)?,
        attribution: row.try_get("attribution")?,
        rights_evidence: row.try_get("rights_evidence")?,
        allow_stream: row.try_get("allow_stream")?,
        allow_download: row.try_get("allow_download")?,
        revision: row.try_get("publication_revision")?,
        created_at: row.try_get("created_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

fn public_track_from_row(row: &SqliteRow) -> Result<PublicTrack> {
    let title = public_text(row.try_get("title")?, 512);
    Ok(PublicTrack {
        id: row.try_get("id")?,
        revision: row.try_get("revision")?,
        title: if title.trim().is_empty() {
            "Untitled".into()
        } else {
            title
        },
        artist: public_text(row.try_get("artist")?, 512),
        album: public_text(row.try_get("album")?, 512),
        genre: public_text(row.try_get("genre")?, 256),
        year: row
            .try_get::<Option<i64>, _>("year")?
            .filter(|year| (1..=9999).contains(year)),
        duration_ms: row.try_get("duration_ms")?,
        codec: public_text(row.try_get("codec")?, 64),
        license: license(row.try_get("license")?)?,
        attribution: bounded_text(row.try_get("attribution")?, 2000),
        can_stream: row.try_get("allow_stream")?,
        can_download: row.try_get("allow_download")?,
    })
}

fn public_text(bytes: Vec<u8>, maximum: usize) -> String {
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    text.retain(|character| !character.is_control());
    bounded_text(text, maximum)
}

fn license(value: &str) -> Result<PublicLicense> {
    match value {
        "CC0-1.0" => Ok(PublicLicense::Cc0),
        "CC-BY-4.0" => Ok(PublicLicense::Attribution),
        "CC-BY-SA-4.0" => Ok(PublicLicense::ShareAlike),
        _ => Err(PublicationError::Invalid.into()),
    }
}

fn bounded_text(mut text: String, maximum: usize) -> String {
    let mut end = text.len().min(maximum);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}

fn valid_evidence(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 2048
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains('\\')
    {
        return false;
    }
    let Some(authority) = value
        .split_once("://")
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
        .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
    else {
        return false;
    };
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && canonical_public_origin(&url.origin().ascii_serialization()).is_ok()
    })
}
