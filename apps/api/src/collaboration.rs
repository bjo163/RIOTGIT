use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::error;
use uuid::Uuid;

use crate::{ApiError, AppState};

#[derive(Debug, FromRow)]
struct RepositoryContext {
    id: String,
    owner: String,
    visibility: String,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Issue {
    id: String,
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    author_id: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    milestone_id: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct IssueComment {
    id: String,
    issue_number: i64,
    author_id: String,
    body: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListIssuesQuery {
    state: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateIssueRequest {
    title: String,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateIssueRequest {
    title: Option<String>,
    body: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateCommentRequest {
    body: String,
}

async fn repository_context(
    state: &AppState,
    headers: &HeaderMap,
    owner: &str,
    name: &str,
) -> Result<(crate::auth::Actor, RepositoryContext), ApiError> {
    let actor = crate::auth::authenticate(headers, state).await?;
    let repository = sqlx::query_as::<_, RepositoryContext>(
        "SELECT id, owner, visibility FROM repositories
         WHERE owner = ? AND name = ? AND state = 'ready'",
    )
    .bind(owner)
    .bind(name)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to resolve repository for issue operation");
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

    Ok((actor, repository))
}

fn require_named_actor(actor: &crate::auth::Actor) -> Result<(), ApiError> {
    if actor.user_id == "system" {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "named_account_required",
            "Use a named account for collaboration actions.",
        ));
    }
    Ok(())
}

fn validate_body(body: &str) -> Result<(), ApiError> {
    if body.trim().is_empty() || body.len() > 100_000 {
        return Err(ApiError::bad_request(
            "Issue body must contain text and be 100000 bytes or fewer.",
        ));
    }
    Ok(())
}

async fn load_issue(state: &AppState, repository_id: &str, number: i64) -> Result<Issue, ApiError> {
    sqlx::query_as::<_, Issue>(
        "SELECT id, number, title, body, state, author_id, created_at, updated_at, closed_at, milestone_id
         FROM issues WHERE repository_id = ? AND number = ?",
    )
    .bind(repository_id)
    .bind(number)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to load issue");
        ApiError::internal()
    })?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "issue_not_found", "Issue not found."))
}

pub(crate) async fn list_issues(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<Vec<Issue>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    let state_filter = match query.state.as_deref() {
        None | Some("") => None,
        Some("open") => Some("open"),
        Some("closed") => Some("closed"),
        Some(_) => return Err(ApiError::bad_request("Issue state must be open or closed.")),
    };
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let issues = sqlx::query_as::<_, Issue>(
        "SELECT id, number, title, body, state, author_id, created_at, updated_at, closed_at, milestone_id
         FROM issues
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
        error!(%error, "failed to list repository issues");
        ApiError::internal()
    })?;

    Ok(Json(issues))
}

pub(crate) async fn create_issue(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Json(input): Json<CreateIssueRequest>,
) -> Result<(StatusCode, Json<Issue>), ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_named_actor(&actor)?;
    if !crate::auth::can_access_repository(
        &state,
        &actor,
        &repository.id,
        &repository.owner,
        &repository.visibility,
        true,
    )
    .await
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Write permission is required to create an issue.",
        ));
    }

    let title = input.title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(ApiError::bad_request(
            "Issue title must contain 1-200 bytes.",
        ));
    }
    if let Some(body) = input.body.as_deref() {
        if !body.is_empty() {
            validate_body(body)?;
        }
    }

    let issue = sqlx::query_as::<_, Issue>(
        "INSERT INTO issues (id, repository_id, number, title, body, state, author_id)
         VALUES (?, ?, (SELECT COALESCE(MAX(number), 0) + 1 FROM issues WHERE repository_id = ?), ?, ?, 'open', ?)
         RETURNING id, number, title, body, state, author_id, created_at, updated_at, closed_at, milestone_id",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&repository.id)
    .bind(&repository.id)
    .bind(title)
    .bind(input.body.filter(|body| !body.is_empty()))
    .bind(&actor.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to create repository issue");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.create",
        "issue",
        Some(&issue.id),
    )
    .await;

    Ok((StatusCode::CREATED, Json(issue)))
}

