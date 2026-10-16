//! Private-by-default playlists with server-authoritative revision checks.
use crate::{
    Error, Result,
    auth::{random_token, token_hash},
    now,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use std::collections::HashSet;
use vela_domain::*;

#[derive(Clone)]
pub struct PlaylistService {
    pool: SqlitePool,
}
impl PlaylistService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    pub async fn create(&self, user: &User, input: CreatePlaylist) -> Result<PlaylistSummary> {
        let name = input.name.trim();
        if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
            return Err(Error::Invalid(
                "Playlist name must be 1–120 characters".into(),
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let inserted=sqlx::query("INSERT INTO playlists(id,owner_id,name) SELECT ?,?,? WHERE (SELECT COUNT(*) FROM playlists WHERE owner_id=?) < 200").bind(&id).bind(&user.id).bind(name).bind(&user.id).execute(&self.pool).await?;
        if inserted.rows_affected() == 0 {
            return Err(Error::Invalid("Playlist limit reached".into()));
        }
        Ok(PlaylistSummary {
            id,
            name: name.into(),
            owner_id: user.id.clone(),
            revision: 0,
            item_count: 0,
            can_edit: true,
        })
    }
    pub async fn list(&self, user: &User) -> Result<Vec<PlaylistSummary>> {
        let rows=sqlx::query("SELECT p.*, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id=p.id) item_count, (p.owner_id=? OR COALESCE(c.can_edit,0)) can_edit FROM playlists p LEFT JOIN playlist_collaborators c ON c.playlist_id=p.id AND c.user_id=? WHERE p.owner_id=? OR c.user_id IS NOT NULL ORDER BY p.created_at DESC,p.id LIMIT 200").bind(&user.id).bind(&user.id).bind(&user.id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(summary).collect())
    }
    /// Search only owned or explicitly granted names. A result grants no media.
    pub async fn search(&self, user: &User, query: &str) -> Result<Vec<PlaylistSummary>> {
        if query.len() > 256 || query.trim().is_empty() || query.chars().any(char::is_control) {
            return Err(Error::Invalid(
                "Search must contain 1–256 bytes without control characters".into(),
            ));
        }
        let rows = sqlx::query("SELECT p.*, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id=p.id) item_count, (p.owner_id=? OR COALESCE(c.can_edit,0)) can_edit FROM playlists p LEFT JOIN playlist_collaborators c ON c.playlist_id=p.id AND c.user_id=? WHERE (p.owner_id=? OR c.user_id IS NOT NULL) AND instr(lower(p.name),lower(?))>0 ORDER BY p.name COLLATE NOCASE,p.id LIMIT 8")
            .bind(&user.id).bind(&user.id).bind(&user.id).bind(query.trim()).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(summary).collect())
    }
    pub async fn detail(&self, user: &User, id: &str) -> Result<PlaylistDetail> {
        let row=sqlx::query("SELECT p.*, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id=p.id) item_count, (p.owner_id=? OR COALESCE(c.can_edit,0)) can_edit FROM playlists p LEFT JOIN playlist_collaborators c ON c.playlist_id=p.id AND c.user_id=? WHERE p.id=? AND (p.owner_id=? OR c.user_id IS NOT NULL)").bind(&user.id).bind(&user.id).bind(id).bind(&user.id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        self.detail_for(summary(&row), Some(&user.id)).await
    }
    async fn detail_for(
        &self,
        mut playlist: PlaylistSummary,
        user_id: Option<&str>,
    ) -> Result<PlaylistDetail> {
        let rows=sqlx::query("SELECT t.*,a.size_bytes,i.id item_id,i.position,COALESCE(f.track_id IS NOT NULL,0) favorite FROM playlist_items i JOIN tracks t ON t.id=i.track_id JOIN media_assets a ON a.id=t.media_asset_id LEFT JOIN favorites f ON f.track_id=t.id AND f.user_id=? WHERE i.playlist_id=? AND (? IS NOT NULL OR t.owner_id=?) ORDER BY i.position LIMIT 1000").bind(user_id).bind(&playlist.id).bind(user_id).bind(&playlist.owner_id).fetch_all(&self.pool).await?;
        let items: Vec<PlaylistEntry> = rows
            .iter()
            .map(|r| PlaylistEntry {
                id: r.get("item_id"),
                track: track(r),
                position: r.get("position"),
                can_stream: user_id.is_some_and(|u| u == r.get::<String, _>("owner_id")),
            })
            .collect();
        if user_id.is_none() {
            playlist.item_count = items.len() as i64;
        }
        Ok(PlaylistDetail { playlist, items })
    }
    pub async fn add(
        &self,
        user: &User,
        id: &str,
        input: PlaylistMutation,
    ) -> Result<PlaylistDetail> {
        let track_id = input
            .track_id
            .as_ref()
            .ok_or_else(|| Error::Invalid("track_id is required".into()))?;
        let hash = mutation_hash("add", &input)?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if !begin_mutation(&mut tx, user, id, &input, &hash).await? {
            tx.commit().await?;
            return self.detail(user, id).await;
        }
        let permitted: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tracks WHERE id=? AND owner_id=?)")
                .bind(track_id)
                .bind(&user.id)
                .fetch_one(&mut *tx)
                .await?;
        if !permitted {
            return Err(Error::NotFound);
        }
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM playlist_items WHERE playlist_id=?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 1000 {
            return Err(Error::Invalid(
                "Playlist has reached the 1000 entry limit".into(),
            ));
        }
        sqlx::query("INSERT INTO playlist_items(id,playlist_id,track_id,position) VALUES(?,?,?,COALESCE((SELECT MAX(position)+1 FROM playlist_items WHERE playlist_id=?),0))").bind(uuid::Uuid::new_v4().to_string()).bind(id).bind(track_id).bind(id).execute(&mut *tx).await?;
        finish_mutation(&mut tx, user, id, &input, &hash).await?;
        tx.commit().await?;
        self.detail(user, id).await
    }
    pub async fn reorder(
        &self,
        user: &User,
        id: &str,
        input: PlaylistMutation,
    ) -> Result<PlaylistDetail> {
        let item_ids = input
            .item_ids
            .as_ref()
            .ok_or_else(|| Error::Invalid("item_ids is required".into()))?;
        if item_ids.len() > 1000 {
            return Err(Error::Invalid("Too many entries".into()));
        }
        let hash = mutation_hash("reorder", &input)?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if !begin_mutation(&mut tx, user, id, &input, &hash).await? {
            tx.commit().await?;
            return self.detail(user, id).await;
        }
        let existing: Vec<String> =
            sqlx::query_scalar("SELECT id FROM playlist_items WHERE playlist_id=?")
                .bind(id)
                .fetch_all(&mut *tx)
                .await?;
        if existing.len() != item_ids.len()
            || item_ids.iter().collect::<HashSet<_>>().len() != existing.len()
            || existing.iter().any(|id| !item_ids.contains(id))
        {
            return Err(Error::Invalid(
                "Order must contain every entry exactly once".into(),
            ));
        }
        // Shift existing positions above their maximum before assigning the new order.
        sqlx::query("UPDATE playlist_items SET position=position+(SELECT COALESCE(MAX(position),0)+1 FROM playlist_items WHERE playlist_id=?) WHERE playlist_id=?").bind(id).bind(id).execute(&mut *tx).await?;
        for (position, item) in item_ids.iter().enumerate() {
            sqlx::query("UPDATE playlist_items SET position=? WHERE playlist_id=? AND id=?")
                .bind(position as i64)
                .bind(id)
                .bind(item)
                .execute(&mut *tx)
                .await?;
        }
        finish_mutation(&mut tx, user, id, &input, &hash).await?;
        tx.commit().await?;
        self.detail(user, id).await
    }
    pub async fn remove(
        &self,
        user: &User,
        id: &str,
        item: &str,
        input: PlaylistMutation,
    ) -> Result<PlaylistDetail> {
        let hash = mutation_hash(&format!("remove:{item}"), &input)?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if !begin_mutation(&mut tx, user, id, &input, &hash).await? {
            tx.commit().await?;
            return self.detail(user, id).await;
        }
        let result = sqlx::query("DELETE FROM playlist_items WHERE playlist_id=? AND id=?")
            .bind(id)
            .bind(item)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }
        finish_mutation(&mut tx, user, id, &input, &hash).await?;
        tx.commit().await?;
        self.detail(user, id).await
    }
    async fn require_owner(&self, user: &User, id: &str) -> Result<()> {
        if !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM playlists WHERE id=? AND owner_id=?)",
        )
        .bind(id)
        .bind(&user.id)
        .fetch_one(&self.pool)
        .await?
        {
            return Err(Error::NotFound);
        }
        Ok(())
    }
    pub async fn collaborate(
        &self,
        user: &User,
        id: &str,
        input: CollaboratorRequest,
    ) -> Result<()> {
        self.require_owner(user, id).await?;
        let target: String =
            sqlx::query_scalar("SELECT id FROM users WHERE username=? COLLATE NOCASE")
                .bind(input.username)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(Error::NotFound)?;
        if target == user.id {
            return Err(Error::Invalid("Owner already has access".into()));
        }
        let updated=sqlx::query("INSERT INTO playlist_collaborators(playlist_id,user_id,can_edit) SELECT ?,?,? WHERE (SELECT COUNT(*) FROM playlist_collaborators WHERE playlist_id=?) < 100 OR EXISTS(SELECT 1 FROM playlist_collaborators WHERE playlist_id=? AND user_id=?) ON CONFLICT(playlist_id,user_id) DO UPDATE SET can_edit=excluded.can_edit").bind(id).bind(&target).bind(input.can_edit).bind(id).bind(id).bind(&target).execute(&self.pool).await?;
        if updated.rows_affected() == 0 {
            return Err(Error::Invalid("Playlist collaborator limit reached".into()));
        }
        audit(&self.pool, &user.id, "playlist.collaborator", id).await
    }
    pub async fn collaborators(&self, user: &User, id: &str) -> Result<Vec<PlaylistCollaborator>> {
        self.require_owner(user, id).await?;
        let rows=sqlx::query("SELECT u.id user_id,u.username,c.can_edit FROM playlist_collaborators c JOIN users u ON u.id=c.user_id WHERE c.playlist_id=? ORDER BY u.username LIMIT 100")
            .bind(id).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| PlaylistCollaborator {
                user_id: r.get("user_id"),
                username: r.get("username"),
                can_edit: r.get("can_edit"),
            })
            .collect())
    }
    pub async fn revoke_collaborator(&self, user: &User, id: &str, target: &str) -> Result<()> {
        if target != user.id {
            self.require_owner(user, id).await?;
        } else if sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM playlists WHERE id=? AND owner_id=?)",
        )
        .bind(id)
        .bind(&user.id)
        .fetch_one(&self.pool)
        .await?
        {
            return Err(Error::Invalid(
                "An owner cannot leave their own playlist".into(),
            ));
        }
        let removed =
            sqlx::query("DELETE FROM playlist_collaborators WHERE playlist_id=? AND user_id=?")
                .bind(id)
                .bind(target)
                .execute(&self.pool)
                .await?;
        if removed.rows_affected() == 0 {
            return Err(Error::NotFound);
        }
        audit(&self.pool, &user.id, "playlist.collaborator.revoke", id).await
    }
    pub async fn share(&self, user: &User, id: &str, input: ShareRequest) -> Result<ShareResponse> {
        self.require_owner(user, id).await?;
        if !(60..=604800).contains(&input.expires_in_seconds) {
            return Err(Error::Invalid(
                "Share expiry must be 1 minute to 7 days".into(),
            ));
        }
        let token = random_token();
        let share_id = uuid::Uuid::new_v4().to_string();
        let expires_at = now() + input.expires_in_seconds;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM playlist_shares WHERE expires_at<=?")
            .bind(now())
            .execute(&mut *tx)
            .await?;
        let result=sqlx::query("INSERT INTO playlist_shares(id,playlist_id,token_hash,expires_at) SELECT ?,?,?,? WHERE (SELECT COUNT(*) FROM playlist_shares WHERE playlist_id=?) < 20").bind(&share_id).bind(id).bind(token_hash(&token)).bind(expires_at).bind(id).execute(&mut *tx).await?;
        if result.rows_affected() == 0 {
            return Err(Error::Invalid(
                "Share link limit reached; revoke old links".into(),
            ));
        }
        tx.commit().await?;
        audit(&self.pool, &user.id, "playlist.share", id).await?;
        Ok(ShareResponse {
            id: share_id,
            token,
            expires_at,
        })
    }
    pub async fn revoke_shares(&self, user: &User, id: &str) -> Result<()> {
        self.require_owner(user, id).await?;
        sqlx::query("DELETE FROM playlist_shares WHERE playlist_id=?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        audit(&self.pool, &user.id, "playlist.share.revoke", id).await
    }
    pub async fn shared(&self, token: &str) -> Result<PlaylistDetail> {
        if token.len() != 64 {
            return Err(Error::NotFound);
        }
        let row=sqlx::query("SELECT p.*,0 can_edit,(SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id=p.id) item_count FROM playlist_shares s JOIN playlists p ON p.id=s.playlist_id WHERE s.token_hash=? AND s.expires_at>?").bind(token_hash(token)).bind(now()).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        self.detail_for(summary(&row), None).await
    }
}
fn summary(r: &sqlx::sqlite::SqliteRow) -> PlaylistSummary {
    PlaylistSummary {
        id: r.get("id"),
        name: r.get("name"),
        owner_id: r.get("owner_id"),
        revision: r.get("revision"),
        item_count: r.get("item_count"),
        can_edit: r.get("can_edit"),
    }
}
fn track(r: &sqlx::sqlite::SqliteRow) -> Track {
    Track {
        id: r.get("id"),
        title: r.get("title"),
        artist: r.get("artist"),
        album: r.get("album"),
        album_artist: r.get("album_artist"),
        genre: r.get("genre"),
        year: r.get("year"),
        track_number: r.get("track_number"),
        disc_number: r.get("disc_number"),
        duration_ms: r.get("duration_ms"),
        codec: r.get("codec"),
        bitrate: r.get("bitrate"),
        sample_rate: r.get("sample_rate"),
        bit_depth: r.get("bit_depth"),
        channels: r.get("channels"),
        size_bytes: r.get("size_bytes"),
        artwork_id: r.get("artwork_id"),
        added_at: r.get("added_at"),
        favorite: r.get("favorite"),
    }
}
fn mutation_hash(action: &str, input: &PlaylistMutation) -> Result<String> {
    if uuid::Uuid::parse_str(&input.mutation_id).is_err() || input.revision < 0 {
        return Err(Error::Invalid(
            "A valid mutation UUID and revision are required".into(),
        ));
    }
    Ok(token_hash(&format!(
        "{action}:{}",
        serde_json::to_string(input).map_err(|e| Error::Internal(e.into()))?
    )))
}
async fn begin_mutation(
    tx: &mut Transaction<'_, Sqlite>,
    user: &User,
    id: &str,
    input: &PlaylistMutation,
    hash: &str,
) -> Result<bool> {
    let access:Option<bool>=sqlx::query_scalar("SELECT p.owner_id=? OR COALESCE(c.can_edit,0) FROM playlists p LEFT JOIN playlist_collaborators c ON c.playlist_id=p.id AND c.user_id=? WHERE p.id=?").bind(&user.id).bind(&user.id).bind(id).fetch_optional(&mut **tx).await?;
    match access {
        None => return Err(Error::NotFound),
        Some(false) => return Err(Error::Forbidden),
        Some(true) => {}
    }
    if let Some(row)=sqlx::query("SELECT actor_id,request_hash FROM playlist_mutations WHERE playlist_id=? AND mutation_id=?").bind(id).bind(&input.mutation_id).fetch_optional(&mut **tx).await? {
        if row.get::<String,_>("actor_id")==user.id && row.get::<String,_>("request_hash")==hash { return Ok(false); }
        return Err(Error::Conflict("Mutation identifier was already used for another edit".into()));
    }
    let result = sqlx::query("UPDATE playlists SET revision=revision+1 WHERE id=? AND revision=?")
        .bind(id)
        .bind(input.revision)
        .execute(&mut **tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Conflict(
            "Playlist changed. Refresh and retry the edit.".into(),
        ));
    }
    Ok(true)
}
async fn finish_mutation(
    tx: &mut Transaction<'_, Sqlite>,
    user: &User,
    id: &str,
    input: &PlaylistMutation,
    hash: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO playlist_mutations(playlist_id,mutation_id,actor_id,request_hash,result_revision) VALUES(?,?,?,?,?)").bind(id).bind(&input.mutation_id).bind(&user.id).bind(hash).bind(input.revision+1).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO audit_events(actor_id,action,object_id) VALUES(?,'playlist.edit',?)")
        .bind(&user.id)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn audit(pool: &SqlitePool, actor: &str, action: &str, id: &str) -> Result<()> {
    sqlx::query("INSERT INTO audit_events(actor_id,action,object_id) VALUES(?,?,?)")
        .bind(actor)
        .bind(action)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
