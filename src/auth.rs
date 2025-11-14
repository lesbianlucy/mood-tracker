#![allow(dead_code)]

use anyhow::anyhow;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;
use sqlx::{sqlite::SqliteQueryResult, Row};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{error::AppError, models::user::UserRole, state::AppState};

pub const SESSION_COOKIE: &str = "kawaii_session";
const MIN_PASSWORD_LENGTH: usize = 8;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Default)]
pub struct CurrentUser(pub Option<AuthenticatedUser>);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            return Ok(Self(Some(user.clone())));
        }

        let state = match parts.extensions.get::<AppState>() {
            Some(state) => state.clone(),
            None => return Ok(Self(None)),
        };

        let jar = CookieJar::from_headers(&parts.headers);
        let Some(session_cookie) = jar.get(SESSION_COOKIE) else {
            return Ok(Self(None));
        };

        match load_user_from_session(&state, session_cookie.value()).await? {
            Some(user) => {
                parts.extensions.insert(user.clone());
                Ok(Self(Some(user)))
            }
            None => Ok(Self(None)),
        }
    }
}

impl CurrentUser {
    pub fn require_user(&self) -> Result<&AuthenticatedUser, AppError> {
        self.0.as_ref().ok_or(AppError::Unauthorized)
    }

    pub fn require_admin(&self) -> Result<&AuthenticatedUser, AppError> {
        let user = self.require_user()?;
        if user.role == UserRole::Admin {
            Ok(user)
        } else {
            Err(AppError::Forbidden)
        }
    }
}

pub fn unauthorized() -> AppError {
    AppError::Unauthorized
}

pub fn forbidden() -> AppError {
    AppError::Forbidden
}

pub async fn register_user(
    state: &AppState,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthenticatedUser, AppError> {
    let username = username.trim();
    let email = email.trim();

    if username.is_empty() || email.is_empty() {
        return Err(AppError::BadRequest(
            "Bitte Nutzername und E-Mail ausfüllen.".into(),
        ));
    }

    if !email.contains('@') {
        return Err(AppError::BadRequest(
            "Bitte eine gültige E-Mail-Adresse eingeben.".into(),
        ));
    }

    validate_password(password)?;

    let password_hash = hash_password(password)?;
    let uuid = Uuid::new_v4().to_string();
    let now = Utc::now();

    let insert_result = sqlx::query(
        r#"
        INSERT INTO users (uuid, username, email, password_hash, role, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
    )
    .bind(&uuid)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(UserRole::User.as_str())
    .bind(now)
    .execute(&state.db)
    .await;

    let insert_result: SqliteQueryResult = match insert_result {
        Ok(result) => result,
        Err(err) => {
            return Err(match_unique_error(err));
        }
    };

    let id = insert_result.last_insert_rowid();

    let _ = state
        .storage
        .ensure_user_scaffold(&uuid, username)
        .await
        .map_err(|err| {
            warn!(%uuid, %username, "Konnte Benutzerverzeichnis nicht vorbereiten: {err}");
            err
        })?;

    if let Err(err) = state
        .git
        .commit_ai_changes(&format!("feat: neuer Account für {username} 💖"))
    {
        warn!(%username, "Git Commit nach Registrierung fehlgeschlagen: {err}");
    }

    info!(%username, %uuid, "Neuer Benutzer registriert");

    Ok(AuthenticatedUser {
        id,
        uuid,
        username: username.to_string(),
        role: UserRole::User,
    })
}

pub async fn authenticate_user(
    state: &AppState,
    identifier: &str,
    password: &str,
) -> Result<AuthenticatedUser, AppError> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(AppError::BadRequest(
            "Bitte Nutzername oder E-Mail eingeben.".into(),
        ));
    }

    let row = sqlx::query(
        r#"
        SELECT id, uuid, username, role, password_hash
        FROM users
        WHERE username = ?1 OR email = ?1
        "#,
    )
    .bind(identifier)
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Err(AppError::Unauthorized);
    };

    let password_hash: String = row.try_get("password_hash")?;

    if !verify_password(&password_hash, password)? {
        return Err(AppError::Unauthorized);
    }

    let id: i64 = row.try_get("id")?;
    let uuid: String = row.try_get("uuid")?;
    let username: String = row.try_get("username")?;
    let role = parse_role(row.try_get::<String, _>("role")?.as_str());

    sqlx::query("UPDATE users SET last_login_at = ?1 WHERE id = ?2")
        .bind(Utc::now())
        .bind(id)
        .execute(&state.db)
        .await?;

    info!(user_id = id, %username, "Login erfolgreich");

    Ok(AuthenticatedUser {
        id,
        uuid,
        username,
        role,
    })
}

pub async fn create_session(state: &AppState, user_id: i64) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, created_at, last_seen_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
    )
    .bind(&session_id)
    .bind(user_id)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;
    Ok(session_id)
}

pub async fn destroy_session(state: &AppState, session_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE id = ?1")
        .bind(session_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub fn apply_session_cookie(jar: CookieJar, session_id: &str) -> CookieJar {
    let cookie = Cookie::build((SESSION_COOKIE, session_id.to_owned()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    jar.add(cookie)
}

pub fn clear_session_cookie(jar: CookieJar) -> CookieJar {
    let removal = Cookie::build((SESSION_COOKIE, "")).path("/").build();
    jar.remove(removal)
}

async fn load_user_from_session(
    state: &AppState,
    session_id: &str,
) -> Result<Option<AuthenticatedUser>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.uuid, users.username, users.role
        FROM sessions
        JOIN users ON users.id = sessions.user_id
        WHERE sessions.id = ?1
        "#,
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    sqlx::query("UPDATE sessions SET last_seen_at = ?1 WHERE id = ?2")
        .bind(Utc::now())
        .bind(session_id)
        .execute(&state.db)
        .await?;

    Ok(Some(AuthenticatedUser {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        username: row.try_get("username")?,
        role: parse_role(row.try_get::<String, _>("role")?.as_str()),
    }))
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| AppError::Other(anyhow!(err.to_string())))?
        .to_string();
    Ok(hash)
}

fn verify_password(hash: &str, password: &str) -> Result<bool, AppError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|err| AppError::Other(anyhow!(err.to_string())))?;
    let argon = Argon2::default();
    Ok(argon
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn parse_role(role: &str) -> UserRole {
    match role {
        "admin" => UserRole::Admin,
        _ => UserRole::User,
    }
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::BadRequest(format!(
            "Passwort muss mindestens {MIN_PASSWORD_LENGTH} Zeichen lang sein."
        )));
    }
    Ok(())
}

fn match_unique_error(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err {
        let code = db_err.code().map(|c| c.to_string());
        if matches_unique_constraint(code.as_deref(), db_err.message()) {
            return AppError::BadRequest("Nutzername oder E-Mail bereits vergeben.".into());
        }
    }
    err.into()
}

fn matches_unique_constraint(code: Option<&str>, message: &str) -> bool {
    if let Some(code) = code {
        if code == "2067" {
            return true;
        }
    }
    if message.contains("UNIQUE constraint failed") {
        return true;
    }
    if message.contains("UNIQUE") {
        return true;
    }
    false
}