pub(crate) async fn update_issue(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
    Json(input): Json<UpdateIssueRequest>,
) -> Result<Json<Issue>, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_named_actor(&actor)?;
    let current = load_issue(&state, &repository.id, number).await?;
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
            "Issue author or repository write permission is required.",
        ));
    }

    if let Some(title) = input.title.as_deref() {
        if title.trim().is_empty() || title.trim().len() > 200 {
            return Err(ApiError::bad_request(
                "Issue title must contain 1-200 bytes.",
            ));
        }
    }
    if let Some(body) = input.body.as_deref() {
        if !body.is_empty() {
            validate_body(body)?;
        }
    }
    let next_state = match input.state.as_deref() {
        None => None,
        Some("open") => Some("open"),
        Some("closed") => Some("closed"),
        Some(_) => return Err(ApiError::bad_request("Issue state must be open or closed.")),
    };

    let issue = sqlx::query_as::<_, Issue>(
        "UPDATE issues
         SET title = COALESCE(?, title),
             body = COALESCE(?, body),
             state = COALESCE(?, state),
             closed_at = CASE
                 WHEN ? = 'closed' THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHEN ? = 'open' THEN NULL
                 ELSE closed_at
             END,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE repository_id = ? AND number = ?
         RETURNING id, number, title, body, state, author_id, created_at, updated_at, closed_at, milestone_id",
    )
    .bind(input.title.map(|value| value.trim().to_owned()))
    .bind(input.body)
    .bind(next_state)
    .bind(next_state)
    .bind(next_state)
    .bind(&repository.id)
    .bind(number)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to update issue");
        ApiError::internal()
    })?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "issue_not_found", "Issue not found."))?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.update",
        "issue",
        Some(&issue.id),
    )
    .await;

    Ok(Json(issue))
}

pub(crate) async fn list_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
) -> Result<Json<Vec<IssueComment>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    let _issue = load_issue(&state, &repository.id, number).await?;
    let comments = sqlx::query_as::<_, IssueComment>(
        "SELECT c.id, i.number AS issue_number, c.author_id, c.body, c.created_at
         FROM issue_comments c
         JOIN issues i ON i.id = c.issue_id
         WHERE c.issue_id = (
             SELECT id FROM issues WHERE repository_id = ? AND number = ?
         )
         ORDER BY c.created_at ASC",
    )
    .bind(&repository.id)
    .bind(number)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list issue comments");
        ApiError::internal()
    })?;

    Ok(Json(comments))
}

pub(crate) async fn create_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
    Json(input): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<IssueComment>), ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_named_actor(&actor)?;
    let issue = load_issue(&state, &repository.id, number).await?;
    if issue.state != "open" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "issue_closed",
            "Comments cannot be added to a closed issue.",
        ));
    }
    if input.body.trim().is_empty() || input.body.len() > 10_000 {
        return Err(ApiError::bad_request(
            "Comment body must contain 1-10000 bytes.",
        ));
    }

    let comment = sqlx::query_as::<_, IssueComment>(
        "INSERT INTO issue_comments (id, issue_id, author_id, body)
         VALUES (?, ?, ?, ?)
         RETURNING id,
                   (SELECT number FROM issues WHERE id = issue_comments.issue_id) AS issue_number,
                   author_id, body, created_at",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&issue.id)
    .bind(&actor.user_id)
    .bind(input.body.trim())
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to create issue comment");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.comment",
        "issue",
        Some(&issue.id),
    )
    .await;

    Ok((StatusCode::CREATED, Json(comment)))
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Label {
    id: String,
    name: String,
    color: String,
    description: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Milestone {
    id: String,
    number: i64,
    title: String,
    description: Option<String>,
    state: String,
    due_on: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateLabelRequest {
    name: String,
    color: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateMilestoneRequest {
    title: String,
    description: Option<String>,
    due_on: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssignMilestoneRequest {
    number: i64,
}

async fn require_repository_write(
    state: &AppState,
    actor: &crate::auth::Actor,
    repository: &RepositoryContext,
) -> Result<(), ApiError> {
    if !crate::auth::can_access_repository(
        state,
        actor,
        &repository.id,
        &repository.owner,
        &repository.visibility,
        true,
    )
    .await
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Repository write permission is required.",
        ));
    }
    require_named_actor(actor)
}

pub(crate) async fn list_labels(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
) -> Result<Json<Vec<Label>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    let labels = sqlx::query_as::<_, Label>(
        "SELECT id, name, color, description, created_at
         FROM labels WHERE repository_id = ? ORDER BY name COLLATE NOCASE",
    )
    .bind(&repository.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list repository labels");
        ApiError::internal()
    })?;
    Ok(Json(labels))
}

