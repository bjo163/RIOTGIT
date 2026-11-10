use std::{path::PathBuf, process::Stdio, time::Duration};

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::{io::AsyncReadExt, process::Command, time::timeout};
use tracing::{error, warn};

use crate::{ApiError, AppState};

const TREE_LIMIT: usize = 8 * 1024 * 1024;
const BLOB_LIMIT: u64 = 1024 * 1024;
const COMMITS_LIMIT: i64 = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct TreeQuery {
    reference: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BlobQuery {
    reference: Option<String>,
    path: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CommitsQuery {
    reference: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DiffQuery {
    base: Option<String>,
    head: Option<String>,
}

#[derive(Debug, FromRow)]
struct RepositoryContext {
    id: String,
    owner: String,
    visibility: String,
    storage_key: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TreeEntry {
    name: String,
    path: String,
    mode: String,
    kind: String,
    object_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TreeResponse {
    owner: String,
    repository: String,
    reference: String,
    path: String,
    entries: Vec<TreeEntry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct BlobResponse {
    owner: String,
    repository: String,
    reference: String,
    path: String,
    size: u64,
    binary: bool,
    content: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommitEntry {
    id: String,
    author_name: String,
    author_email: String,
    authored_at: i64,
    subject: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommitListResponse {
    owner: String,
    repository: String,
    reference: String,
    commits: Vec<CommitEntry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DiffResponse {
    owner: String,
    repository: String,
    base: String,
    head: String,
    diff: String,
}

fn valid_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.starts_with('-')
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains("@{")
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains(':')
        && !value.chars().any(char::is_control)
        && !value.chars().any(char::is_whitespace)
}

fn valid_repository_path(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if value.starts_with('/') || value.starts_with('\\') || value.contains('\\') {
        return false;
    }
    value
        .split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
        && !value.chars().any(char::is_control)
}

async fn load_repository(
    state: &AppState,
    headers: &HeaderMap,
    owner: &str,
    name: &str,
) -> Result<RepositoryContext, ApiError> {
    let actor = crate::auth::authenticate(headers, state).await?;
    let repository = sqlx::query_as::<_, RepositoryContext>(
        "SELECT id, owner, visibility, storage_key
         FROM repositories
         WHERE owner = ? AND name = ? AND state = 'ready'",
    )
    .bind(owner)
    .bind(name)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to resolve repository for code browser");
        ApiError::internal()
    })?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found", "Repository not found."))?;

    if !crate::auth::can_access_repository(
        state,
        &actor,
        &repository.id,
        &repository.owner,
        &repository.visibility,
        false,
    )
    .await
    {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "Repository not found.",
        ));
    }

    Ok(repository)
}

async fn run_git(
    state: &AppState,
    storage_key: &str,
    args: &[String],
    output_limit: usize,
) -> Result<Vec<u8>, ApiError> {
    let repository_path: PathBuf = state.storage_root.join(storage_key);
    let mut command = Command::new(&state.git_bin);
    command
        .arg("--git-dir")
        .arg(repository_path)
        .args(args)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &state.empty_git_config)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(|error| {
        error!(%error, "failed to execute native Git for code browser");
        ApiError::internal()
    })?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill().await;
        return Err(ApiError::internal());
    };

    let mut bytes = Vec::with_capacity(output_limit.min(64 * 1024));
    let read_result = timeout(
        Duration::from_secs(15),
        stdout.take(output_limit as u64 + 1).read_to_end(&mut bytes),
    )
    .await;

    match read_result {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            error!(%error, "failed to read bounded native Git output");
            let _ = child.kill().await;
            return Err(ApiError::internal());
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "git_timeout",
                "Repository read exceeded the 15 second execution limit.",
            ));
        }
    }

    if bytes.len() > output_limit {
        let _ = child.kill().await;
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "git_response_too_large",
            "The requested repository response exceeds the configured limit.",
        ));
    }

    let status = timeout(Duration::from_secs(1), child.wait())
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "git_timeout",
                "Repository read exceeded the execution limit.",
            )
        })?
        .map_err(|error| {
            error!(%error, "failed to wait for native Git process");
            ApiError::internal()
        })?;

    if !status.success() {
        warn!(status = ?status, storage_key, "native Git read command returned an error");
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "git_object_not_found",
            "The requested revision or repository path was not found.",
        ));
    }

    Ok(bytes)
}

