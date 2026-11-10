use std::{
    path::PathBuf,
    process::{ExitStatus, Stdio},
    time::Duration,
};

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::{io::AsyncReadExt, process::Command, time::timeout};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{ApiError, AppState};

const GIT_OUTPUT_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Debug, FromRow, Serialize)]
pub(crate) struct PullRequest {
    id: String,
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    head_ref: String,
    base_ref: String,
    head_sha: String,
    base_sha: String,
    author_id: String,
    merged_by: Option<String>,
    merge_commit: Option<String>,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    merged_at: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub(crate) struct PullRequestReview {
    id: String,
    reviewer_id: String,
    state: String,
    body: Option<String>,
    submitted_at: String,
}

#[derive(Debug, FromRow)]
struct RepositoryContext {
    id: String,
    owner: String,
    visibility: String,
    storage_key: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListPullRequestsQuery {
    state: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreatePullRequestRequest {
    title: String,
    body: Option<String>,
    head_ref: String,
    base_ref: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SubmitReviewRequest {
    state: String,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdatePullRequestRequest {
    state: String,
}

async fn repository_context(
    state: &AppState,
    headers: &HeaderMap,
    owner: &str,
    name: &str,
    write: bool,
) -> Result<(crate::auth::Actor, RepositoryContext), ApiError> {
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
        error!(%error, "failed to resolve repository for pull request");
        ApiError::internal()
    })?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found", "Repository not found."))?;

    if !crate::auth::can_access_repository(
        state,
        &actor,
        &repository.id,
        &repository.owner,
        &repository.visibility,
        write,
    )
    .await
    {
        return Err(ApiError::new(
            if write {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::NOT_FOUND
            },
            if write { "forbidden" } else { "not_found" },
            if write {
                "Repository write permission is required."
            } else {
                "Repository not found."
            },
        ));
    }

    Ok((actor, repository))
}

fn validate_title_and_body(title: &str, body: Option<&str>) -> Result<(), ApiError> {
    if title.trim().is_empty() || title.trim().len() > 200 {
        return Err(ApiError::bad_request(
            "Pull request title must contain 1-200 bytes.",
        ));
    }
    if body.is_some_and(|value| value.len() > 100_000) {
        return Err(ApiError::bad_request(
            "Pull request description must be 100000 bytes or fewer.",
        ));
    }
    Ok(())
}

async fn run_git(
    state: &AppState,
    storage_key: &str,
    args: &[String],
    output_limit: usize,
    commit_identity: bool,
) -> Result<(ExitStatus, Vec<u8>), ApiError> {
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

    if commit_identity {
        command
            .env("GIT_AUTHOR_NAME", "R!OTGIT")
            .env("GIT_AUTHOR_EMAIL", "merge-bot@riotgit.invalid")
            .env("GIT_COMMITTER_NAME", "R!OTGIT")
            .env("GIT_COMMITTER_EMAIL", "merge-bot@riotgit.invalid");
    }

    let mut child = command.spawn().map_err(|error| {
        error!(%error, "failed to spawn Git for pull request operation");
        ApiError::internal()
    })?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill().await;
        return Err(ApiError::internal());
    };

    let mut output = Vec::with_capacity(output_limit.min(64 * 1024));
    match timeout(
        Duration::from_secs(30),
        stdout
            .take(output_limit as u64 + 1)
            .read_to_end(&mut output),
    )
    .await
    {
        Ok(Ok(_)) if output.len() <= output_limit => {}
        Ok(Ok(_)) => {
            let _ = child.kill().await;
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "git_output_too_large",
                "Git operation returned too much output.",
            ));
        }
        Ok(Err(error)) => {
            error!(%error, "failed to read Git output for pull request");
            let _ = child.kill().await;
            return Err(ApiError::internal());
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "git_timeout",
                "Git operation exceeded the 30 second execution limit.",
            ));
        }
    }

    let status = timeout(Duration::from_secs(2), child.wait())
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "git_timeout",
                "Git process did not exit after producing output.",
            )
        })?
        .map_err(|error| {
            error!(%error, "failed to wait for Git pull request process");
            ApiError::internal()
        })?;

    Ok((status, output))
}

