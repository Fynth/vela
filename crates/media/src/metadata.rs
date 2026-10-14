//! Authorized revisioned catalog edits; original media assets are immutable.
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite, sqlite::SqliteRow};
use vela_domain::{TrackMetadata, UpdateTrackMetadata};

use crate::{MediaService, TRACK_SELECT, track_from_row};

#[derive(Debug, thiserror::Error)]
pub enum CatalogMetadataError {
    #[error("{0}")]
    Invalid(String),
    #[error("Track metadata changed; reload before saving")]
    Conflict,
}

impl MediaService {
    pub async fn metadata(&self, owner_id: &str, id: &str) -> Result<Option<TrackMetadata>> {
        metadata_query(owner_id, id)
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(snapshot_from_row)
            .transpose()
    }

    pub async fn update_metadata(
        &self,
        owner_id: &str,
        id: &str,
        input: UpdateTrackMetadata,
    ) -> Result<Option<TrackMetadata>> {
        // Acquire the SQLite writer before reading the expected revision. This
        // avoids a deferred read transaction racing another editor's commit.
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let Some(row) = metadata_query(owner_id, id)
            .build()
            .fetch_optional(&mut *transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        // Ownership is checked before validation or conflict reporting, so a
        // collaborator cannot use this endpoint to probe private revisions.
        let input = validate_input(input)?;
        let current_revision: i64 = row.try_get("metadata_revision")?;
        if current_revision != input.revision {
            return Err(CatalogMetadataError::Conflict.into());
        }
        let next_revision = current_revision.checked_add(1).ok_or_else(|| {
            CatalogMetadataError::Invalid("Metadata revision cannot be advanced".into())
        })?;
        let changed = sqlx::query("UPDATE tracks SET title=?,artist=?,album=?,album_artist=?,genre=?,year=?,track_number=?,disc_number=?,metadata_revision=? WHERE owner_id=? AND id=? AND metadata_revision=?")
            .bind(input.title).bind(input.artist).bind(input.album).bind(input.album_artist)
            .bind(input.genre).bind(input.year).bind(input.track_number).bind(input.disc_number)
            .bind(next_revision).bind(owner_id).bind(id).bind(current_revision)
            .execute(&mut *transaction).await?;
        if changed.rows_affected() != 1 {
            return Err(CatalogMetadataError::Conflict.into());
        }
        sqlx::query("INSERT INTO audit_events(actor_id,action,object_id) VALUES(?,'track.metadata_updated',?)")
            .bind(owner_id).bind(id).execute(&mut *transaction).await?;
        let row = metadata_query(owner_id, id)
            .build()
            .fetch_one(&mut *transaction)
            .await?;
        let snapshot = snapshot_from_row(&row)?;
        transaction.commit().await?;
        Ok(Some(snapshot))
    }
}

fn metadata_query<'a>(owner_id: &'a str, id: &'a str) -> QueryBuilder<'a, Sqlite> {
    let mut builder = QueryBuilder::<Sqlite>::new(TRACK_SELECT.replacen(
        "SELECT ",
        "SELECT t.metadata_revision,",
        1,
    ));
    builder
        .push_bind(owner_id)
        .push(" WHERE t.owner_id=")
        .push_bind(owner_id)
        .push(" AND t.id=")
        .push_bind(id);
    builder
}

fn snapshot_from_row(row: &SqliteRow) -> Result<TrackMetadata> {
    Ok(TrackMetadata {
        track: track_from_row(row)?,
        revision: row.try_get("metadata_revision")?,
    })
}

fn validate_input(mut input: UpdateTrackMetadata) -> Result<UpdateTrackMetadata> {
    if input.revision < 0 {
        return Err(CatalogMetadataError::Invalid("Revision must be nonnegative".into()).into());
    }
    input.title = validate_text("Title", input.title, 512, true)?;
    input.artist = validate_text("Artist", input.artist, 1024, false)?;
    input.album = validate_text("Album", input.album, 1024, false)?;
    input.album_artist = validate_text("Album artist", input.album_artist, 1024, false)?;
    input.genre = validate_text("Genre", input.genre, 1024, false)?;
    for (label, value) in [
        ("Year", input.year),
        ("Track number", input.track_number),
        ("Disc number", input.disc_number),
    ] {
        if value.is_some_and(|number| !(1..=9999).contains(&number)) {
            return Err(CatalogMetadataError::Invalid(format!(
                "{label} must be between 1 and 9999 or unknown"
            ))
            .into());
        }
    }
    Ok(input)
}

fn validate_text(label: &str, value: String, max_bytes: usize, required: bool) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(CatalogMetadataError::Invalid(format!(
            "{label} must not contain control characters"
        ))
        .into());
    }
    let trimmed = value.trim();
    if required && trimmed.is_empty() {
        return Err(CatalogMetadataError::Invalid(format!("{label} must not be empty")).into());
    }
    if trimmed.len() > max_bytes {
        return Err(CatalogMetadataError::Invalid(format!(
            "{label} must not exceed {max_bytes} bytes"
        ))
        .into());
    }
    Ok(trimmed.to_owned())
}
