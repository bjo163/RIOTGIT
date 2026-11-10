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

#[derive(Debug, FromRow, Serialize)]
pub(crate) struct BranchProtectionRule {
    id: String,
    branch_name: String,
    require_pull_request: bool,
    prevent_force_push: bool,
    prevent_deletion: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct RepositoryContext {
    id: String,
    owner: String,
    visibility: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SetBranchProtectionRequest {
    branch_name: String,
    #[serde(default = "default_true")]
    require_pull_request: bool,
    #[serde(default = "default_true")]
    prevent_force_push: bool,
    #[serde(default = "default_true")]
    prevent_deletion: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteBranchProtectionQuery {
    branch_name: String,
}

fn default_true() -> bool {
    true
}

pub(crate) fn valid_branch_name(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 200
        || value.starts_with('-')
        || value.ends_with('.')
        || value.ends_with('/')
        || value.contains("..")
        || value.contains("//")
        || value.contains("@{")
        || value
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.') || part.ends_with(".lock"))
    {
        return false;
    }

    value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '/' | '-' | '_' | '.')
    })
}

async fn repository_context(
    state: &AppState,
    owner: &str,
    name: &str,
) -> Result<RepositoryContext, ApiError> {
    sqlx::query_as::<_, RepositoryContext>(
        "SELECT id, owner, visibility FROM repositories
         WHERE owner = ? AND name = ? AND state = 'ready'",
    )
    .bind(owner)
    .bind(name)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to resolve repository for branch protection");
        ApiError::internal()
    })?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found", "Repository not found."))
}

pub(crate) async fn list_rules(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
) -> Result<Json<Vec<BranchProtectionRule>>, ApiError> {
    let actor = crate::auth::authenticate(&headers, &state).await?;
    let repository = repository_context(&state, &owner, &name).await?;
    if !crate::auth::can_access_repository(
        &state,
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

    let rules = sqlx::query_as::<_, BranchProtectionRule>(
        "SELECT id, branch_name, require_pull_request, prevent_force_push,
                prevent_deletion, created_at, updated_at
         FROM branch_protection_rules
         WHERE repository_id = ?
         ORDER BY branch_name COLLATE NOCASE",
    )
    .bind(&repository.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to list branch protection rules");
        ApiError::internal()
    })?;

    Ok(Json(rules))
}

pub(crate) async fn set_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Json(input): Json<SetBranchProtectionRequest>,
) -> Result<(StatusCode, Json<BranchProtectionRule>), ApiError> {
    let actor = crate::auth::authenticate(&headers, &state).await?;
    let repository = repository_context(&state, &owner, &name).await?;
    if !crate::auth::can_manage_repository(&state, &actor, &repository.id, &repository.owner).await
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Repository admin permission is required to manage branch protection.",
        ));
    }
    if !valid_branch_name(&input.branch_name) {
        return Err(ApiError::bad_request("Branch name is invalid."));
    }
    if !input.require_pull_request && !input.prevent_force_push && !input.prevent_deletion {
        return Err(ApiError::bad_request(
            "A branch protection rule must enable at least one protection.",
        ));
    }

    let rule = sqlx::query_as::<_, BranchProtectionRule>(
        "INSERT INTO branch_protection_rules (
             id, repository_id, branch_name, require_pull_request,
             prevent_force_push, prevent_deletion, created_by
         )
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(repository_id, branch_name) DO UPDATE SET
             require_pull_request = excluded.require_pull_request,
             prevent_force_push = excluded.prevent_force_push,
             prevent_deletion = excluded.prevent_deletion,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         RETURNING id, branch_name, require_pull_request, prevent_force_push,
                   prevent_deletion, created_at, updated_at",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(&repository.id)
    .bind(&input.branch_name)
    .bind(input.require_pull_request)
    .bind(input.prevent_force_push)
    .bind(input.prevent_deletion)
    .bind((actor.user_id != "system").then_some(actor.user_id.as_str()))
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to save branch protection rule");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "branch_protection.set",
        "repository",
        Some(&repository.id),
    )
    .await;

    Ok((StatusCode::OK, Json(rule)))
}

pub(crate) async fn delete_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((owner, name)): AxumPath<(String, String)>,
    Query(query): Query<DeleteBranchProtectionQuery>,
) -> Result<StatusCode, ApiError> {
    let actor = crate::auth::authenticate(&headers, &state).await?;
    let repository = repository_context(&state, &owner, &name).await?;
    if !crate::auth::can_manage_repository(&state, &actor, &repository.id, &repository.owner).await
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Repository admin permission is required to manage branch protection.",
        ));
    }
    if !valid_branch_name(&query.branch_name) {
        return Err(ApiError::bad_request("Branch name is invalid."));
    }

    sqlx::query("DELETE FROM branch_protection_rules WHERE repository_id = ? AND branch_name = ?")
        .bind(&repository.id)
    .bind(&query.branch_name)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, "failed to delete branch protection rule");
        ApiError::internal()
    })?;

    crate::auth::record_audit(
        &state.pool,
        Some(&actor.user_id),
        "branch_protection.delete",
        "repository",
        Some(&repository.id),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::valid_branch_name;

    #[test]
    fn accepts_safe_branch_names() {
        assert!(valid_branch_name("main"));
        assert!(valid_branch_name("feature/branch-protection"));
        assert!(valid_branch_name("release-1.0"));
    }

    #[test]
    fn rejects_unsafe_branch_names() {
        assert!(!valid_branch_name(""));
        assert!(!valid_branch_name("../outside"));
        assert!(!valid_branch_name("feature//name"));
        assert!(!valid_branch_name("feature/@{base}"));
        assert!(!valid_branch_name(".hidden"));
        assert!(!valid_branch_name("main.lock"));
        assert!(!valid_branch_name("main;command"));
    }
}