async fn resolve_branch(
    state: &AppState,
    repository: &RepositoryContext,
    branch: &str,
) -> Result<String, ApiError> {
    if !crate::branch_protection::valid_branch_name(branch) {
        return Err(ApiError::bad_request("Invalid Git branch name."));
    }

    let revision = format!("refs/heads/{branch}^{{commit}}");
    let (status, output) = run_git(
        state,
        &repository.storage_key,
        &[
            "rev-parse".to_owned(),
            "--verify".to_owned(),
            "--end-of-options".to_owned(),
            revision,
        ],
        1024,
        false,
    )
    .await?;
    if !status.success() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "branch_not_found",
            "The selected branch does not exist.",
        ));
    }
    let sha = String::from_utf8(output)
        .map_err(|_| ApiError::internal())?
        .trim()
        .to_owned();
    if !valid_object_id(&sha) {
        return Err(ApiError::internal());
    }
    Ok(sha)
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

async fn reconcile_pending_merge(
    state: &AppState,
    repository: &RepositoryContext,
    pull: PullRequest,
) -> Result<PullRequest, ApiError> {
    if pull.state != "merging" {
        return Ok(pull);
    }

    let Some(merge_commit) = pull.merge_commit.as_deref() else {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "merge_recovery_required",
            "The pull request has an incomplete merge record and needs administrator recovery.",
        ));
    };

    let current_base = resolve_branch(state, repository, &pull.base_ref).await?;
    if current_base == merge_commit {
        sqlx::query(
            "UPDATE pull_requests
             SET state = 'merged',
                 merged_at = COALESCE(merged_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND state = 'merging' AND merge_commit = ?",
        )
        .bind(&pull.id)
        .bind(merge_commit)
        .execute(&state.pool)
        .await
        .map_err(|error| {
            error!(%error, pull_request_id = %pull.id, "failed to finalize recovered pull request merge");
            ApiError::internal()
        })?;
    } else if current_base == pull.base_sha {
        sqlx::query(
            "UPDATE pull_requests
             SET state = 'open', merged_by = NULL, merge_commit = NULL,
                 merged_at = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND state = 'merging' AND merge_commit = ?",
        )
        .bind(&pull.id)
        .bind(merge_commit)
        .execute(&state.pool)
        .await
        .map_err(|error| {
            error!(%error, pull_request_id = %pull.id, "failed to reopen interrupted pull request merge");
            ApiError::internal()
        })?;
    } else {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "merge_recovery_required",
            "The base branch moved to an unexpected commit while this pull request was merging. Administrator recovery is required.",
        ));
    }

    load_pull_request(state, &repository.id, pull.number).await
}

async fn load_pull_request(
    state: &AppState,
    repository_id: &str,
    number: i64,
) -> Result<PullRequest, ApiError> {
    sqlx::query_as::<_, PullRequest>(
        "SELECT id, number, title, body, state, head_ref, base_ref, head_sha, base_sha,
                author_id, merged_by, merge_commit, created_at, updated_at, closed_at, merged_at
         FROM pull_requests
         WHERE repository_id = ? AND number = ?",
    )
    .bind(repository_id)
    .bind(number)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to load pull request");
        ApiError::internal()
    })?
    .ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "pull_request_not_found",
            "Pull request not found.",
        )
    })
}

pub(crate) async fn list_pull_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<ListPullRequestsQuery>,
) -> Result<Json<Vec<PullRequest>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name, false).await?;
    let state_filter = match query.state.as_deref() {
        None | Some("") => None,
        Some("open") => Some("open"),
        Some("closed") => Some("closed"),
        Some("merged") => Some("merged"),
        Some(_) => {
            return Err(ApiError::bad_request(
                "Pull request state must be open, closed, or merged.",
            ));
        }
    };
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let pulls = sqlx::query_as::<_, PullRequest>(
        "SELECT id, number, title, body, state, head_ref, base_ref, head_sha, base_sha,
                author_id, merged_by, merge_commit, created_at, updated_at, closed_at, merged_at
         FROM pull_requests
         WHERE repository_id = ? AND (? IS NULL OR state = ?)
         ORDER BY number DESC
         LIMIT ? OFFSET ?",
    )
    .bind(&repository.id)
    .bind(state_filter)
    .bind(state_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list pull requests");
        ApiError::internal()
    })?;

    Ok(Json(pulls))
}

