//! Private, explicitly consented audible listening progress and history.
//!
//! Client progress is a personal journal, not proof of physical sound or a
//! public popularity signal. Every write serializes consent and ownership with
//! the progress mutation; a privacy change invalidates all prior revisions.
use crate::{Error, Result, now};
use sqlx::{Row, Sqlite, SqlitePool, Transaction, sqlite::SqliteRow};
use vela_domain::*;

const DEFAULT_RETENTION: u16 = 90;
const MAX_DEVICES: i64 = 32;
const MAX_SESSIONS: i64 = 10_000;
const DAY: i64 = 86_400;

#[derive(Clone)]
pub struct ListeningService {
    pool: SqlitePool,
}

impl ListeningService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn preferences(&self, user: &User, device_id: &str) -> Result<ListeningPreferences> {
        uuid(device_id)?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        prune_owner(&mut tx, &user.id, now()).await?;
        let preferences = preferences(&mut tx, &user.id, device_id).await?;
        tx.commit().await?;
        Ok(preferences)
    }

    pub async fn update_preferences(
        &self,
        user: &User,
        input: UpdateListeningPreferences,
    ) -> Result<ListeningPreferences> {
        uuid(&input.device_id)?;
        if input.revision < 0 || !(7..=365).contains(&input.retention_days) {
            return Err(invalid(
                "A current revision and retention of 7–365 days are required",
            ));
        }
        let timestamp = now();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        ensure_preferences(&mut tx, &user.id, timestamp).await?;
        let current = preferences(&mut tx, &user.id, &input.device_id).await?;
        if current.revision != input.revision || current.revision == i64::MAX {
            return Err(conflict(
                "Listening preferences changed; reload them before saving",
            ));
        }
        let known: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM listening_devices WHERE user_id=? AND device_id=?)",
        )
        .bind(&user.id)
        .bind(&input.device_id)
        .fetch_one(&mut *tx)
        .await?;
        if !known {
            let devices: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM listening_devices WHERE user_id=?")
                    .bind(&user.id)
                    .fetch_one(&mut *tx)
                    .await?;
            if devices >= MAX_DEVICES {
                return Err(invalid("Listening device limit reached"));
            }
        }
        sqlx::query("INSERT INTO listening_devices(user_id,device_id,enabled) VALUES(?,?,?) ON CONFLICT(user_id,device_id) DO UPDATE SET enabled=excluded.enabled")
            .bind(&user.id).bind(&input.device_id).bind(input.device_enabled)
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE listening_preferences SET enabled=?,retention_days=?,revision=revision+1,updated_at=? WHERE user_id=?")
            .bind(input.enabled).bind(i64::from(input.retention_days)).bind(timestamp).bind(&user.id)
            .execute(&mut *tx).await?;
        prune_owner(&mut tx, &user.id, timestamp).await?;
        let result = preferences(&mut tx, &user.id, &input.device_id).await?;
        tx.commit().await?;
        Ok(result)
    }

    pub async fn record(&self, user: &User, batch: ListeningBatch) -> Result<ListeningReceipt> {
        uuid(&batch.device_id)?;
        if batch.revision < 0 || batch.events.is_empty() || batch.events.len() > 20 {
            return Err(invalid(
                "A current revision and 1–20 listening updates are required",
            ));
        }
        for event in &batch.events {
            validate_progress(event)?;
        }
        let timestamp = now();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let consent = preferences(&mut tx, &user.id, &batch.device_id).await?;
        if consent.revision != batch.revision {
            return Err(conflict("Listening consent changed; discard old updates"));
        }
        if !consent.enabled || !consent.device_enabled {
            return Err(Error::Forbidden);
        }
        prune_owner(&mut tx, &user.id, timestamp).await?;
        let cutoff = timestamp - i64::from(consent.retention_days) * DAY;
        let mut receipt = ListeningReceipt {
            accepted: 0,
            duplicates: 0,
            revision: consent.revision,
        };
        for event in batch.events {
            // Even retries and older updates must refer to an owned track. A
            // playlist grant is deliberately irrelevant to this private journal.
            let duration: Option<i64> =
                sqlx::query_scalar("SELECT duration_ms FROM tracks WHERE id=? AND owner_id=?")
                    .bind(&event.track_id)
                    .bind(&user.id)
                    .fetch_optional(&mut *tx)
                    .await?;
            let duration = duration.ok_or(Error::NotFound)?;
            if event.started_at < cutoff || event.started_at > timestamp + 5 {
                return Err(invalid(
                    "Listening start time is outside the retained time window",
                ));
            }
            let wall_ms = (timestamp - event.started_at + 5).max(0) * 1_000;
            if event.listened_ms > wall_ms {
                return Err(invalid("Listened time exceeds elapsed wall time"));
            }
            let existing =
                sqlx::query("SELECT * FROM listening_sessions WHERE user_id=? AND session_id=?")
                    .bind(&user.id)
                    .bind(&event.session_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some(row) = existing {
                let previous = progress(&row);
                if row.get::<String, _>("device_id") != batch.device_id
                    || previous.track_id != event.track_id
                    || previous.started_at != event.started_at
                {
                    return Err(conflict(
                        "A listening session cannot change its track, device or start time",
                    ));
                }
                if event.sequence < previous.sequence || event == previous {
                    receipt.duplicates += 1;
                    continue;
                }
                if event.sequence == previous.sequence {
                    return Err(conflict(
                        "A listening sequence was already used for different progress",
                    ));
                }
                if event.listened_ms < previous.listened_ms || previous.ended {
                    return Err(conflict(
                        "Listening progress cannot decrease or reopen a completed session",
                    ));
                }
                let qualified =
                    row.get::<bool, _>("qualified") || qualifies(event.listened_ms, duration);
                sqlx::query("UPDATE listening_sessions SET sequence=?,listened_ms=?,position_ms=?,updated_at=?,qualified=?,ended=? WHERE user_id=? AND session_id=?")
                    .bind(i64::from(event.sequence)).bind(event.listened_ms).bind(event.position_ms)
                    .bind(timestamp).bind(qualified).bind(event.ended).bind(&user.id).bind(&event.session_id)
                    .execute(&mut *tx).await?;
            } else {
                let count: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM listening_sessions WHERE user_id=?")
                        .bind(&user.id)
                        .fetch_one(&mut *tx)
                        .await?;
                if count >= MAX_SESSIONS {
                    return Err(invalid(
                        "Listening history limit reached; export or clear history",
                    ));
                }
                sqlx::query("INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
                    .bind(&user.id).bind(&event.session_id).bind(&batch.device_id).bind(&event.track_id)
                    .bind(i64::from(event.sequence)).bind(event.listened_ms).bind(event.position_ms)
                    .bind(event.started_at).bind(timestamp).bind(qualifies(event.listened_ms, duration)).bind(event.ended)
                    .execute(&mut *tx).await?;
            }
            receipt.accepted += 1;
        }
        tx.commit().await?;
        Ok(receipt)
    }

    pub async fn history(
        &self,
        user: &User,
        query: &ListeningHistoryQuery,
    ) -> Result<ListeningHistoryPage> {
        let limit = query.limit.unwrap_or(20);
        if !(1..=50).contains(&limit) {
            return Err(invalid("History page size must be 1–50"));
        }
        let cursor = query.cursor.as_deref().map(parse_cursor).transpose()?;
        let timestamp = now();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        prune_owner(&mut tx, &user.id, timestamp).await?;
        let retention = retention(&mut tx, &user.id).await?;
        let cutoff = timestamp - i64::from(retention) * DAY;
        let totals = sqlx::query("SELECT COUNT(*) total_listens,COALESCE(SUM(s.listened_ms),0) total_listened_ms FROM listening_sessions s JOIN tracks t ON t.id=s.track_id AND t.owner_id=s.user_id WHERE s.user_id=? AND s.qualified=1 AND s.started_at>=?")
            .bind(&user.id).bind(cutoff).fetch_one(&mut *tx).await?;
        let mut builder = sqlx::QueryBuilder::<Sqlite>::new(
            "SELECT t.*,a.size_bytes,(f.track_id IS NOT NULL) favorite,s.session_id,s.listened_ms,s.position_ms,s.started_at,s.updated_at,s.ended FROM listening_sessions s JOIN tracks t ON t.id=s.track_id AND t.owner_id=s.user_id JOIN media_assets a ON a.id=t.media_asset_id LEFT JOIN favorites f ON f.track_id=t.id AND f.user_id=s.user_id WHERE s.user_id=",
        );
        builder
            .push_bind(&user.id)
            .push(" AND s.qualified=1 AND s.started_at>=")
            .push_bind(cutoff);
        if let Some((at, id)) = cursor {
            builder
                .push(" AND (s.updated_at<")
                .push_bind(at)
                .push(" OR (s.updated_at=")
                .push_bind(at)
                .push(" AND s.session_id<")
                .push_bind(id)
                .push("))");
        }
        builder
            .push(" ORDER BY s.updated_at DESC,s.session_id DESC LIMIT ")
            .push_bind(i64::from(limit) + 1);
        let rows = builder.build().fetch_all(&mut *tx).await?;
        let has_more = rows.len() > limit as usize;
        let items: Vec<ListeningEntry> = rows
            .iter()
            .take(limit as usize)
            .map(|row| ListeningEntry {
                session_id: row.get("session_id"),
                track: track(row),
                listened_ms: row.get("listened_ms"),
                position_ms: row.get("position_ms"),
                started_at: row.get("started_at"),
                updated_at: row.get("updated_at"),
                ended: row.get("ended"),
            })
            .collect();
        let next_cursor = if has_more {
            items
                .last()
                .map(|item| format!("v1:{}:{}", item.updated_at, item.session_id))
        } else {
            None
        };
        let page = ListeningHistoryPage {
            items,
            next_cursor,
            total_listens: totals.get("total_listens"),
            total_listened_ms: totals.get("total_listened_ms"),
        };
        tx.commit().await?;
        Ok(page)
    }

    pub async fn export(&self, user: &User) -> Result<ListeningExport> {
        let timestamp = now();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        prune_owner(&mut tx, &user.id, timestamp).await?;
        let row =
            sqlx::query("SELECT enabled,retention_days FROM listening_preferences WHERE user_id=?")
                .bind(&user.id)
                .fetch_optional(&mut *tx)
                .await?;
        let (enabled, retention_days) = row
            .map(|row| {
                (
                    row.get("enabled"),
                    row.get::<i64, _>("retention_days") as u16,
                )
            })
            .unwrap_or((false, DEFAULT_RETENTION));
        let rows = sqlx::query("SELECT s.* FROM listening_sessions s JOIN tracks t ON t.id=s.track_id AND t.owner_id=s.user_id WHERE s.user_id=? AND s.started_at>=? ORDER BY s.updated_at DESC,s.session_id DESC LIMIT 10000")
            .bind(&user.id).bind(timestamp - i64::from(retention_days) * DAY).fetch_all(&mut *tx).await?;
        let events = rows
            .iter()
            .map(|row| ListeningExportEntry {
                device_id: row.get("device_id"),
                progress: progress(row),
                updated_at: row.get("updated_at"),
                qualified: row.get("qualified"),
            })
            .collect();
        tx.commit().await?;
        Ok(ListeningExport {
            format_version: 1,
            exported_at: timestamp,
            enabled,
            retention_days,
            events,
        })
    }

    /// Erasure also revokes collection, so delayed batches cannot resurrect it.
    pub async fn devices(&self, user: &User) -> Result<ListeningDevices> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        prune_owner(&mut tx, &user.id, now()).await?;
        ensure_preferences(&mut tx, &user.id, now()).await?;
        let devices = devices_in(&mut tx, &user.id).await?;
        tx.commit().await?;
        Ok(devices)
    }

    pub async fn revoke_device(
        &self,
        user: &User,
        device_id: &str,
        revision: i64,
    ) -> Result<ListeningDevices> {
        uuid(device_id)?;
        if revision < 0 {
            return Err(invalid("A current revision is required"));
        }
        let timestamp = now();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        ensure_preferences(&mut tx, &user.id, timestamp).await?;
        let current: i64 =
            sqlx::query_scalar("SELECT revision FROM listening_preferences WHERE user_id=?")
                .bind(&user.id)
                .fetch_one(&mut *tx)
                .await?;
        if current != revision || current == i64::MAX {
            return Err(conflict(
                "Listening privacy settings changed; reload before revoking a device",
            ));
        }
        let removed = sqlx::query("DELETE FROM listening_devices WHERE user_id=? AND device_id=?")
            .bind(&user.id)
            .bind(device_id)
            .execute(&mut *tx)
            .await?;
        if removed.rows_affected() == 0 {
            return Err(Error::NotFound);
        }
        sqlx::query(
            "UPDATE listening_preferences SET revision=revision+1,updated_at=? WHERE user_id=?",
        )
        .bind(timestamp)
        .bind(&user.id)
        .execute(&mut *tx)
        .await?;
        prune_owner(&mut tx, &user.id, timestamp).await?;
        let devices = devices_in(&mut tx, &user.id).await?;
        tx.commit().await?;
        Ok(devices)
    }

    pub async fn clear(&self, user: &User, device_id: &str) -> Result<ListeningPreferences> {
        uuid(device_id)?;
        let timestamp = now();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        ensure_preferences(&mut tx, &user.id, timestamp).await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM listening_preferences WHERE user_id=?")
                .bind(&user.id)
                .fetch_one(&mut *tx)
                .await?;
        if revision == i64::MAX {
            return Err(conflict("Listening preferences revision limit reached"));
        }
        sqlx::query("DELETE FROM listening_sessions WHERE user_id=?")
            .bind(&user.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE listening_devices SET enabled=0 WHERE user_id=?")
            .bind(&user.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE listening_preferences SET enabled=0,revision=revision+1,updated_at=? WHERE user_id=?")
            .bind(timestamp).bind(&user.id).execute(&mut *tx).await?;
        let result = preferences(&mut tx, &user.id, device_id).await?;
        tx.commit().await?;
        Ok(result)
    }

    pub async fn prune(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM listening_sessions WHERE started_at < ? - COALESCE((SELECT retention_days FROM listening_preferences p WHERE p.user_id=listening_sessions.user_id),90)*86400")
            .bind(now()).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}

async fn ensure_preferences(
    tx: &mut Transaction<'_, Sqlite>,
    owner: &str,
    timestamp: i64,
) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO listening_preferences(user_id,updated_at) VALUES(?,?)")
        .bind(owner)
        .bind(timestamp)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn preferences(
    tx: &mut Transaction<'_, Sqlite>,
    owner: &str,
    device_id: &str,
) -> Result<ListeningPreferences> {
    let row = sqlx::query(
        "SELECT enabled,retention_days,revision FROM listening_preferences WHERE user_id=?",
    )
    .bind(owner)
    .fetch_optional(&mut **tx)
    .await?;
    let (enabled, retention_days, revision) = row
        .map(|row| {
            (
                row.get("enabled"),
                row.get::<i64, _>("retention_days") as u16,
                row.get("revision"),
            )
        })
        .unwrap_or((false, DEFAULT_RETENTION, 0));
    let device_enabled: bool = sqlx::query_scalar("SELECT COALESCE((SELECT enabled FROM listening_devices WHERE user_id=? AND device_id=?),0)")
        .bind(owner).bind(device_id).fetch_one(&mut **tx).await?;
    Ok(ListeningPreferences {
        enabled,
        retention_days,
        revision,
        device_id: device_id.into(),
        device_enabled,
    })
}
async fn retention(tx: &mut Transaction<'_, Sqlite>, owner: &str) -> Result<u16> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE((SELECT retention_days FROM listening_preferences WHERE user_id=?),90)",
    )
    .bind(owner)
    .fetch_one(&mut **tx)
    .await? as u16)
}
async fn devices_in(tx: &mut Transaction<'_, Sqlite>, owner: &str) -> Result<ListeningDevices> {
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM listening_preferences WHERE user_id=?")
            .bind(owner)
            .fetch_one(&mut **tx)
            .await?;
    let rows = sqlx::query(
        "SELECT d.device_id,d.enabled,(SELECT COUNT(*) FROM listening_sessions s WHERE s.user_id=d.user_id AND s.device_id=d.device_id) session_count,(SELECT MAX(s.updated_at) FROM listening_sessions s WHERE s.user_id=d.user_id AND s.device_id=d.device_id) last_activity_at FROM listening_devices d WHERE d.user_id=? ORDER BY last_activity_at IS NULL,last_activity_at DESC,d.device_id ASC",
    )
    .bind(owner)
    .fetch_all(&mut **tx)
    .await?;
    let devices = rows
        .into_iter()
        .map(|row| {
            Ok(ListeningDevice {
                device_id: row.try_get("device_id")?,
                enabled: row.try_get("enabled")?,
                session_count: row.try_get("session_count")?,
                last_activity_at: row.try_get("last_activity_at")?,
            })
        })
        .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    Ok(ListeningDevices { revision, devices })
}

async fn prune_owner(tx: &mut Transaction<'_, Sqlite>, owner: &str, timestamp: i64) -> Result<u64> {
    Ok(sqlx::query("DELETE FROM listening_sessions WHERE user_id=? AND started_at < ? - COALESCE((SELECT retention_days FROM listening_preferences WHERE user_id=?),90)*86400")
        .bind(owner).bind(timestamp).bind(owner).execute(&mut **tx).await?.rows_affected())
}
fn uuid(value: &str) -> Result<()> {
    if uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value) {
        Ok(())
    } else {
        Err(invalid("A canonical UUID is required"))
    }
}
fn validate_progress(event: &ListeningProgress) -> Result<()> {
    uuid(&event.session_id)?;
    uuid(&event.track_id)?;
    if !(1..=i32::MAX as u32).contains(&event.sequence)
        || !(0..=DAY * 1_000).contains(&event.listened_ms)
        || !(0..=7 * DAY * 1_000).contains(&event.position_ms)
        || event.started_at < 0
    {
        return Err(invalid(
            "Listening progress is outside its supported bounds",
        ));
    }
    Ok(())
}
fn qualifies(listened_ms: i64, duration: i64) -> bool {
    duration > 0 && listened_ms >= 30_000.min(1_000.max(duration / 2)).min(duration)
}
fn parse_cursor(value: &str) -> Result<(i64, String)> {
    if value.len() > 80 {
        return Err(invalid("Invalid history cursor"));
    }
    let fields: Vec<_> = value.split(':').collect();
    if fields.len() != 3 || fields[0] != "v1" {
        return Err(invalid("Invalid history cursor"));
    }
    let timestamp = fields[1]
        .parse::<i64>()
        .map_err(|_| invalid("Invalid history cursor"))?;
    if timestamp < 0 || timestamp.to_string() != fields[1] || uuid(fields[2]).is_err() {
        return Err(invalid("Invalid history cursor"));
    }
    Ok((timestamp, fields[2].into()))
}
fn progress(row: &SqliteRow) -> ListeningProgress {
    ListeningProgress {
        session_id: row.get("session_id"),
        sequence: row.get::<i64, _>("sequence") as u32,
        track_id: row.get("track_id"),
        listened_ms: row.get("listened_ms"),
        position_ms: row.get("position_ms"),
        started_at: row.get("started_at"),
        ended: row.get("ended"),
    }
}
fn track(row: &SqliteRow) -> Track {
    Track {
        id: row.get("id"),
        title: row.get("title"),
        artist: row.get("artist"),
        album: row.get("album"),
        album_artist: row.get("album_artist"),
        genre: row.get("genre"),
        year: row.get("year"),
        track_number: row.get("track_number"),
        disc_number: row.get("disc_number"),
        duration_ms: row.get("duration_ms"),
        codec: row.get("codec"),
        bitrate: row.get("bitrate"),
        sample_rate: row.get("sample_rate"),
        bit_depth: row.get("bit_depth"),
        channels: row.get("channels"),
        size_bytes: row.get("size_bytes"),
        artwork_id: row.get("artwork_id"),
        added_at: row.get("added_at"),
        favorite: row.get("favorite"),
    }
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn conflict(message: &str) -> Error {
    Error::Conflict(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _directory: tempfile::TempDir,
        pool: SqlitePool,
        service: ListeningService,
        alice: User,
        bob: User,
        track: String,
        foreign_track: String,
        device: String,
    }
    impl Fixture {
        async fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let pool = crate::database::open(directory.path()).await.unwrap();
            let alice = user(&pool, "alice", true).await;
            let bob = user(&pool, "bob", false).await;
            let track = seed_track(&pool, &alice, 120_000).await;
            let foreign_track = seed_track(&pool, &bob, 120_000).await;
            Self {
                _directory: directory,
                service: ListeningService::new(pool.clone()),
                pool,
                alice,
                bob,
                track,
                foreign_track,
                device: uuid::Uuid::new_v4().to_string(),
            }
        }
        async fn enable(&self) -> ListeningPreferences {
            let current = self
                .service
                .preferences(&self.alice, &self.device)
                .await
                .unwrap();
            self.service
                .update_preferences(
                    &self.alice,
                    UpdateListeningPreferences {
                        revision: current.revision,
                        enabled: true,
                        retention_days: 90,
                        device_id: self.device.clone(),
                        device_enabled: true,
                    },
                )
                .await
                .unwrap()
        }
        fn event(&self, listened_ms: i64) -> ListeningProgress {
            ListeningProgress {
                session_id: uuid::Uuid::new_v4().to_string(),
                sequence: 1,
                track_id: self.track.clone(),
                listened_ms,
                position_ms: listened_ms,
                started_at: now() - 120,
                ended: false,
            }
        }
        fn batch(&self, revision: i64, events: Vec<ListeningProgress>) -> ListeningBatch {
            ListeningBatch {
                device_id: self.device.clone(),
                revision,
                events,
            }
        }
        async fn count(&self, user: &User) -> i64 {
            sqlx::query_scalar("SELECT COUNT(*) FROM listening_sessions WHERE user_id=?")
                .bind(&user.id)
                .fetch_one(&self.pool)
                .await
                .unwrap()
        }
    }
    async fn user(pool: &SqlitePool, name: &str, is_admin: bool) -> User {
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: name.into(),
            is_admin,
        };
        sqlx::query("INSERT INTO users(id,username,password_hash,is_admin) VALUES(?,?,?,?)")
            .bind(&user.id)
            .bind(name)
            .bind("unit-fixture-not-a-password")
            .bind(is_admin)
            .execute(pool)
            .await
            .unwrap();
        user
    }
    async fn seed_track(pool: &SqlitePool, owner: &User, duration_ms: i64) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let asset = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',100)")
            .bind(&asset).bind(&asset).bind("private/path/never-export").execute(pool).await.unwrap();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,duration_ms,codec) VALUES(?,?,?,'Fixture song','Fixture artist','Fixture album',?,'FLAC')")
            .bind(&id).bind(&owner.id).bind(asset).bind(duration_ms).execute(pool).await.unwrap();
        id
    }

    #[tokio::test]
    async fn consent_is_off_by_default_and_per_device_even_for_administrators() {
        let fixture = Fixture::new().await;
        let prefs = fixture
            .service
            .preferences(&fixture.alice, &fixture.device)
            .await
            .unwrap();
        assert!(!prefs.enabled && !prefs.device_enabled);
        assert_eq!((prefs.retention_days, prefs.revision), (90, 0));
        assert!(matches!(
            fixture
                .service
                .record(
                    &fixture.alice,
                    fixture.batch(0, vec![fixture.event(30_000)])
                )
                .await,
            Err(Error::Forbidden)
        ));
        let prefs = fixture.enable().await;
        let unconsented = ListeningBatch {
            device_id: uuid::Uuid::new_v4().to_string(),
            ..fixture.batch(prefs.revision, vec![fixture.event(30_000)])
        };
        assert!(matches!(
            fixture.service.record(&fixture.alice, unconsented).await,
            Err(Error::Forbidden)
        ));
        assert_eq!(fixture.count(&fixture.alice).await, 0);
    }

    #[tokio::test]
    async fn cumulative_progress_retries_and_sequences_do_not_double_count() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let event = fixture.event(15_000);
        let first = fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![event.clone()]),
            )
            .await
            .unwrap();
        assert_eq!((first.accepted, first.duplicates), (1, 0));
        assert_eq!(
            fixture
                .service
                .history(&fixture.alice, &ListeningHistoryQuery::default())
                .await
                .unwrap()
                .total_listens,
            0
        );
        let duplicate = fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![event.clone()]),
            )
            .await
            .unwrap();
        assert_eq!((duplicate.accepted, duplicate.duplicates), (0, 1));
        let mut second = event.clone();
        second.sequence = 2;
        second.listened_ms = 30_000;
        second.position_ms = 5_000; // A seek changes position, never cumulative listening.
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![second.clone()]),
            )
            .await
            .unwrap();
        let late = fixture
            .service
            .record(&fixture.alice, fixture.batch(prefs.revision, vec![event]))
            .await
            .unwrap();
        assert_eq!(late.duplicates, 1);
        let history = fixture
            .service
            .history(&fixture.alice, &ListeningHistoryQuery::default())
            .await
            .unwrap();
        assert_eq!(
            (history.total_listens, history.total_listened_ms),
            (1, 30_000)
        );
        assert_eq!(history.items[0].position_ms, 5_000);
        second.position_ms += 1;
        assert!(matches!(
            fixture
                .service
                .record(&fixture.alice, fixture.batch(prefs.revision, vec![second]))
                .await,
            Err(Error::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn decreasing_progress_and_ended_reopening_are_rejected() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let first = fixture.event(30_000);
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![first.clone()]),
            )
            .await
            .unwrap();
        let mut decreased = first.clone();
        decreased.sequence = 2;
        decreased.listened_ms = 29_999;
        assert!(matches!(
            fixture
                .service
                .record(
                    &fixture.alice,
                    fixture.batch(prefs.revision, vec![decreased])
                )
                .await,
            Err(Error::Conflict(_))
        ));
        let mut ended = first;
        ended.sequence = 2;
        ended.ended = true;
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![ended.clone()]),
            )
            .await
            .unwrap();
        assert_eq!(
            fixture
                .service
                .record(
                    &fixture.alice,
                    fixture.batch(prefs.revision, vec![ended.clone()])
                )
                .await
                .unwrap()
                .duplicates,
            1
        );
        ended.sequence = 3;
        ended.ended = false;
        assert!(matches!(
            fixture
                .service
                .record(&fixture.alice, fixture.batch(prefs.revision, vec![ended]))
                .await,
            Err(Error::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn foreign_track_denial_rolls_back_entire_batch_and_admin_has_no_bypass() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let valid = fixture.event(30_000);
        let foreign = ListeningProgress {
            track_id: fixture.foreign_track.clone(),
            ..fixture.event(30_000)
        };
        assert!(matches!(
            fixture
                .service
                .record(
                    &fixture.alice,
                    fixture.batch(prefs.revision, vec![valid, foreign])
                )
                .await,
            Err(Error::NotFound)
        ));
        assert_eq!(fixture.count(&fixture.alice).await, 0);
        let bob_prefs = fixture
            .service
            .update_preferences(
                &fixture.bob,
                UpdateListeningPreferences {
                    revision: 0,
                    enabled: true,
                    retention_days: 90,
                    device_id: fixture.device.clone(),
                    device_enabled: true,
                },
            )
            .await
            .unwrap();
        let bob_event = ListeningProgress {
            track_id: fixture.foreign_track.clone(),
            ..fixture.event(30_000)
        };
        fixture
            .service
            .record(
                &fixture.bob,
                fixture.batch(bob_prefs.revision, vec![bob_event]),
            )
            .await
            .unwrap();
        assert_eq!(
            fixture
                .service
                .history(&fixture.alice, &ListeningHistoryQuery::default())
                .await
                .unwrap()
                .total_listens,
            0
        );
        assert_eq!(
            fixture
                .service
                .export(&fixture.alice)
                .await
                .unwrap()
                .events
                .len(),
            0
        );
        assert_eq!(
            fixture
                .service
                .history(&fixture.bob, &ListeningHistoryQuery::default())
                .await
                .unwrap()
                .total_listens,
            1
        );
    }

    #[tokio::test]
    async fn privacy_cas_is_atomic_and_clear_invalidates_inflight_updates() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let input = UpdateListeningPreferences {
            revision: prefs.revision,
            enabled: true,
            retention_days: 30,
            device_id: fixture.device.clone(),
            device_enabled: true,
        };
        let (first, second) = tokio::join!(
            fixture
                .service
                .update_preferences(&fixture.alice, input.clone()),
            fixture.service.update_preferences(&fixture.alice, input)
        );
        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        assert!(matches!(
            first.as_ref().err().or(second.as_ref().err()),
            Some(Error::Conflict(_))
        ));
        let prefs = fixture
            .service
            .preferences(&fixture.alice, &fixture.device)
            .await
            .unwrap();
        let event = fixture.event(30_000);
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![event.clone()]),
            )
            .await
            .unwrap();
        let cleared = fixture
            .service
            .clear(&fixture.alice, &fixture.device)
            .await
            .unwrap();
        assert!(!cleared.enabled && !cleared.device_enabled);
        assert!(cleared.revision > prefs.revision);
        assert_eq!(fixture.count(&fixture.alice).await, 0);
        let reenabled = fixture.enable().await;
        assert!(reenabled.revision > cleared.revision);
        assert!(matches!(
            fixture
                .service
                .record(&fixture.alice, fixture.batch(prefs.revision, vec![event]))
                .await,
            Err(Error::Conflict(_))
        ));
        assert_eq!(fixture.count(&fixture.alice).await, 0);
    }

    #[tokio::test]
    async fn short_tracks_qualify_at_half_duration_and_partial_time_is_exported_privately() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let short = seed_track(&fixture.pool, &fixture.alice, 8_000).await;
        let first = ListeningProgress {
            track_id: short,
            ..fixture.event(3_999)
        };
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![first.clone()]),
            )
            .await
            .unwrap();
        assert_eq!(
            fixture
                .service
                .history(&fixture.alice, &ListeningHistoryQuery::default())
                .await
                .unwrap()
                .total_listens,
            0
        );
        let exported = fixture.service.export(&fixture.alice).await.unwrap();
        assert_eq!(exported.events.len(), 1);
        assert!(!exported.events[0].qualified);
        let serialized = serde_json::to_string(&exported).unwrap();
        assert!(
            !serialized.contains("never-export")
                && !serialized.contains("password")
                && !serialized.contains("csrf")
        );
        let next = ListeningProgress {
            sequence: 2,
            listened_ms: 4_000,
            ..first
        };
        fixture
            .service
            .record(&fixture.alice, fixture.batch(prefs.revision, vec![next]))
            .await
            .unwrap();
        assert_eq!(
            fixture
                .service
                .history(&fixture.alice, &ListeningHistoryQuery::default())
                .await
                .unwrap()
                .total_listens,
            1
        );
        assert!(qualifies(500, 500));
        assert!(!qualifies(499, 500));
        assert!(!qualifies(30_000, 0));
    }

    #[tokio::test]
    async fn history_cursor_is_stable_for_timestamp_ties_and_retains_actual_track_metadata() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        sqlx::query("INSERT INTO favorites(user_id,track_id) VALUES(?,?)")
            .bind(&fixture.alice.id)
            .bind(&fixture.track)
            .execute(&fixture.pool)
            .await
            .unwrap();
        let events = (0..5).map(|_| fixture.event(30_000)).collect();
        fixture
            .service
            .record(&fixture.alice, fixture.batch(prefs.revision, events))
            .await
            .unwrap();
        sqlx::query("UPDATE listening_sessions SET updated_at=? WHERE user_id=?")
            .bind(now())
            .bind(&fixture.alice.id)
            .execute(&fixture.pool)
            .await
            .unwrap();
        let mut cursor = None;
        let mut ids = Vec::new();
        loop {
            let page = fixture
                .service
                .history(
                    &fixture.alice,
                    &ListeningHistoryQuery {
                        cursor,
                        limit: Some(2),
                    },
                )
                .await
                .unwrap();
            assert_eq!((page.total_listens, page.total_listened_ms), (5, 150_000));
            assert!(page.items.iter().all(|item| item.track.id == fixture.track
                && item.track.favorite
                && item.track.size_bytes == 100));
            ids.extend(page.items.into_iter().map(|item| item.session_id));
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(ids.len(), 5);
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 5);
        assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
        for value in [
            "",
            "v2:0:bad",
            "v1:-1:bad",
            "v1:01:00000000-0000-0000-0000-000000000000",
            "v1:0:ABCDEF00-0000-0000-0000-000000000000",
        ] {
            assert!(matches!(
                fixture
                    .service
                    .history(
                        &fixture.alice,
                        &ListeningHistoryQuery {
                            cursor: Some(value.into()),
                            limit: None
                        }
                    )
                    .await,
                Err(Error::Invalid(_))
            ));
        }
    }

    #[tokio::test]
    async fn retention_reduction_prunes_rows_and_background_pruning_is_owner_correct() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let ancient = ListeningProgress {
            started_at: now() - 8 * DAY,
            ..fixture.event(30_000)
        };
        let recent = fixture.event(30_000);
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![ancient, recent]),
            )
            .await
            .unwrap();
        let next = fixture
            .service
            .update_preferences(
                &fixture.alice,
                UpdateListeningPreferences {
                    revision: prefs.revision,
                    enabled: false,
                    retention_days: 7,
                    device_id: fixture.device.clone(),
                    device_enabled: false,
                },
            )
            .await
            .unwrap();
        assert!(!next.enabled);
        assert_eq!(fixture.count(&fixture.alice).await, 1);
        sqlx::query("UPDATE listening_sessions SET started_at=? WHERE user_id=?")
            .bind(now() - 8 * DAY)
            .bind(&fixture.alice.id)
            .execute(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(fixture.service.prune().await.unwrap(), 1);
        assert_eq!(fixture.count(&fixture.alice).await, 0);
    }

    #[tokio::test]
    async fn validation_rejects_bad_ids_time_ranges_payloads_and_changed_session_identity() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        for invalid in [
            ListeningProgress {
                session_id: "bad".into(),
                ..fixture.event(1_000)
            },
            ListeningProgress {
                sequence: 0,
                ..fixture.event(1_000)
            },
            ListeningProgress {
                sequence: u32::MAX,
                ..fixture.event(1_000)
            },
            ListeningProgress {
                listened_ms: -1,
                ..fixture.event(1_000)
            },
            ListeningProgress {
                position_ms: 7 * DAY * 1_000 + 1,
                ..fixture.event(1_000)
            },
            ListeningProgress {
                started_at: now() + 6,
                ..fixture.event(1_000)
            },
            ListeningProgress {
                started_at: now() - 91 * DAY,
                ..fixture.event(1_000)
            },
            ListeningProgress {
                started_at: now(),
                ..fixture.event(20_000)
            },
        ] {
            assert!(matches!(
                fixture
                    .service
                    .record(&fixture.alice, fixture.batch(prefs.revision, vec![invalid]))
                    .await,
                Err(Error::Invalid(_))
            ));
        }
        assert!(matches!(
            fixture
                .service
                .record(&fixture.alice, fixture.batch(prefs.revision, vec![]))
                .await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            fixture
                .service
                .record(
                    &fixture.alice,
                    fixture.batch(
                        prefs.revision,
                        (0..21).map(|_| fixture.event(1_000)).collect()
                    )
                )
                .await,
            Err(Error::Invalid(_))
        ));
        let first = fixture.event(30_000);
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![first.clone()]),
            )
            .await
            .unwrap();
        let changed = ListeningProgress {
            sequence: 2,
            started_at: first.started_at - 1,
            ..first
        };
        assert!(matches!(
            fixture
                .service
                .record(&fixture.alice, fixture.batch(prefs.revision, vec![changed]))
                .await,
            Err(Error::Conflict(_))
        ));
        assert_eq!(fixture.count(&fixture.alice).await, 1);
    }

    #[tokio::test]
    async fn device_and_history_quotas_are_bounded_without_discarding_retained_history() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        for _ in 1..MAX_DEVICES {
            sqlx::query("INSERT INTO listening_devices(user_id,device_id,enabled) VALUES(?,?,0)")
                .bind(&fixture.alice.id)
                .bind(uuid::Uuid::new_v4().to_string())
                .execute(&fixture.pool)
                .await
                .unwrap();
        }
        assert!(matches!(
            fixture
                .service
                .update_preferences(
                    &fixture.alice,
                    UpdateListeningPreferences {
                        revision: prefs.revision,
                        enabled: true,
                        retention_days: 90,
                        device_id: uuid::Uuid::new_v4().to_string(),
                        device_enabled: true,
                    }
                )
                .await,
            Err(Error::Invalid(_))
        ));
        let mut tx = fixture.pool.begin().await.unwrap();
        for _ in 0..MAX_SESSIONS {
            sqlx::query("INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES(?,?,?,?,1,1000,1000,?,?,0,0)")
                .bind(&fixture.alice.id).bind(uuid::Uuid::new_v4().to_string()).bind(&fixture.device).bind(&fixture.track)
                .bind(now() - 120).bind(now()).execute(&mut *tx).await.unwrap();
        }
        tx.commit().await.unwrap();
        assert!(matches!(
            fixture
                .service
                .record(
                    &fixture.alice,
                    fixture.batch(prefs.revision, vec![fixture.event(30_000)])
                )
                .await,
            Err(Error::Invalid(_))
        ));
        assert_eq!(fixture.count(&fixture.alice).await, MAX_SESSIONS);
        assert_eq!(
            fixture
                .service
                .export(&fixture.alice)
                .await
                .unwrap()
                .events
                .len(),
            MAX_SESSIONS as usize
        );
    }

    #[tokio::test]
    async fn database_foreign_keys_prevent_cross_owner_rows_and_track_deletion_cascades() {
        let fixture = Fixture::new().await;
        let prefs = fixture.enable().await;
        let direct = sqlx::query("INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES(?,?,?,?,1,0,0,?,?,0,0)")
            .bind(&fixture.alice.id).bind(uuid::Uuid::new_v4().to_string()).bind(&fixture.device).bind(&fixture.foreign_track)
            .bind(now()).bind(now()).execute(&fixture.pool).await;
        assert!(direct.is_err());
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(prefs.revision, vec![fixture.event(30_000)]),
            )
            .await
            .unwrap();
        sqlx::query("DELETE FROM tracks WHERE id=?")
            .bind(&fixture.track)
            .execute(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(fixture.count(&fixture.alice).await, 0);
    }

    #[tokio::test]
    async fn erasure_and_identical_client_session_ids_are_scoped_to_the_authenticated_owner() {
        let fixture = Fixture::new().await;
        let alice = fixture.enable().await;
        let bob = fixture
            .service
            .update_preferences(
                &fixture.bob,
                UpdateListeningPreferences {
                    revision: 0,
                    enabled: true,
                    retention_days: 90,
                    device_id: fixture.device.clone(),
                    device_enabled: true,
                },
            )
            .await
            .unwrap();
        let alice_event = fixture.event(30_000);
        let bob_event = ListeningProgress {
            track_id: fixture.foreign_track.clone(),
            ..alice_event.clone()
        };
        fixture
            .service
            .record(
                &fixture.alice,
                fixture.batch(alice.revision, vec![alice_event]),
            )
            .await
            .unwrap();
        fixture
            .service
            .record(&fixture.bob, fixture.batch(bob.revision, vec![bob_event]))
            .await
            .unwrap();
        fixture
            .service
            .clear(&fixture.alice, &fixture.device)
            .await
            .unwrap();
        assert_eq!(fixture.count(&fixture.alice).await, 0);
        assert_eq!(fixture.count(&fixture.bob).await, 1);
        let bob_export = fixture.service.export(&fixture.bob).await.unwrap();
        assert!(bob_export.enabled);
        assert_eq!(
            bob_export.events[0].progress.track_id,
            fixture.foreign_track
        );
        assert!(
            fixture
                .service
                .preferences(&fixture.bob, &fixture.device)
                .await
                .unwrap()
                .device_enabled
        );
    }
}
