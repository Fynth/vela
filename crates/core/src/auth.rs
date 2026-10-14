use crate::{Error, Result, now};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use rand::RngCore;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use tokio::sync::Semaphore;
use vela_domain::{AuthResponse, Credentials, User};

pub const SESSION_SECONDS: i64 = 7 * 24 * 60 * 60;
#[derive(Clone)]
pub struct AuthService {
    pool: SqlitePool,
    hashing: Arc<Semaphore>,
    dummy_hash: Arc<String>,
}
#[derive(Clone, Debug)]
pub struct Session {
    pub response: AuthResponse,
    pub token_hash: String,
}
pub struct Login {
    pub response: AuthResponse,
    pub token: String,
}

pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
pub fn token_hash(token: &str) -> String {
    blake3::hash(token.as_bytes()).to_hex().to_string()
}
fn user_from_row(row: &sqlx::sqlite::SqliteRow) -> User {
    User {
        id: row.get("id"),
        username: row.get("username"),
        is_admin: row.get("is_admin"),
    }
}
fn hash_password(password: &str) -> anyhow::Result<String> {
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|e| anyhow::anyhow!("password hash failed: {e}"))?
        .to_string())
}
fn validate_credentials(c: &Credentials) -> Result<()> {
    if c.username.len() < 3
        || c.username.len() > 32
        || !c
            .username
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Err(Error::Invalid(
            "Username must be 3–32 ASCII letters, numbers, dots, hyphens or underscores".into(),
        ));
    }
    if !(12..=128).contains(&c.password.len()) {
        return Err(Error::Invalid("Password must be 12–128 bytes".into()));
    }
    Ok(())
}
impl AuthService {
    pub async fn new(pool: SqlitePool) -> anyhow::Result<Self> {
        let dummy_hash = tokio::task::spawn_blocking(|| hash_password(&random_token())).await??;
        Ok(Self {
            pool,
            hashing: Arc::new(Semaphore::new(2)),
            dummy_hash: Arc::new(dummy_hash),
        })
    }
    pub async fn needs_setup(&self) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?
            == 0)
    }
    async fn password_hash(&self, password: String) -> Result<String> {
        let permit = self
            .hashing
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::Internal(e.into()))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            hash_password(&password)
        })
        .await
        .map_err(|e| Error::Internal(e.into()))?
        .map_err(Error::Internal)
    }
    pub async fn setup(&self, credentials: Credentials) -> Result<Login> {
        validate_credentials(&credentials)?;
        if !self.needs_setup().await? {
            return Err(Error::Conflict("Setup has already finished".into()));
        }
        let hash = self.password_hash(credentials.password).await?;
        let id = uuid::Uuid::new_v4().to_string();
        let row=sqlx::query("INSERT INTO users(id,username,password_hash,is_admin) SELECT ?,?,?,1 WHERE NOT EXISTS(SELECT 1 FROM users) RETURNING id,username,is_admin")
            .bind(&id).bind(&credentials.username).bind(hash).fetch_optional(&self.pool).await?;
        let user = row
            .as_ref()
            .map(user_from_row)
            .ok_or_else(|| Error::Conflict("Setup has already finished".into()))?;
        self.audit(&user.id, "account.setup", Some(&user.id))
            .await?;
        self.new_session(user).await
    }
    pub async fn create_user(&self, actor: &User, credentials: Credentials) -> Result<User> {
        if !actor.is_admin {
            return Err(Error::Forbidden);
        }
        validate_credentials(&credentials)?;
        let hash = self.password_hash(credentials.password).await?;
        let id = uuid::Uuid::new_v4().to_string();
        let result=sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,?) RETURNING id,username,is_admin").bind(&id).bind(credentials.username).bind(hash).fetch_one(&self.pool).await;
        let row = result.map_err(|e| {
            if e.as_database_error()
                .is_some_and(|e| e.is_unique_violation())
            {
                Error::Conflict("Username already exists".into())
            } else {
                e.into()
            }
        })?;
        self.audit(&actor.id, "account.create", Some(&id)).await?;
        Ok(user_from_row(&row))
    }
    pub async fn login(&self, credentials: Credentials) -> Result<Login> {
        if credentials.username.len() > 32 || credentials.password.len() > 128 {
            return Err(Error::Unauthorized);
        }
        let row = sqlx::query(
            "SELECT id,username,is_admin,password_hash FROM users WHERE username=? COLLATE NOCASE",
        )
        .bind(credentials.username)
        .fetch_optional(&self.pool)
        .await?;
        let hash = row
            .as_ref()
            .map(|r| r.get::<String, _>("password_hash"))
            .unwrap_or_else(|| self.dummy_hash.as_ref().clone());
        let permit = self
            .hashing
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::Internal(e.into()))?;
        let ok = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            PasswordHash::new(&hash).is_ok_and(|hash| {
                Argon2::default()
                    .verify_password(credentials.password.as_bytes(), &hash)
                    .is_ok()
            })
        })
        .await
        .map_err(|e| Error::Internal(e.into()))?;
        if !ok {
            return Err(Error::Unauthorized);
        }
        let user = row.as_ref().map(user_from_row).ok_or(Error::Unauthorized)?;
        self.new_session(user).await
    }
    async fn new_session(&self, user: User) -> Result<Login> {
        let token = random_token();
        let csrf_token = random_token();
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM sessions WHERE expires_at<=?")
            .bind(now())
            .execute(&mut *tx)
            .await?;
        // Bound active sessions per account; newest sessions remain valid.
        sqlx::query("DELETE FROM sessions WHERE token_hash IN (SELECT token_hash FROM sessions WHERE user_id=? ORDER BY created_at DESC,token_hash LIMIT -1 OFFSET 19)").bind(&user.id).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO sessions(token_hash,user_id,csrf_token,expires_at) VALUES(?,?,?,?)",
        )
        .bind(token_hash(&token))
        .bind(&user.id)
        .bind(&csrf_token)
        .bind(now() + SESSION_SECONDS)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Login {
            response: AuthResponse { user, csrf_token },
            token,
        })
    }
    pub async fn session(&self, token: &str) -> Result<Session> {
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::Unauthorized);
        }
        let hash = token_hash(token);
        let row=sqlx::query("SELECT u.id,u.username,u.is_admin,s.csrf_token FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=? AND s.expires_at>?").bind(&hash).bind(now()).fetch_optional(&self.pool).await?.ok_or(Error::Unauthorized)?;
        Ok(Session {
            response: AuthResponse {
                user: user_from_row(&row),
                csrf_token: row.get("csrf_token"),
            },
            token_hash: hash,
        })
    }
    pub async fn logout(&self, session: &Session) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE token_hash=?")
            .bind(&session.token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn audit(&self, actor: &str, action: &str, object: Option<&str>) -> Result<()> {
        sqlx::query("INSERT INTO audit_events(actor_id,action,object_id) VALUES(?,?,?)")
            .bind(actor)
            .bind(action)
            .bind(object)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

impl std::fmt::Debug for Login {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Login")
            .field("response", &self.response)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn setup_is_atomic_sessions_revocable_and_credentials_private() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::database::open(dir.path()).await.unwrap();
        let auth = AuthService::new(pool.clone()).await.unwrap();
        let creds = Credentials {
            username: "admin".into(),
            password: "a valid test password".into(),
        };
        let (a, b) = tokio::join!(
            auth.setup(creds.clone()),
            auth.setup(Credentials {
                username: "other".into(),
                ..creds.clone()
            })
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        let login = a.or(b).unwrap();
        let session = auth.session(&login.token).await.unwrap();
        let diagnostics = format!("{login:?} {session:?} {creds:?}");
        assert!(!diagnostics.contains(&login.token));
        assert!(!diagnostics.contains(&session.response.csrf_token));
        assert!(!diagnostics.contains(&creds.password));
        assert!(session.response.user.is_admin);
        let stored: String = sqlx::query_scalar("SELECT password_hash FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(stored.starts_with("$argon2id$"));
        assert!(!stored.contains(&creds.password));
        auth.logout(&session).await.unwrap();
        assert!(auth.session(&login.token).await.is_err());
        assert!(
            auth.login(Credentials {
                username: "missing".into(),
                password: "wrong".into()
            })
            .await
            .is_err()
        );
    }
}
