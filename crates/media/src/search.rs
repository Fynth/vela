//! Bounded local search. Group names are literal substrings; tracks use FTS.
use crate::{MediaService, catalog::*};
use anyhow::Result;
use sqlx::{QueryBuilder, Sqlite};
use vela_domain::{AlbumSummary, ArtistSummary, Track};

#[derive(Debug, thiserror::Error)]
#[error("Search must contain 1–256 bytes without control characters")]
pub struct InvalidSearchQuery;

pub struct CatalogMatches {
    pub tracks: Vec<Track>,
    pub albums: Vec<AlbumSummary>,
    pub artists: Vec<ArtistSummary>,
}

impl MediaService {
    pub async fn search_catalog(&self, owner: &str, query: &str) -> Result<CatalogMatches> {
        if query.len() > 256 || query.trim().is_empty() || query.chars().any(char::is_control) {
            return Err(InvalidSearchQuery.into());
        }
        let query = query.trim();
        let tracks = self
            .list_tracks(owner, Some(query), None, 8, false)
            .await?
            .items;
        let mut albums = QueryBuilder::<Sqlite>::new(ALBUM_SELECT);
        albums
            .push(" WHERE b.owner_id=")
            .push_bind(owner)
            .push(" AND ")
            .push(ALBUM_VISIBLE)
            .push(" AND (instr(lower(b.title),lower(")
            .push_bind(query)
            .push("))>0 OR instr(lower(ar.name),lower(")
            .push_bind(query)
            .push("))>0) ORDER BY b.title COLLATE NOCASE,b.id LIMIT 8");
        let albums = albums
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(album_from_row)
            .collect::<Result<Vec<_>>>()?;
        let mut artists = QueryBuilder::<Sqlite>::new(artist_select());
        artists
            .push(" WHERE ar.owner_id=")
            .push_bind(owner)
            .push(" AND ")
            .push(ARTIST_VISIBLE)
            .push(" AND instr(lower(ar.name),lower(")
            .push_bind(query)
            .push("))>0 ORDER BY ar.name COLLATE NOCASE,ar.id LIMIT 8");
        let artists = artists
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(artist_from_row)
            .collect::<Result<Vec<_>>>()?;
        Ok(CatalogMatches {
            tracks,
            albums,
            artists,
        })
    }
}
