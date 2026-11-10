use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use subtle::ConstantTimeEq;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{ApiError, AppState};

const SESSION_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone)]
pub(crate) struct Actor {
    pub(crate) user_id: String,
    pub(crate) username: String,
    pub(crate) role: String,
    pub(crate) session_id: Option<String>,
}

impl Actor {
    pub(crate) fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub(crate) struct UserSummary {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) created_at: String,
}

#[derive(Debug, FromRow)]
struct UserCredential {
    id: String,
    username: String,
    display_name: String,
    password_hash: String,
    role: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginResponse {
    access_token: String,
    token_type: &'static str,
    expires_at: i64,
    user: UserSummary,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateUserRequest {
    username: String,
    display_name: Option<String>,
    password: String,
    #[serde(default = "default_role")]
    role: String,
}

fn default_role() -> String {
    "member".to_owned()
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i64::MAX as u64) as i64
}

fn valid_username(value: &str) -> bool {
    (3..=39).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

async fn hash_password(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow::anyhow!("password hashing failed: {error}"))
    })
    .await?
}

async fn verify_password(hash: String, password: String) -> bool {
    tokio::task::spawn_blocking(move || {
        let Ok(parsed_hash) = PasswordHash::new(&hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

pub(crate) async fn record_audit(
    pool: &SqlitePool,
    actor_user_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
) {
    let result = sqlx::query(
        "INSERT INTO audit_events (id, actor_user_id, action, resource_type, resource_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .execute(pool)
    .await;

    if let Err(error) = result {
        error!(%error, action, resource_type, "failed to write audit event");
    }
}

pub(crate) async fn bootstrap_admin(pool: &SqlitePool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        return Ok(());
    }

    let username = std::env::var("RIOTGIT_ADMIN_USERNAME").ok();
    let password = std::env::var("RIOTGIT_ADMIN_PASSWORD").ok();

    match (username, password) {
        (None, None) => {
            warn!(
                "no users configured; set RIOTGIT_ADMIN_USERNAME and RIOTGIT_ADMIN_PASSWORD to bootstrap the first administrator"
            );
            Ok(())
        }
        (Some(username), Some(password)) => {
            if !valid_username(&username) {
                anyhow::bail!(
                    "RIOTGIT_ADMIN_USERNAME must be 3-39 ASCII letters, digits, dashes, or underscores"
                );
            }
            if password.len() < 12 || password.len() > 1024 {
                anyhow::bail!("RIOTGIT_ADMIN_PASSWORD must contain 12-1024 bytes");
            }

            let password_hash = hash_password(password).await?;
            let user_id = Uuid::new_v4().simple().to_string();
            sqlx::query(
                "INSERT INTO users (id, username, display_name, password_hash, role, status)
                 VALUES (?, ?, ?, ?, 'admin', 'active')",
            )
            .bind(&user_id)
            .bind(&username)
            .bind(&username)
            .bind(password_hash)
            .execute(pool)
            .await?;

            record_audit(
                pool,
                Some(&user_id),
                "identity.bootstrap_admin",
                "user",
                Some(&user_id),
            )
            .await;
            info!(username = %username, "bootstrapped first R!OTGIT administrator");
            Ok(())
        }
        _ => anyhow::bail!(
            "RIOTGIT_ADMIN_USERNAME and RIOTGIT_ADMIN_PASSWORD must be configured together"
        ),
    }
}

pub(crate) async fn actor_from_token(token: &str, state: &AppState) -> Option<Actor> {
    if token.is_empty() || token.len() > 4096 {
        return None;
    }

    let actual: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    if let Some(expected) = state.api_token_digest {
        if bool::from(expected.ct_eq(&actual)) {
            return Some(Actor {
                user_id: "system".to_owned(),
                username: "system".to_owned(),
                role: "admin".to_owned(),
                session_id: None,
            });
        }
    }

    let digest = hex::encode(actual);
    sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT u.id, u.username, u.role, s.id
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token_digest = ?
           AND s.revoked_at IS NULL
           AND s.expires_at > ?
           AND u.status = 'active'",
    )
    .bind(digest)
    .bind(unix_now())
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|(user_id, username, role, session_id)| Actor {
        user_id,
        username,
        role,
        session_id: Some(session_id),
    })
}

pub(crate) async fn authenticate(headers: &HeaderMap, state: &AppState) -> Result<Actor, ApiError> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(ApiError::unauthorized)?;

    actor_from_token(token, state)
        .await
        .ok_or_else(ApiError::unauthorized)
}

pub(crate) async fn can_manage_repository(
    state: &AppState,
    actor: &Actor,
    repository_id: &str,
    owner: &str,
) -> bool {
    if actor.is_admin() || actor.username.eq_ignore_ascii_case(owner) {
        return true;
    }

    matches!(
        sqlx::query_scalar::<_, String>(
            "SELECT permission FROM repository_access
             WHERE repository_id = ? AND user_id = ?",
        )
        .bind(repository_id)
        .bind(&actor.user_id)
        .fetch_optional(&state.pool)
        .await,
        Ok(Some(permission)) if permission == "admin"
    )
}

pub(crate) async fn can_access_repository(
    state: &AppState,
    actor: &Actor,
    repository_id: &str,
    owner: &str,
    visibility: &str,
    write: bool,
) -> bool {
    if actor.is_admin() || actor.username.eq_ignore_ascii_case(owner) {
        return true;
    }

    if !write && matches!(visibility, "public" | "internal") {
        return true;
    }

    let permission = sqlx::query_scalar::<_, String>(
        "SELECT permission FROM repository_access
         WHERE repository_id = ? AND user_id = ?",
    )
    .bind(repository_id)
    .bind(&actor.user_id)
    .fetch_optional(&state.pool)
    .await;

    match permission {
        Ok(Some(permission)) if write => matches!(permission.as_str(), "write" | "admin"),
        Ok(Some(_)) => true,
        _ => false,
    }
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), ApiError> {
    if input.username.len() > 100 || input.password.len() > 1024 {
        return Err(ApiError::unauthorized());
    }

    let user = sqlx::query_as::<_, UserCredential>(
        "SELECT id, username, display_name, password_hash, role, status, created_at
         FROM users WHERE username = ? COLLATE NOCASE",
    )
    .bind(input.username.trim())
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to load login identity");
        ApiError::internal()
    })?;

    let Some(user) = user else {
        return Err(ApiError::unauthorized());
    };
    if user.status != "active" || !verify_password(user.password_hash, input.password).await {
        return Err(ApiError::unauthorized());
    }

    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let token_digest = hex::encode(Sha256::digest(token.as_bytes()));
    let expires_at = unix_now() + SESSION_TTL_SECONDS;
    let session_id = Uuid::new_v4().simple().to_string();

    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_digest, expires_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(token_digest)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to create user session");
        ApiError::internal()
    })?;

    record_audit(
        &state.pool,
        Some(&user.id),
        "identity.login",
        "session",
        Some(&session_id),
    )
    .await;

    let summary = UserSummary {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        role: user.role,
        status: user.status,
        created_at: user.created_at,
    };

    Ok((
        StatusCode::CREATED,
        Json(LoginResponse {
            access_token: token,
            token_type: "Bearer",
            expires_at,
            user: summary,
        }),
    ))
}

pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserSummary>, ApiError> {
    let actor = authenticate(&headers, &state).await?;
    let user = sqlx::query_as::<_, UserSummary>(
        "SELECT id, username, display_name, role, status, created_at
         FROM users WHERE id = ?",
    )
    .bind(&actor.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to load authenticated user");
        ApiError::internal()
    })?;

    match user {
        Some(user) => Ok(Json(user)),
        None if actor.user_id == "system" => Ok(Json(UserSummary {
            id: "system".to_owned(),
            username: "system".to_owned(),
            display_name: "Instance administrator".to_owned(),
            role: "admin".to_owned(),
            status: "active".to_owned(),
            created_at: String::new(),
        })),
        None => Err(ApiError::unauthorized()),
    }
}

pub(crate) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let actor = authenticate(&headers, &state).await?;
    if let Some(session_id) = actor.session_id {
        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(unix_now())
            .bind(&session_id)
            .execute(&state.pool)
            .await
            .map_err(|error| {
                error!(%error, "failed to revoke session");
                ApiError::internal()
            })?;

        record_audit(
            &state.pool,
            Some(&actor.user_id),
            "identity.logout",
            "session",
            Some(&session_id),
        )
        .await;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserSummary>), ApiError> {
    let actor = authenticate(&headers, &state).await?;
    if !actor.is_admin() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Administrator permission is required.",
        ));
    }

    if !valid_username(&input.username) {
        return Err(ApiError::bad_request(
            "Username must be 3-39 ASCII letters, digits, dashes, or underscores.",
        ));
    }
    if input.password.len() < 12 || input.password.len() > 1024 {
        return Err(ApiError::bad_request(
            "Password must contain 12-1024 bytes.",
        ));
    }
    if !matches!(input.role.as_str(), "admin" | "member") {
        return Err(ApiError::bad_request("Role must be admin or member."));
    }

    let display_name = input
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| input.username.clone());
    if display_name.len() > 100 {
        return Err(ApiError::bad_request(
            "Display name must be 100 characters or fewer.",
        ));
    }

    let password_hash = hash_password(input.password).await.map_err(|error| {
        error!(%error, "failed to hash new user password");
        ApiError::internal()
    })?;
    let id = Uuid::new_v4().simple().to_string();

    let user = sqlx::query_as::<_, UserSummary>(
        "INSERT INTO users (id, username, display_name, password_hash, role, status)
         VALUES (?, ?, ?, ?, ?, 'active')
         RETURNING id, username, display_name, role, status, created_at",
    )
    .bind(&id)
    .bind(&input.username)
    .bind(display_name)
    .bind(password_hash)
    .bind(&input.role)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|db| db.is_unique_violation())
        {
            ApiError::new(
                StatusCode::CONFLICT,
                "already_exists",
                "A user with this username already exists.",
            )
        } else {
            error!(%error, "failed to create user");
            ApiError::internal()
        }
    })?;

    record_audit(
        &state.pool,
        (actor.user_id != "system").then_some(actor.user_id.as_str()),
        "identity.create_user",
        "user",
        Some(&user.id),
    )
    .await;

    Ok((StatusCode::CREATED, Json(user)))
}