pub(crate) async fn create_pull_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Json(input): Json<CreatePullRequestRequest>,
) -> Result<(StatusCode, Json<PullRequest>), ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name, true).await?;
    if actor.user_id == "system" {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "named_account_required",
            "Use a named account to open a pull request.",
        ));
    }
    validate_title_and_body(&input.title, input.body.as_deref())?;

    if input.head_ref == input.base_ref {
        return Err(ApiError::bad_request(
            "Head and base branches must be different.",
        ));
    }

    let head_sha = resolve_branch(&state, &repository, &input.head_ref).await?;
    let base_sha = resolve_branch(&state, &repository, &input.base_ref).await?;
    if head_sha == base_sha {
        return Err(ApiError::bad_request(
            "The selected branches have no different commits.",
        ));
    }

    let (merge_base_status, _) = run_git(
        &state,
        &repository.storage_key,
        &["merge-base".to_owned(), base_sha.clone(), head_sha.clone()],
        1024,
        false,
    )
    .await?;
    if !merge_base_status.success() {
        return Err(ApiError::bad_request(
            "The selected branches do not share a merge base.",
        ));
    }

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM pull_requests
            WHERE repository_id = ? AND head_ref = ? AND base_ref = ? AND state = 'open'
        )",
    )
    .bind(&repository.id)
    .bind(&input.head_ref)
    .bind(&input.base_ref)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to check duplicate pull request");
        ApiError::internal()
    })?;
    if exists {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_exists",
            "An open pull request already exists for these branches.",
        ));
    }

    let pull = sqlx::query_as::<_, PullRequest>(
        "INSERT INTO pull_requests (
             id, repository_id, number, title, body, state,
             head_ref, base_ref, head_sha, base_sha, author_id
         )
         VALUES (
             ?, ?, (SELECT COALESCE(MAX(number), 0) + 1 FROM pull_requests WHERE repository_id = ?),
             ?, ?, 'open', ?, ?, ?, ?, ?
         )
         RETURNING id, number, title, body, state, head_ref, base_ref, head_sha, base_sha,
                   author_id, merged_by, merge_commit, created_at, updated_at, closed_at, merged_at",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&repository.id)
    .bind(&repository.id)
    .bind(input.title.trim())
    .bind(input.body.filter(|value| !value.trim().is_empty()))
    .bind(&input.head_ref)
    .bind(&input.base_ref)
    .bind(&head_sha)
    .bind(&base_sha)
    .bind(&actor.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to create pull request");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "pull_request.create",
        "pull_request",
        Some(&pull.id),
    )
    .await;

    Ok((StatusCode::CREATED, Json(pull)))
}

pub(crate) async fn get_pull_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
) -> Result<Json<PullRequest>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name, false).await?;
    let pull = load_pull_request(&state, &repository.id, number).await?;
    Ok(Json(
        reconcile_pending_merge(&state, &repository, pull).await?,
    ))
}

pub(crate) async fn update_pull_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
    Json(input): Json<UpdatePullRequestRequest>,
) -> Result<Json<PullRequest>, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name, false).await?;
    if !matches!(input.state.as_str(), "open" | "closed") {
        return Err(ApiError::bad_request(
            "Pull request state must be open or closed.",
        ));
    }
    let current = load_pull_request(&state, &repository.id, number).await?;
    if current.state == "merged" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_merged",
            "A merged pull request cannot be reopened or closed.",
        ));
    }
    if current.state == "merging" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_merging",
            "A pull request cannot be changed while its merge is being finalized.",
        ));
    }
    let can_write = crate::auth::can_access_repository(
        &state,
        &actor,
        &repository.id,
        &repository.owner,
        &repository.visibility,
        true,
    )
    .await;
    if !can_write && current.author_id != actor.user_id {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Pull request author or repository writer permission is required.",
        ));
    }

    let pull = sqlx::query_as::<_, PullRequest>(
        "UPDATE pull_requests
         SET state = ?,
             closed_at = CASE WHEN ? = 'closed' THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE NULL END,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE repository_id = ? AND number = ?
         RETURNING id, number, title, body, state, head_ref, base_ref, head_sha, base_sha,
                   author_id, merged_by, merge_commit, created_at, updated_at, closed_at, merged_at",
    )
    .bind(&input.state)
    .bind(&input.state)
    .bind(&repository.id)
    .bind(number)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to update pull request state");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "pull_request.update",
        "pull_request",
        Some(&pull.id),
    )
    .await;
    Ok(Json(pull))
}