pub(crate) async fn create_label(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Json(input): Json<CreateLabelRequest>,
) -> Result<(StatusCode, Json<Label>), ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_repository_write(&state, &actor, &repository).await?;

    let label_name = input.name.trim();
    if label_name.is_empty() || label_name.len() > 50 {
        return Err(ApiError::bad_request("Label name must contain 1-50 bytes."));
    }
    let color = input.color.unwrap_or_else(|| "D7FF38".to_owned());
    let color = color.trim().trim_start_matches('#');
    if color.len() != 6 || !color.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request(
            "Label color must be a six-digit hexadecimal value.",
        ));
    }
    if input
        .description
        .as_ref()
        .is_some_and(|value| value.len() > 500)
    {
        return Err(ApiError::bad_request(
            "Label description must be 500 bytes or fewer.",
        ));
    }

    let label = sqlx::query_as::<_, Label>(
        "INSERT INTO labels (id, repository_id, name, color, description)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id, name, color, description, created_at",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&repository.id)
    .bind(label_name)
    .bind(color.to_ascii_uppercase())
    .bind(input.description)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|db| db.is_unique_violation())
        {
            ApiError::new(
                StatusCode::CONFLICT,
                "label_exists",
                "A label with this name already exists.",
            )
        } else {
            error!(%error, "failed to create repository label");
            ApiError::internal()
        }
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "label.create",
        "label",
        Some(&label.id),
    )
    .await;
    Ok((StatusCode::CREATED, Json(label)))
}

pub(crate) async fn list_issue_labels(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
) -> Result<Json<Vec<Label>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    let issue = load_issue(&state, &repository.id, number).await?;
    let labels = sqlx::query_as::<_, Label>(
        "SELECT l.id, l.name, l.color, l.description, l.created_at
         FROM labels l
         INNER JOIN issue_labels il ON il.label_id = l.id
         WHERE il.issue_id = ?
         ORDER BY l.name COLLATE NOCASE",
    )
    .bind(&issue.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list issue labels");
        ApiError::internal()
    })?;
    Ok(Json(labels))
}

