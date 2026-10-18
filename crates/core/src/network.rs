//! Explicit server opt-in. No change of server settings publishes a music object.
use crate::{Error, Result};
use sqlx::{Row, SqlitePool};
use vela_domain::{NetworkSettings, UpdateNetworkSettings, User, canonical_public_origin};
#[derive(Clone)]
pub struct NetworkService {
    pool: SqlitePool,
}
impl NetworkService {
    pub async fn initialize(pool: SqlitePool) -> Result<Self> {
        sqlx::query("INSERT INTO network_settings(id,server_id,enabled,origin,name,revision) VALUES(1,?,0,NULL,'Vela Music',0) ON CONFLICT(id) DO NOTHING").bind(uuid::Uuid::new_v4().to_string()).execute(&pool).await?;
        Ok(Self { pool })
    }
    pub async fn settings(&self) -> Result<NetworkSettings> {
        let row = sqlx::query(
            "SELECT server_id,enabled,origin,name,revision FROM network_settings WHERE id=1",
        )
        .fetch_one(&self.pool)
        .await?;
        settings(row)
    }
    pub async fn update(
        &self,
        user: &User,
        input: UpdateNetworkSettings,
    ) -> Result<NetworkSettings> {
        if !user.is_admin {
            return Err(Error::Forbidden);
        }
        let name = input.name.trim();
        if name.is_empty()
            || name.len() > 80
            || name.chars().any(char::is_control)
            || input.revision < 0
        {
            return Err(Error::Invalid("Invalid public server settings".into()));
        }
        let origin = if input.origin.is_empty() && !input.enabled {
            None
        } else {
            Some(canonical_public_origin(&input.origin).map_err(|e| Error::Invalid(e.into()))?)
        };
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let changed=sqlx::query("UPDATE network_settings SET enabled=?,origin=?,name=?,revision=revision+1 WHERE id=1 AND revision=? AND revision<9223372036854775807").bind(input.enabled).bind(origin).bind(name).bind(input.revision).execute(&mut *tx).await?.rows_affected();
        if changed != 1 {
            return Err(Error::Conflict(
                "Public server settings changed; refresh before saving".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT server_id,enabled,origin,name,revision FROM network_settings WHERE id=1",
        )
        .fetch_one(&mut *tx)
        .await?;
        let result = settings(row)?;
        tx.commit().await?;
        Ok(result)
    }
}
fn settings(row: sqlx::sqlite::SqliteRow) -> Result<NetworkSettings> {
    Ok(NetworkSettings {
        server_id: row.try_get("server_id")?,
        enabled: row.try_get("enabled")?,
        origin: row.try_get("origin")?,
        name: row.try_get("name")?,
        revision: row.try_get("revision")?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn optin_is_admin_cas_and_never_changes_existing_private_data() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::database::open(dir.path()).await.unwrap();
        let service = NetworkService::initialize(pool.clone()).await.unwrap();
        let first = service.settings().await.unwrap();
        assert!(!first.enabled);
        assert!(first.origin.is_none());
        assert!(uuid::Uuid::parse_str(&first.server_id).is_ok());
        let member = User {
            id: "owner".into(),
            username: "owner".into(),
            is_admin: false,
        };
        let admin = User {
            is_admin: true,
            ..member.clone()
        };
        let input = UpdateNetworkSettings {
            revision: 0,
            enabled: true,
            origin: "https://music.example.org".into(),
            name: "My music".into(),
        };
        assert!(matches!(
            service.update(&member, input.clone()).await,
            Err(Error::Forbidden)
        ));
        let enabled = service.update(&admin, input.clone()).await.unwrap();
        assert!(enabled.enabled);
        assert_eq!(enabled.revision, 1);
        assert_eq!(enabled.server_id, first.server_id);
        assert!(matches!(
            service.update(&admin, input).await,
            Err(Error::Conflict(_))
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM publications")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            NetworkService::initialize(pool.clone())
                .await
                .unwrap()
                .settings()
                .await
                .unwrap(),
            enabled
        );
        let off = service
            .update(
                &admin,
                UpdateNetworkSettings {
                    revision: 1,
                    enabled: false,
                    origin: String::new(),
                    name: "Private music".into(),
                },
            )
            .await
            .unwrap();
        assert!(!off.enabled);
        assert!(off.origin.is_none());
    }
}
