//! Bounded local search. Group names are literal substrings; tracks use FTS.
use crate::{MediaService, catalog::*};
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite};
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
        let tracks_fut = self.list_tracks(owner, Some(query), None, 8, false);
        let albums_fut = async {
            let mut album_ids = QueryBuilder::<Sqlite>::new(
                "SELECT b.id FROM albums b JOIN artists ar ON ar.id=b.artist_id AND ar.owner_id=b.owner_id WHERE b.owner_id=",
            );
            album_ids
                .push_bind(owner)
                .push(" AND ")
                .push(ALBUM_VISIBLE)
                .push(" AND (instr(lower(b.title),lower(")
                .push_bind(query)
                .push("))>0 OR instr(lower(ar.name),lower(")
                .push_bind(query)
                .push("))>0) ORDER BY b.title COLLATE NOCASE,b.id LIMIT 8");
            let album_ids = album_ids.build().fetch_all(&self.pool).await?;
            let album_ids = album_ids
                .iter()
                .map(|row| row.get::<String, _>("id"))
                .collect::<Vec<_>>();
            if album_ids.is_empty() {
                return Ok(Vec::new());
            }
            let mut albums = QueryBuilder::<Sqlite>::new(ALBUM_SELECT);
            albums
                .push(" WHERE b.owner_id=")
                .push_bind(owner)
                .push(" AND b.id IN (");
            let mut ids = albums.separated(",");
            for id in &album_ids {
                ids.push_bind(id);
            }
            ids.push_unseparated(") ORDER BY b.title COLLATE NOCASE,b.id");
            albums
                .build()
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(album_from_row)
                .collect::<Result<Vec<_>>>()
        };
        let artists_fut = async {
            let mut artist_ids =
                QueryBuilder::<Sqlite>::new("SELECT ar.id FROM artists ar WHERE ar.owner_id=");
            artist_ids
                .push_bind(owner)
                .push(" AND ")
                .push(ARTIST_VISIBLE)
                .push(" AND instr(lower(ar.name),lower(")
                .push_bind(query)
                .push("))>0 ORDER BY ar.name COLLATE NOCASE,ar.id LIMIT 8");
            let artist_ids = artist_ids.build().fetch_all(&self.pool).await?;
            let artist_ids = artist_ids
                .iter()
                .map(|row| row.get::<String, _>("id"))
                .collect::<Vec<_>>();
            if artist_ids.is_empty() {
                return Ok(Vec::new());
            }
            let mut artists = QueryBuilder::<Sqlite>::new(artist_select());
            artists
                .push(" WHERE ar.owner_id=")
                .push_bind(owner)
                .push(" AND ar.id IN (");
            let mut ids = artists.separated(",");
            for id in &artist_ids {
                ids.push_bind(id);
            }
            ids.push_unseparated(") ORDER BY ar.name COLLATE NOCASE,ar.id");
            artists
                .build()
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(artist_from_row)
                .collect::<Result<Vec<_>>>()
        };
        let (tracks, albums, artists) = tokio::try_join!(tracks_fut, albums_fut, artists_fut)?;
        let tracks = tracks.items;
        Ok(CatalogMatches {
            tracks,
            albums,
            artists,
        })
    }
}