async fn resolve_commit(
    state: &AppState,
    repository: &RepositoryContext,
    reference: &str,
) -> Result<String, ApiError> {
    if !valid_reference(reference) {
        return Err(ApiError::bad_request("Invalid Git reference."));
    }
    let revision = format!("{reference}^{{commit}}");
    let output = run_git(
        state,
        &repository.storage_key,
        &[
            "rev-parse".to_owned(),
            "--verify".to_owned(),
            "--end-of-options".to_owned(),
            revision,
        ],
        1024,
    )
    .await?;
    String::from_utf8(output)
        .map(|value| value.trim().to_owned())
        .map_err(|_| ApiError::internal())
}

pub(crate) async fn tree(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<TreeQuery>,
) -> Result<Json<TreeResponse>, ApiError> {
    let repository = load_repository(&state, &headers, &owner, &name).await?;
    let reference = query.reference.unwrap_or_else(|| "HEAD".to_owned());
    let path = query.path.unwrap_or_default();
    if !valid_repository_path(&path) {
        return Err(ApiError::bad_request("Invalid repository path."));
    }

    let commit = resolve_commit(&state, &repository, &reference).await?;
    let tree_object = if path.is_empty() {
        commit
    } else {
        format!("{commit}:{path}")
    };
    let output = run_git(
        &state,
        &repository.storage_key,
        &[
            "ls-tree".to_owned(),
            "-z".to_owned(),
            "--full-tree".to_owned(),
            tree_object,
        ],
        TREE_LIMIT,
    )
    .await?;

    let mut entries = Vec::new();
    for record in output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let Some(tab_index) = record.iter().position(|byte| *byte == b'\t') else {
            continue;
        };
        let metadata =
            std::str::from_utf8(&record[..tab_index]).map_err(|_| ApiError::internal())?;
        let mut fields = metadata.split_whitespace();
        let mode = fields.next().unwrap_or_default().to_owned();
        let kind = fields.next().unwrap_or_default().to_owned();
        let object_id = fields.next().unwrap_or_default().to_owned();
        if mode.is_empty() || kind.is_empty() || object_id.is_empty() {
            continue;
        }
        let entry_name = String::from_utf8_lossy(&record[tab_index + 1..]).into_owned();
        let entry_path = if path.is_empty() {
            entry_name.clone()
        } else {
            format!("{path}/{entry_name}")
        };
        entries.push(TreeEntry {
            name: entry_name,
            path: entry_path,
            mode,
            kind,
            object_id,
        });
    }

    Ok(Json(TreeResponse {
        owner,
        repository: name,
        reference,
        path,
        entries,
    }))
}

pub(crate) async fn blob(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<BlobQuery>,
) -> Result<Json<BlobResponse>, ApiError> {
    let repository = load_repository(&state, &headers, &owner, &name).await?;
    let reference = query.reference.unwrap_or_else(|| "HEAD".to_owned());
    if !valid_repository_path(&query.path) || query.path.is_empty() {
        return Err(ApiError::bad_request(
            "A valid repository file path is required.",
        ));
    }

    let commit = resolve_commit(&state, &repository, &reference).await?;
    let object_spec = format!("{commit}:{}", query.path);
    let object_id = run_git(
        &state,
        &repository.storage_key,
        &[
            "rev-parse".to_owned(),
            "--verify".to_owned(),
            "--end-of-options".to_owned(),
            object_spec,
        ],
        1024,
    )
    .await?;
    let object_id = String::from_utf8(object_id)
        .map_err(|_| ApiError::internal())?
        .trim()
        .to_owned();

    let kind = run_git(
        &state,
        &repository.storage_key,
        &["cat-file".to_owned(), "-t".to_owned(), object_id.clone()],
        256,
    )
    .await?;
    if String::from_utf8_lossy(&kind).trim() != "blob" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "not_a_file",
            "The requested repository path is not a file.",
        ));
    }

    let size_output = run_git(
        &state,
        &repository.storage_key,
        &["cat-file".to_owned(), "-s".to_owned(), object_id.clone()],
        256,
    )
    .await?;
    let size = String::from_utf8(size_output)
        .map_err(|_| ApiError::internal())?
        .trim()
        .parse::<u64>()
        .map_err(|_| ApiError::internal())?;
    if size > BLOB_LIMIT {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "blob_too_large",
            "File preview is limited to 1 MiB.",
        ));
    }

    let bytes = run_git(
        &state,
        &repository.storage_key,
        &["cat-file".to_owned(), "blob".to_owned(), object_id],
        BLOB_LIMIT as usize,
    )
    .await?;
    let content = String::from_utf8(bytes).ok();
    let binary = content
        .as_ref()
        .is_none_or(|value| value.as_bytes().contains(&0));

    Ok(Json(BlobResponse {
        owner,
        repository: name,
        reference,
        path: query.path,
        size,
        binary,
        content: if binary { None } else { content },
    }))
}