pub(crate) async fn list_reviews(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
) -> Result<Json<Vec<PullRequestReview>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name, false).await?;
    let pull = load_pull_request(&state, &repository.id, number).await?;
    let reviews = sqlx::query_as::<_, PullRequestReview>(
        "SELECT id, reviewer_id, state, body, submitted_at
         FROM pull_request_reviews
         WHERE pull_request_id = ?
         ORDER BY submitted_at DESC",
    )
    .bind(&pull.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list pull request reviews");
        ApiError::internal()
    })?;
    Ok(Json(reviews))
}

pub(crate) async fn submit_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
    Json(input): Json<SubmitReviewRequest>,
) -> Result<(StatusCode, Json<PullRequestReview>), ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name, false).await?;
    if actor.user_id == "system" {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "named_account_required",
            "Use a named account to submit a pull request review.",
        ));
    }
    if !matches!(
        input.state.as_str(),
        "comment" | "approved" | "changes_requested"
    ) {
        return Err(ApiError::bad_request(
            "Review state must be comment, approved, or changes_requested.",
        ));
    }
    if input
        .body
        .as_ref()
        .is_some_and(|value| value.len() > 20_000)
    {
        return Err(ApiError::bad_request(
            "Review body must be 20000 bytes or fewer.",
        ));
    }

    let pull = load_pull_request(&state, &repository.id, number).await?;
    let pull = reconcile_pending_merge(&state, &repository, pull).await?;
    if pull.state != "open" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_not_open",
            "Only an open pull request can be reviewed.",
        ));
    }
    if pull.author_id == actor.user_id {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "self_review",
            "Pull request authors cannot approve or review their own pull request.",
        ));
    }

    let review = sqlx::query_as::<_, PullRequestReview>(
        "INSERT INTO pull_request_reviews (id, pull_request_id, reviewer_id, state, body)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id, reviewer_id, state, body, submitted_at",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&pull.id)
    .bind(&actor.user_id)
    .bind(&input.state)
    .bind(input.body.filter(|value| !value.trim().is_empty()))
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to save pull request review");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "pull_request.review",
        "pull_request",
        Some(&pull.id),
    )
    .await;

    Ok((StatusCode::CREATED, Json(review)))
}

async fn has_required_approval(state: &AppState, pull: &PullRequest) -> Result<bool, ApiError> {
    let approvals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM (
             SELECT reviewer_id, state,
                    ROW_NUMBER() OVER (
                        PARTITION BY reviewer_id
                        ORDER BY submitted_at DESC, id DESC
                    ) AS review_rank
             FROM pull_request_reviews
             WHERE pull_request_id = ?
         ) latest
         WHERE review_rank = 1 AND state = 'approved' AND reviewer_id != ?",
    )
    .bind(&pull.id)
    .bind(&pull.author_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to evaluate pull request approvals");
        ApiError::internal()
    })?;
    Ok(approvals > 0)
}

