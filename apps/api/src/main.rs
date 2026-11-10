use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    process::Stdio,
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    FromRow, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use subtle::ConstantTimeEq;
use tokio::{fs, net::TcpListener, process::Command, time::timeout};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    storage_root: PathBuf,
    template_dir: PathBuf,
    empty_git_config: PathBuf,
    git_bin: String,
    api_token_digest: Option<[u8; 32]>,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self { status, code, message }
    }

    fn bad_request(message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", "A valid bearer token is required.")
    }

    fn unavailable(message: &'static str) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, "not_configured", message)
    }

    fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "The request could not be completed.")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct MetaResponse {
    name: &'static str,
    display_name: &'static str,
    tagline: &'static str,
    api_version: &'static str,
    git_engine: &'static str,
    storage: &'static str,
}

#[derive(Debug, Clone, Serialize, FromRow)]
struct Repository {
    id: String,
    owner: String,
    name: String,
    description: Option<String>,
    visibility: String,
    state: String,
    #[serde(skip_serializing)]
    storage_key: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateRepositoryRequest {
    owner: String,
    name: String,
    description: Option<String>,
    #[serde(default = "default_visibility")]
    visibility: String,
}

fn default_visibility() -> String {
    "private".to_owned()
}

#[derive(Debug, Deserialize)]
struct Pagination {
    limit: Option<i64>,
    offset: Option<i64>,
    owner: Option<String>,
}

fn valid_segment(value: &str) -> bool {
    if value.is_empty() || value.len() > 100 || value == "." || value == ".." {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !value.ends_with('.')
}

fn authorize(headers: &HeaderMap, state: &AppState) -> Result<(), ApiError> {
    let Some(expected) = state.api_token_digest else {
        return Err(ApiError::unavailable(
            "Set RIOTGIT_API_TOKEN before using repository administration endpoints.",
        ));
    };

    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(ApiError::unauthorized)?;

    let actual: [u8; 32] = Sha256::digest(supplied.as_bytes()).into();
    if bool::from(expected.ct_eq(&actual)) {
        Ok(())
    } else {
        Err(ApiError::unauthorized())
    }
}

fn parse_bind() -> anyhow::Result<SocketAddr> {
    let raw = env::var("RIOTGIT_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());
    raw.parse()
        .map_err(|error| anyhow::anyhow!("invalid RIOTGIT_BIND '{raw}': {error}"))
}

async fn open_database() -> anyhow::Result<SqlitePool> {
    let path = env::var("RIOTGIT_DATABASE_PATH")
        .unwrap_or_else(|_| "data/riotgit.db".to_owned());
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

async fn prepare_storage() -> anyhow::Result<(PathBuf, PathBuf, PathBuf)> {
    let configured = env::var("RIOTGIT_STORAGE_ROOT")
        .unwrap_or_else(|_| "data/repositories".to_owned());
    fs::create_dir_all(&configured).await?;
    let root = fs::canonicalize(&configured).await?;
    let template_dir = root.join(".templates");
    let staging_dir = root.join(".staging");
    fs::create_dir_all(&template_dir).await?;
    fs::create_dir_all(&staging_dir).await?;
    let empty_git_config = root.join(".empty-gitconfig");
    if fs::metadata(&empty_git_config).await.is_err() {
        fs::write(&empty_git_config, b"").await?;
    }
    Ok((root, template_dir, empty_git_config))
}

async fn live() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse { status: "live", service: "riotgit-api" }),
    )
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let database_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();
    let storage_ok = fs::metadata(&state.storage_root).await.is_ok();
    if database_ok && storage_ok {
        (
            StatusCode::OK,
            Json(HealthResponse { status: "ready", service: "riotgit-api" }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse { status: "not_ready", service: "riotgit-api" }),
        )
    }
}

async fn meta() -> Json<MetaResponse> {
    Json(MetaResponse {
        name: "riotgit",
        display_name: "R!OTGIT",
        tagline: "THE UNDERGROUND CODE NETWORK",
        api_version: "v1",
        git_engine: "native-git",
        storage: "filesystem",
    })
}

async fn list_repositories(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Repository>>, ApiError> {
    authorize(&headers, &state)?;
    let limit = pagination.limit.unwrap_or(30).clamp(1, 100);
    let offset = pagination.offset.unwrap_or(0).max(0);

    let records = if let Some(owner) = pagination.owner {
        sqlx::query_as::<_, Repository>(
            "SELECT id, owner, name, description, visibility, state, storage_key, created_at, updated_at
             FROM repositories WHERE owner = ? AND state != 'failed'
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(owner)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, Repository>(
            "SELECT id, owner, name, description, visibility, state, storage_key, created_at, updated_at
             FROM repositories WHERE state != 'failed'
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|error| {
        error!(%error, "failed to list repositories");
        ApiError::internal()
    })?;

    Ok(Json(records))
}

async fn get_repository(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
) -> Result<Json<Repository>, ApiError> {
    authorize(&headers, &state)?;
    let record = sqlx::query_as::<_, Repository>(
        "SELECT id, owner, name, description, visibility, state, storage_key, created_at, updated_at
         FROM repositories WHERE owner = ? AND name = ? AND state != 'failed'",
    )
    .bind(owner)
    .bind(name)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to read repository metadata");
        ApiError::internal()
    })?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found", "Repository not found."))?;

    Ok(Json(record))
}

async fn create_repository(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateRepositoryRequest>,
) -> Result<(StatusCode, Json<Repository>), ApiError> {
    authorize(&headers, &state)?;

    if !valid_segment(&input.owner) || !valid_segment(&input.name) {
        return Err(ApiError::bad_request(
            "Owner and repository names must start with a letter or digit and contain only letters, digits, dot, dash, or underscore.",
        ));
    }
    if !matches!(input.visibility.as_str(), "public" | "private" | "internal") {
        return Err(ApiError::bad_request("Visibility must be public, private, or internal."));
    }
    if input.description.as_ref().is_some_and(|value| value.len() > 1000) {
        return Err(ApiError::bad_request("Description must be 1000 characters or fewer."));
    }

    let id = Uuid::new_v4().simple().to_string();
    let storage_key = format!("{id}.git");
    let staging_root = state.storage_root.join(".staging");
    let staging_path = staging_root.join(format!("{id}.git"));
    let final_path = state.storage_root.join(&storage_key);

    sqlx::query(
        "INSERT INTO repositories (id, owner, name, description, visibility, state, storage_key)
         VALUES (?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&id)
    .bind(&input.owner)
    .bind(&input.name)
    .bind(input.description.as_deref())
    .bind(&input.visibility)
    .bind(&storage_key)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        if error.as_database_error().is_some_and(|db| db.is_unique_violation()) {
            ApiError::new(StatusCode::CONFLICT, "already_exists", "A repository with this owner and name already exists.")
        } else {
            error!(%error, "failed to reserve repository metadata");
            ApiError::internal()
        }
    })?;

    let init = timeout(
        Duration::from_secs(20),
        Command::new(&state.git_bin)
            .arg("init")
            .arg("--bare")
            .arg(format!("--template={}", state.template_dir.display()))
            .arg("--initial-branch=main")
            .arg(&staging_path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &state.empty_git_config)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await;

    let output = match init {
        Ok(Ok(output)) if output.status.success() => output,
        Ok(Ok(output)) => {
            warn!(repo_id = %id, status = ?output.status, "native git init failed");
            let _ = fs::remove_dir_all(&staging_path).await;
            let _ = sqlx::query("DELETE FROM repositories WHERE id = ?")
                .bind(&id)
                .execute(&state.pool)
                .await;
            return Err(ApiError::internal());
        }
        Ok(Err(error)) => {
            error!(%error, repo_id = %id, "could not execute native git");
            let _ = fs::remove_dir_all(&staging_path).await;
            let _ = sqlx::query("DELETE FROM repositories WHERE id = ?")
                .bind(&id)
                .execute(&state.pool)
                .await;
            return Err(ApiError::internal());
        }
        Err(_) => {
            warn!(repo_id = %id, "native git init timed out");
            let _ = fs::remove_dir_all(&staging_path).await;
            let _ = sqlx::query("DELETE FROM repositories WHERE id = ?")
                .bind(&id)
                .execute(&state.pool)
                .await;
            return Err(ApiError::internal());
        }
    };

    if let Err(error) = fs::rename(&staging_path, &final_path).await {
        error!(%error, repo_id = %id, "could not activate repository storage");
        let _ = fs::remove_dir_all(&staging_path).await;
        let _ = sqlx::query("DELETE FROM repositories WHERE id = ?")
            .bind(&id)
            .execute(&state.pool)
            .await;
        return Err(ApiError::internal());
    }

    let updated = sqlx::query_as::<_, Repository>(
        "UPDATE repositories
         SET state = 'ready', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?
         RETURNING id, owner, name, description, visibility, state, storage_key, created_at, updated_at",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await;

    match updated {
        Ok(record) => Ok((StatusCode::CREATED, Json(record))),
        Err(error) => {
            error!(%error, repo_id = %id, "could not mark repository ready");
            let _ = fs::remove_dir_all(&final_path).await;
            let _ = sqlx::query("DELETE FROM repositories WHERE id = ?")
                .bind(&id)
                .execute(&state.pool)
                .await;
            Err(ApiError::internal())
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            warn!(%error, "failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => warn!(%error, "failed to listen for SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("riotgit_api=info,sqlx=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let (storage_root, template_dir, empty_git_config) = prepare_storage().await?;
    let pool = open_database().await?;
    let api_token_digest = env::var("RIOTGIT_API_TOKEN")
        .ok()
        .filter(|token| !token.trim().is_empty())
        .map(|token| Sha256::digest(token.as_bytes()).into());
    let git_bin = env::var("RIOTGIT_GIT_BIN").unwrap_or_else(|_| "git".to_owned());

    if api_token_digest.is_none() {
        warn!("RIOTGIT_API_TOKEN is not set; repository API is disabled until configured");
    }

    let state = AppState {
        pool,
        storage_root,
        template_dir,
        empty_git_config,
        git_bin,
        api_token_digest,
    };

    let app = Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/api/v1/meta", get(meta))
        .route("/api/v1/repositories", get(list_repositories).post(create_repository))
        .route("/api/v1/repositories/{owner}/{name}", get(get_repository))
        .with_state(state);

    let address = parse_bind()?;
    if !address.ip().is_loopback() {
        warn!(%address, "R!OTGIT API is exposed beyond loopback; use a trusted reverse proxy, TLS, and firewall rules");
    }
    let listener = TcpListener::bind(address).await?;
    info!(%address, "R!OTGIT API listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::valid_segment;

    #[test]
    fn accepts_simple_repository_segments() {
        assert!(valid_segment("moon-witness"));
        assert!(valid_segment("riot_git"));
        assert!(valid_segment("repo.v2"));
    }

    #[test]
    fn rejects_path_and_invalid_segments() {
        assert!(!valid_segment(""));
        assert!(!valid_segment("."));
        assert!(!valid_segment(".."));
        assert!(!valid_segment("../outside"));
        assert!(!valid_segment("repo/name"));
        assert!(!valid_segment("-starts-with-dash"));
        assert!(!valid_segment("trailing."));
        assert!(!valid_segment("white space"));
    }
}
