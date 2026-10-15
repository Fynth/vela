//! Typed, owner-scoped catalog queries. All user values remain SQL parameters.
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite};
use uuid::Uuid;
use vela_domain::{CatalogFacets, TrackFilters, TrackPage, TrackSort};

use crate::{MediaService, TRACK_SELECT, track_from_row};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct InvalidTrackFilter(pub String);

impl MediaService {
    pub async fn list_tracks(
        &self,
        owner_id: &str,
        query: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
        favorites_only: bool,
    ) -> Result<TrackPage> {
        self.list_tracks_filtered(
            owner_id,
            query,
            cursor,
            limit,
            favorites_only,
            &TrackFilters::default(),
        )
        .await
    }

    pub async fn list_tracks_filtered(
        &self,
        owner_id: &str,
        query: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
        favorites_only: bool,
        filters: &TrackFilters,
    ) -> Result<TrackPage> {
        let (genre, codec) = validate_filters(query, filters)?;
        let cursor = cursor
            .map(|cursor| {
                if cursor.len() > 128 {
                    return Err(invalid("Invalid track cursor"));
                }
                Uuid::parse_str(cursor)
                    .map(|id| id.to_string())
                    .map_err(|_| invalid("Invalid track cursor"))
            })
            .transpose()?;
        let limit = limit.clamp(1, 100) as usize;
        let fts = query
            .filter(|query| !query.trim().is_empty())
            .map(search_expression);

        // The cursor's sort key and page belong to one coherent read snapshot.
        // Ordinary UUID traversal needs no lookup and keeps its efficient index.
        let mut transaction = self.pool.begin().await?;
        let sort_key = if filters.sort != TrackSort::Library {
            if let Some(cursor) = &cursor {
                let row =
                    sqlx::query("SELECT title,added_at FROM tracks WHERE owner_id=? AND id=?")
                        .bind(owner_id)
                        .bind(cursor)
                        .fetch_optional(&mut *transaction)
                        .await?
                        .ok_or_else(|| invalid("Invalid track cursor"))?;
                Some((
                    row.try_get::<String, _>("title")?,
                    row.try_get::<i64, _>("added_at")?,
                ))
            } else {
                None
            }
        } else {
            None
        };

        if fts.as_ref().is_some_and(String::is_empty) {
            transaction.commit().await?;
            return Ok(empty_page());
        }
        // SQLite's owner/order index is ideal for broad searches, but scans an
        // entire catalog for rare matches. Probe at most 4097 FTS rowids, then
        // drive sparse searches from FTS as before (never materialize all hits).
        let sparse = if let Some(expression) = &fts {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ? LIMIT 4097)")
                .bind(expression).fetch_one(&mut *transaction).await?;
            if count == 0 {
                transaction.commit().await?;
                return Ok(empty_page());
            }
            count <= 4096
        } else {
            false
        };
        let sparse_projection;
        let projection = if sparse {
            sparse_projection = TRACK_SELECT.replace(
                "FROM tracks t",
                "FROM tracks_fts CROSS JOIN tracks t ON t.rowid=tracks_fts.rowid",
            );
            sparse_projection.as_str()
        } else {
            TRACK_SELECT
        };
        let mut builder = QueryBuilder::<Sqlite>::new(projection);
        builder
            .push_bind(owner_id)
            .push(" WHERE t.owner_id=")
            .push_bind(owner_id);
        if let Some(expression) = &fts {
            if sparse {
                builder.push(" AND tracks_fts MATCH ").push_bind(expression);
            } else {
                builder
                    .push(" AND t.rowid IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ")
                    .push_bind(expression)
                    .push(")");
            }
        }
        if favorites_only {
            builder.push(" AND f.track_id IS NOT NULL");
        }
        if let Some(genre) = genre {
            builder
                .push(" AND trim(t.genre) COLLATE NOCASE=")
                .push_bind(genre);
        }
        if let Some(codec) = codec {
            builder
                .push(" AND t.codec COLLATE NOCASE=")
                .push_bind(codec);
        }
        for (column, minimum) in [
            ("t.year", filters.year_from),
            ("t.bitrate", filters.min_bitrate),
            ("t.sample_rate", filters.min_sample_rate),
            ("t.bit_depth", filters.min_bit_depth),
        ] {
            if let Some(minimum) = minimum {
                builder
                    .push(" AND ")
                    .push(column)
                    .push(">=")
                    .push_bind(minimum);
            }
        }
        if let Some(maximum) = filters.year_to {
            builder.push(" AND t.year<=").push_bind(maximum);
        }
        match filters.sort {
            TrackSort::Library => {
                if let Some(cursor) = &cursor {
                    builder.push(" AND t.id>").push_bind(cursor);
                }
                builder.push(" ORDER BY t.id ASC");
            }
            TrackSort::Title => {
                if let (Some(cursor), Some((title, _))) = (&cursor, &sort_key) {
                    // Explicit RHS NOCASE matches ORDER BY, including case
                    // ties. Keep the indexed LHS columns bare so SQLite can
                    // seek the entire (owner,title,id) tuple for deep pages.
                    builder
                        .push(" AND (t.title,t.id)>(")
                        .push_bind(title)
                        .push(" COLLATE NOCASE,")
                        .push_bind(cursor)
                        .push(")");
                }
                builder.push(" ORDER BY t.title COLLATE NOCASE ASC,t.id ASC");
            }
            TrackSort::Newest => {
                if let (Some(cursor), Some((_, added_at))) = (&cursor, &sort_key) {
                    builder
                        .push(" AND (t.added_at,t.id)<(")
                        .push_bind(*added_at)
                        .push(",")
                        .push_bind(cursor)
                        .push(")");
                }
                builder.push(" ORDER BY t.added_at DESC,t.id DESC");
            }
        }
        builder.push(" LIMIT ").push_bind((limit + 1) as i64);
        let rows = builder.build().fetch_all(&mut *transaction).await?;
        transaction.commit().await?;
        let mut items = rows
            .iter()
            .map(track_from_row)
            .collect::<Result<Vec<_>>>()?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = has_more.then(|| items.last().unwrap().id.clone());
        Ok(TrackPage { items, next_cursor })
    }

    pub async fn catalog_facets(&self, owner_id: &str) -> Result<CatalogFacets> {
        // Both lists describe the same SQLite snapshot. Indexed grouping stops
        // after a bounded sentinel; this never fetches a full catalog into Rust.
        let mut transaction = self.pool.begin().await?;
        let mut genres: Vec<String> = sqlx::query_scalar("SELECT MIN(trim(genre)) FROM tracks WHERE owner_id=? AND trim(genre)<>'' GROUP BY trim(genre) COLLATE NOCASE ORDER BY trim(genre) COLLATE NOCASE LIMIT 101")
            .bind(owner_id).fetch_all(&mut *transaction).await?;
        let mut codecs: Vec<String> = sqlx::query_scalar("SELECT MIN(codec) FROM tracks WHERE owner_id=? AND trim(codec)<>'' GROUP BY codec COLLATE NOCASE ORDER BY codec COLLATE NOCASE LIMIT 33")
            .bind(owner_id).fetch_all(&mut *transaction).await?;
        transaction.commit().await?;
        let genres_truncated = genres.len() > 100;
        let codecs_truncated = codecs.len() > 32;
        genres.truncate(100);
        codecs.truncate(32);
        Ok(CatalogFacets {
            genres,
            codecs,
            genres_truncated,
            codecs_truncated,
        })
    }
}