pub(crate) async fn merge_pull_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
) -> Result<Json<PullRequest>, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name, true).await?;
    if actor.user_id == "system" {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "named_account_required",
            "Use a named account to merge a pull request.",
        ));
    }
    let pull = load_pull_request(&state, &repository.id, number).await?;
    if pull.state != "open" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_not_open",
            "Only an open pull request can be merged.",
        ));
    }

    let protected = sqlx::query_scalar::<_, bool>(
        "SELECT require_pull_request FROM branch_protection_rules
         WHERE repository_id = ? AND branch_name = ?",
    )
    .bind(&repository.id)
    .bind(&pull.base_ref)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to load branch protection for pull request merge");
        ApiError::internal()
    })?
    .unwrap_or(false);

    if protected && !has_required_approval(&state, &pull).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "review_required",
            "A protected branch requires at least one current approval from someone other than the pull request author.",
        ));
    }

    let current_head = resolve_branch(&state, &repository, &pull.head_ref).await?;
    let current_base = resolve_branch(&state, &repository, &pull.base_ref).await?;
    if current_head != pull.head_sha || current_base != pull.base_sha {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_stale",
            "The head or base branch changed after this pull request was opened. Recreate or refresh the pull request.",
        ));
    }

    let (tree_status, tree_output) = run_git(
        &state,
        &repository.storage_key,
        &[
            "merge-tree".to_owned(),
            "--write-tree".to_owned(),
            pull.base_sha.clone(),
            pull.head_sha.clone(),
        ],
        GIT_OUTPUT_LIMIT,
        false,
    )
    .await?;
    if !tree_status.success() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "merge_conflict",
            "Git could not produce a clean merge tree. Resolve conflicts on the head branch and open a new pull request.",
        ));
    }

    let tree_sha = String::from_utf8(tree_output)
        .map_err(|_| ApiError::internal())?
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    if !valid_object_id(&tree_sha) {
        return Err(ApiError::internal());
    }

    let message = format!("Merge pull request #{}: {}", pull.number, pull.title);
    let (commit_status, commit_output) = run_git(
        &state,
        &repository.storage_key,
        &[
            "commit-tree".to_owned(),
            tree_sha,
            "-p".to_owned(),
            pull.base_sha.clone(),
            "-p".to_owned(),
            pull.head_sha.clone(),
            "-m".to_owned(),
            message,
        ],
        1024,
        true,
    )
    .await?;
    if !commit_status.success() {
        return Err(ApiError::internal());
    }
    let merge_commit = String::from_utf8(commit_output)
        .map_err(|_| ApiError::internal())?
        .trim()
        .to_owned();
    if !valid_object_id(&merge_commit) {
        return Err(ApiError::internal());
    }

    let reserved = sqlx::query(
        "UPDATE pull_requests
         SET state = 'merging', merged_by = ?, merge_commit = ?,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ? AND state = 'open' AND base_sha = ? AND head_sha = ?",
    )
    .bind(&actor.user_id)
    .bind(&merge_commit)
    .bind(&pull.id)
    .bind(&pull.base_sha)
    .bind(&pull.head_sha)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, pull_request_id = %pull.id, "failed to reserve pull request merge");
        ApiError::internal()
    })?;
    if reserved.rows_affected() != 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "pull_request_changed",
            "The pull request changed before merge could be reserved.",
        ));
    }

    let reference = format!("refs/heads/{}", pull.base_ref);
    let (update_status, _) = run_git(
        &state,
        &repository.storage_key,
        &[
            "update-ref".to_owned(),
            reference,
            merge_commit.clone(),
            pull.base_sha.clone(),
        ],
        1024,
        false,
    )
    .await?;
    if !update_status.success() {
        sqlx::query(
            "UPDATE pull_requests
             SET state = 'open', merged_by = NULL, merge_commit = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND state = 'merging' AND merge_commit = ?",
        )
        .bind(&pull.id)
        .bind(&merge_commit)
        .execute(&state.pool)
        .await
        .map_err(|error| {
            error!(%error, pull_request_id = %pull.id, "failed to roll back reserved pull request merge");
            ApiError::internal()
        })?;
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "base_branch_changed",
            "The base branch changed during merge. Fetch the latest refs and retry.",
        ));
    }

    let updated = sqlx::query_as::<_, PullRequest>(
        "UPDATE pull_requests
         SET state = 'merged',
             merged_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ? AND state = 'merging' AND merge_commit = ?
         RETURNING id, number, title, body, state, head_ref, base_ref, head_sha, base_sha,
                   author_id, merged_by, merge_commit, created_at, updated_at, closed_at, merged_at",
    )
    .bind(&pull.id)
    .bind(&merge_commit)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, pull_request_id = %pull.id, merge_commit = %merge_commit, "branch merged but pull request finalization failed");
        ApiError::internal()
    })?
    .ok_or_else(|| {
        error!(pull_request_id = %pull.id, merge_commit = %merge_commit, "branch merged but pull request finalization did not update a row");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "pull_request.merge",
        "pull_request",
        Some(&pull.id),
    )
    .await;

    Ok(Json(updated))
}

#[cfg(test)]
mod tests {
    use super::valid_object_id;

    #[test]
    fn accepts_sha1_and_sha256_object_ids() {
        assert!(valid_object_id("1111111111111111111111111111111111111111"));
        assert!(valid_object_id(
            "1111111111111111111111111111111111111111111111111111111111111111"
        ));
    }

    #[test]
    fn rejects_invalid_object_ids() {
        assert!(!valid_object_id(""));
        assert!(!valid_object_id("xyz"));
        assert!(!valid_object_id("111111111111111111111111111111111111111g"));
    }
}