pub(crate) async fn commits(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<CommitsQuery>,
) -> Result<Json<CommitListResponse>, ApiError> {
    let repository = load_repository(&state, &headers, &owner, &name).await?;
    let reference = query.reference.unwrap_or_else(|| "HEAD".to_owned());
    let limit = query.limit.unwrap_or(20).clamp(1, COMMITS_LIMIT);
    let commit = resolve_commit(&state, &repository, &reference).await?;
    let format = "%H%x1f%an%x1f%ae%x1f%at%x1f%s";
    let output = run_git(
        &state,
        &repository.storage_key,
        &[
            "log".to_owned(),
            format!("--format={format}"),
            "-n".to_owned(),
            limit.to_string(),
            commit,
        ],
        2 * 1024 * 1024,
    )
    .await?;

    let mut entries = Vec::new();
    for line in String::from_utf8_lossy(&output).lines() {
        let fields: Vec<&str> = line.splitn(5, '\u{1f}').collect();
        if fields.len() != 5 {
            continue;
        }
        let Ok(authored_at) = fields[3].parse::<i64>() else {
            continue;
        };
        entries.push(CommitEntry {
            id: fields[0].to_owned(),
            author_name: fields[1].to_owned(),
            author_email: fields[2].to_owned(),
            authored_at,
            subject: fields[4].to_owned(),
        });
    }

    Ok(Json(CommitListResponse {
        owner,
        repository: name,
        reference,
        commits: entries,
    }))
}

pub(crate) async fn diff(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<DiffQuery>,
) -> Result<Json<DiffResponse>, ApiError> {
    let repository = load_repository(&state, &headers, &owner, &name).await?;
    let base_reference = query.base.unwrap_or_else(|| "main".to_owned());
    let head_reference = query.head.unwrap_or_else(|| "HEAD".to_owned());
    let base_commit = resolve_commit(&state, &repository, &base_reference).await?;
    let head_commit = resolve_commit(&state, &repository, &head_reference).await?;

    let output = run_git(
        &state,
        &repository.storage_key,
        &[
            "diff".to_owned(),
            "--no-ext-diff".to_owned(),
            "--no-textconv".to_owned(),
            "--no-color".to_owned(),
            "--no-renames".to_owned(),
            "--unified=3".to_owned(),
            base_commit,
            head_commit,
            "--".to_owned(),
        ],
        2 * 1024 * 1024,
    )
    .await?;

    Ok(Json(DiffResponse {
        owner,
        repository: name,
        base: base_reference,
        head: head_reference,
        diff: String::from_utf8_lossy(&output).into_owned(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{valid_reference, valid_repository_path};

    #[test]
    fn rejects_unsafe_git_references() {
        assert!(valid_reference("HEAD"));
        assert!(valid_reference("main"));
        assert!(valid_reference("feature/code-browser"));
        assert!(!valid_reference(""));
        assert!(!valid_reference("--upload-pack=evil"));
        assert!(!valid_reference("refs/heads/../secret"));
        assert!(!valid_reference("bad ref"));
        assert!(!valid_reference("main:src"));
    }

    #[test]
    fn rejects_unsafe_repository_paths() {
        assert!(valid_repository_path(""));
        assert!(valid_repository_path("src/main.rs"));
        assert!(valid_repository_path("docs/Release Notes.md"));
        assert!(!valid_repository_path("../outside"));
        assert!(!valid_repository_path("/absolute/path"));
        assert!(!valid_repository_path("src//main.rs"));
        assert!(!valid_repository_path("src\\main.rs"));
    }
}