fn invalid(message: &str) -> InvalidTrackFilter {
    InvalidTrackFilter(message.to_owned())
}

fn validate_filters<'a>(
    query: Option<&str>,
    filters: &'a TrackFilters,
) -> Result<(Option<&'a str>, Option<&'a str>)> {
    if query.is_some_and(|query| query.len() > 256) {
        return Err(invalid("Search query must not exceed 256 bytes").into());
    }
    let genre = validated_text(filters.genre.as_deref(), 1024, "Invalid genre filter")?;
    let codec = validated_text(filters.codec.as_deref(), 64, "Invalid format filter")?;
    for (value, maximum, message) in [
        (filters.year_from, 9999, "Invalid release year range"),
        (filters.year_to, 9999, "Invalid release year range"),
        (filters.min_bitrate, 1_000_000, "Invalid minimum bitrate"),
        (
            filters.min_sample_rate,
            1_536_000,
            "Invalid minimum sample rate",
        ),
        (filters.min_bit_depth, 64, "Invalid minimum bit depth"),
    ] {
        if value.is_some_and(|value| !(1..=maximum).contains(&value)) {
            return Err(invalid(message).into());
        }
    }
    if let (Some(from), Some(to)) = (filters.year_from, filters.year_to)
        && from > to
    {
        return Err(invalid("Invalid release year range").into());
    }
    Ok((genre, codec))
}

fn validated_text<'a>(
    value: Option<&'a str>,
    maximum: usize,
    message: &str,
) -> Result<Option<&'a str>> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
                return Err(invalid(message).into());
            }
            Ok(trimmed)
        })
        .transpose()
}

fn empty_page() -> TrackPage {
    TrackPage {
        items: Vec::new(),
        next_cursor: None,
    }
}

fn search_expression(query: &str) -> String {
    query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .take(20)
        .map(|token| format!("\"{}\"*", token.chars().take(64).collect::<String>()))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_operators_are_quoted_literals() {
        assert_eq!(
            search_expression("hello OR artist:secret*"),
            "\"hello\"* AND \"OR\"* AND \"artist\"* AND \"secret\"*"
        );
        assert_eq!(
            search_expression("\"; DROP TABLE tracks; --"),
            "\"DROP\"* AND \"TABLE\"* AND \"tracks\"*"
        );
        assert_eq!(search_expression("***"), "");
        assert!(validate_filters(Some(&"a".repeat(257)), &TrackFilters::default()).is_err());
    }
}