pub(crate) async fn add_issue_label(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number, label_id)): AxumPath<(String, String, i64, String)>,
) -> Result<StatusCode, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_repository_write(&state, &actor, &repository).await?;
    let issue = load_issue(&state, &repository.id, number).await?;

    let result = sqlx::query(
        "INSERT INTO issue_labels (issue_id, label_id, added_by)
         SELECT ?, id, ? FROM labels WHERE id = ? AND repository_id = ?
         ON CONFLICT(issue_id, label_id) DO NOTHING",
    )
    .bind(&issue.id)
    .bind(&actor.user_id)
    .bind(&label_id)
    .bind(&repository.id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to add issue label");
        ApiError::internal()
    })?;
    if result.rows_affected() == 0 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM labels WHERE id = ? AND repository_id = ?)",
        )
        .bind(&label_id)
        .bind(&repository.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|error| {
            error!(%error, "failed to verify repository label");
            ApiError::internal()
        })?;
        if !exists {
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "label_not_found",
                "Label not found.",
            ));
        }
    }

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.label.add",
        "issue",
        Some(&issue.id),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn remove_issue_label(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number, label_id)): AxumPath<(String, String, i64, String)>,
) -> Result<StatusCode, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_repository_write(&state, &actor, &repository).await?;
    let issue = load_issue(&state, &repository.id, number).await?;

    sqlx::query(
        "DELETE FROM issue_labels
         WHERE issue_id = ? AND label_id IN (
             SELECT id FROM labels WHERE id = ? AND repository_id = ?
         )",
    )
    .bind(&issue.id)
    .bind(&label_id)
    .bind(&repository.id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to remove issue label");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.label.remove",
        "issue",
        Some(&issue.id),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_milestones(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
) -> Result<Json<Vec<Milestone>>, ApiError> {
    let (_actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    let milestones = sqlx::query_as::<_, Milestone>(
        "SELECT id, number, title, description, state, due_on, created_at, updated_at
         FROM milestones WHERE repository_id = ? ORDER BY number DESC",
    )
    .bind(&repository.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list repository milestones");
        ApiError::internal()
    })?;
    Ok(Json(milestones))
}

pub(crate) async fn create_milestone(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Json(input): Json<CreateMilestoneRequest>,
) -> Result<(StatusCode, Json<Milestone>), ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_repository_write(&state, &actor, &repository).await?;

    let title = input.title.trim();
    if title.is_empty() || title.len() > 100 {
        return Err(ApiError::bad_request(
            "Milestone title must contain 1-100 bytes.",
        ));
    }
    if input
        .description
        .as_ref()
        .is_some_and(|value| value.len() > 5000)
    {
        return Err(ApiError::bad_request(
            "Milestone description must be 5000 bytes or fewer.",
        ));
    }
    if input.due_on.as_ref().is_some_and(|value| {
        value.len() != 10
            || !value.as_bytes().iter().enumerate().all(|(i, b)| {
                if matches!(i, 4 | 7) {
                    *b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            })
    }) {
        return Err(ApiError::bad_request(
            "Milestone due date must use YYYY-MM-DD.",
        ));
    }

    let milestone = sqlx::query_as::<_, Milestone>(
        "INSERT INTO milestones (id, repository_id, number, title, description, due_on)
         VALUES (?, ?, (SELECT COALESCE(MAX(number), 0) + 1 FROM milestones WHERE repository_id = ?), ?, ?, ?)
         RETURNING id, number, title, description, state, due_on, created_at, updated_at",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&repository.id)
    .bind(&repository.id)
    .bind(title)
    .bind(input.description)
    .bind(input.due_on)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to create repository milestone");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "milestone.create",
        "milestone",
        Some(&milestone.id),
    )
    .await;
    Ok((StatusCode::CREATED, Json(milestone)))
}

pub(crate) async fn assign_issue_milestone(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
    Json(input): Json<AssignMilestoneRequest>,
) -> Result<StatusCode, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_repository_write(&state, &actor, &repository).await?;
    let issue = load_issue(&state, &repository.id, number).await?;
    let milestone_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM milestones WHERE repository_id = ? AND number = ? AND state = 'open'",
    )
    .bind(&repository.id)
    .bind(input.number)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to resolve milestone");
        ApiError::internal()
    })?
    .ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "milestone_not_found",
            "Open milestone not found.",
        )
    })?;

    sqlx::query(
        "UPDATE issues SET milestone_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?",
    )
    .bind(&milestone_id)
    .bind(&issue.id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to assign milestone");
        ApiError::internal()
    })?;
    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.milestone.assign",
        "issue",
        Some(&issue.id),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn clear_issue_milestone(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name, number)): AxumPath<(String, String, i64)>,
) -> Result<StatusCode, ApiError> {
    let (actor, repository) = repository_context(&state, &headers, &owner, &name).await?;
    require_repository_write(&state, &actor, &repository).await?;
    let issue = load_issue(&state, &repository.id, number).await?;
    sqlx::query(
        "UPDATE issues SET milestone_id = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?",
    )
    .bind(&issue.id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to clear issue milestone");
        ApiError::internal()
    })?;
    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "issue.milestone.clear",
        "issue",
        Some(&issue.id),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
