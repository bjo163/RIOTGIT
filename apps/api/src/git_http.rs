use std::{env, process::Stdio, time::Duration};

use anyhow::{Result, anyhow};

use axum::{
    body::{Body, to_bytes},
    extract::{Path as AxumPath, Request, State},
    http::{
        HeaderMap, HeaderName, HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE},
    },
    response::Response,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use tokio::{fs, io::AsyncWriteExt, process::Command, time::timeout};
use tracing::{error, warn};

use crate::AppState;

async fn authenticated(headers: &HeaderMap, state: &AppState) -> Option<crate::auth::Actor> {
    let auth = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = if let Some(token) = auth.strip_prefix("Bearer ") {
        token.to_owned()
    } else {
        let encoded = auth.strip_prefix("Basic ")?;
        let decoded = STANDARD.decode(encoded).ok()?;
        let credentials = std::str::from_utf8(&decoded).ok()?;
        let (_, password) = credentials.split_once(':')?;
        password.to_owned()
    };

    crate::auth::actor_from_token(&token, state).await
}

fn plain_response(status: StatusCode, message: &'static str) -> Response {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(format!("{message}\n")))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn unauthorized() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            WWW_AUTHENTICATE,
            "Basic realm=\"R!OTGIT Git\", charset=\"UTF-8\"",
        )
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from("Git authentication required.\n"))
        .unwrap_or_else(|_| Response::new(Body::empty()))
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

fn valid_operation(method: &Method, path: &str, query: &str) -> bool {
    (method == Method::GET
        && path == "info/refs"
        && matches!(
            query,
            "service=git-upload-pack" | "service=git-receive-pack"
        ))
        || (method == Method::POST
            && query.is_empty()
            && matches!(path, "git-upload-pack" | "git-receive-pack"))
}


#[derive(Debug, Clone)]
struct RefUpdate {
    old_oid: String,
    new_oid: String,
    branch_name: String,
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_zero_oid(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}

fn parse_receive_pack_updates(body: &[u8]) -> Result<Vec<RefUpdate>, ()> {
    let mut offset = 0usize;
    let mut updates = Vec::new();

    while offset + 4 <= body.len() {
        let length_text = std::str::from_utf8(&body[offset..offset + 4]).map_err(|_| ())?;
        if !length_text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(());
        }
        let packet_length = usize::from_str_radix(length_text, 16).map_err(|_| ())?;
        offset += 4;

        match packet_length {
            0 | 2 => break,
            1 => continue,
            value if value < 4 => return Err(()),
            value => {
                let payload_length = value - 4;
                let end = offset.checked_add(payload_length).ok_or(())?;
                if end > body.len() {
                    return Err(());
                }

                let packet = &body[offset..end];
                offset = end;
                let command = packet
                    .split(|byte| *byte == 0)
                    .next()
                    .unwrap_or_default()
                    .strip_suffix(b"\n")
                    .unwrap_or_else(|| {
                        packet
                            .split(|byte| *byte == 0)
                            .next()
                            .unwrap_or_default()
                    });
                let Ok(line) = std::str::from_utf8(command) else {
                    continue;
                };
                let mut fields = line.split_whitespace();
                let (Some(old_oid), Some(new_oid), Some(reference)) =
                    (fields.next(), fields.next(), fields.next())
                else {
                    continue;
                };
                if !valid_object_id(old_oid) || !valid_object_id(new_oid) {
                    continue;
                }
                let Some(branch_name) = reference.strip_prefix("refs/heads/") else {
                    continue;
                };
                if !crate::branch_protection::valid_branch_name(branch_name) {
                    continue;
                }

                updates.push(RefUpdate {
                    old_oid: old_oid.to_owned(),
                    new_oid: new_oid.to_owned(),
                    branch_name: branch_name.to_owned(),
                });
            }
        }
    }

    Ok(updates)
}

async fn enforce_branch_protection(
    state: &AppState,
    repository_id: &str,
    storage_path: &std::path::Path,
    body: &[u8],
) -> Result<(), StatusCode> {
    let updates = parse_receive_pack_updates(body).map_err(|_| StatusCode::BAD_REQUEST)?;
    if updates.is_empty() {
        return Ok(());
    }

    let rules = sqlx::query_as::<_, (String, bool, bool, bool)>(
        "SELECT branch_name, require_pull_request, prevent_force_push, prevent_deletion
         FROM branch_protection_rules WHERE repository_id = ?",
    )
    .bind(repository_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        error!(%error, repository_id, "failed to load branch protection rules");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    for update in updates {
        let Some((_, require_pull_request, prevent_force_push, prevent_deletion)) =
            rules.iter().find(|rule| rule.0 == update.branch_name)
        else {
            continue;
        };

        if *require_pull_request {
            warn!(
                repository_id,
                branch = %update.branch_name,
                "blocked direct push to branch requiring a pull request"
            );
            return Err(StatusCode::FORBIDDEN);
        }

        if *prevent_deletion && is_zero_oid(&update.new_oid) {
            warn!(
                repository_id,
                branch = %update.branch_name,
                "blocked deletion of protected branch"
            );
            return Err(StatusCode::FORBIDDEN);
        }

        if *prevent_force_push
            && !is_zero_oid(&update.old_oid)
            && !is_zero_oid(&update.new_oid)
        {
            let status = timeout(
                Duration::from_secs(5),
                Command::new(&state.git_bin)
                    .arg("--git-dir")
                    .arg(storage_path)
                    .arg("merge-base")
                    .arg("--is-ancestor")
                    .arg(&update.old_oid)
                    .arg(&update.new_oid)
                    .env_clear()
                    .env("PATH", env::var("PATH").unwrap_or_default())
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", &state.empty_git_config)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .status(),
            )
            .await;

            match status {
                Ok(Ok(status)) if status.success() => {}
                _ => {
                    warn!(
                        repository_id,
                        branch = %update.branch_name,
                        "blocked non-fast-forward update to protected branch"
                    );
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
    }

    Ok(())
}

fn cgi_response(output: &[u8]) -> Result<Response> {
    let (header_end, separator_len) = output
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, 4))
        .or_else(|| {
            output
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|index| (index, 2))
        })
        .ok_or_else(|| anyhow!("missing CGI header separator"))?;

    let mut status = StatusCode::OK;
    let mut response_headers = Vec::new();

    for raw_line in output[..header_end].split(|byte| *byte == b'\n') {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        if line.is_empty() {
            continue;
        }
        let line = std::str::from_utf8(line)?;
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();

        if name.eq_ignore_ascii_case("status") {
            let code = value
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u16>().ok())
                .ok_or_else(|| anyhow!("invalid CGI status line"))?;
            status = StatusCode::from_u16(code)?;
            continue;
        }

        let header_name = HeaderName::from_bytes(name.as_bytes())?;
        if matches!(
            header_name.as_str(),
            "connection"
                | "keep-alive"
                | "proxy-authenticate"
                | "proxy-authorization"
                | "te"
                | "trailer"
                | "transfer-encoding"
                | "upgrade"
        ) {
            continue;
        }
        let header_value = HeaderValue::from_str(value)?;
        response_headers.push((header_name, header_value));
    }

    let body = output[header_end + separator_len..].to_vec();
    let mut builder = Response::builder().status(status);
    for (name, value) in response_headers {
        builder = builder.header(name, value);
    }
    builder.body(Body::from(body)).map_err(Into::into)
}

pub(crate) async fn transport(
    State(state): State<AppState>,
    AxumPath((owner, repo_path, service_path)): AxumPath<(String, String, String)>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();

    let Some(actor) = authenticated(&parts.headers, &state).await else {
        return unauthorized();
    };

    let Some(repo_name) = repo_path.strip_suffix(".git") else {
        return plain_response(StatusCode::NOT_FOUND, "Repository not found.");
    };
    let query = parts.uri.query().unwrap_or("");
    if !valid_segment(&owner)
        || !valid_segment(repo_name)
        || !valid_operation(&parts.method, &service_path, query)
    {
        warn!(method = %parts.method, owner = %owner, repo = %repo_name, service_path = %service_path, query = %query, "rejected unsupported Git smart HTTP request");
        return plain_response(
            StatusCode::BAD_REQUEST,
            "Unsupported Git smart HTTP request.",
        );
    }

    let repository = match sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id, owner, visibility, storage_key FROM repositories
         WHERE owner = ? AND name = ? AND state = 'ready'",
    )
    .bind(&owner)
    .bind(repo_name)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(repository)) => repository,
        Ok(None) => return plain_response(StatusCode::NOT_FOUND, "Repository not found."),
        Err(error) => {
            error!(%error, "failed to resolve Git HTTP repository");
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not resolve repository.",
            );
        }
    };
    let (repository_id, repository_owner, visibility, storage_key) = repository;
    let is_write = (parts.method == Method::GET && query == "service=git-receive-pack")
        || (parts.method == Method::POST && service_path == "git-receive-pack");
    if !crate::auth::can_access_repository(
        &state,
        &actor,
        &repository_id,
        &repository_owner,
        &visibility,
        is_write,
    )
    .await
    {
        return plain_response(StatusCode::NOT_FOUND, "Repository not found.");
    }

    let storage_path = state.storage_root.join(&storage_key);
    if fs::metadata(&storage_path).await.is_err() {
        warn!(%storage_key, "Git HTTP repository storage is missing");
        return plain_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Repository storage is unavailable.",
        );
    }

    const MAX_GIT_HTTP_BODY: usize = 64 * 1024 * 1024;
    let request_body = match to_bytes(body, MAX_GIT_HTTP_BODY).await {
        Ok(body) => body,
        Err(_) => {
            return plain_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Git request exceeds the configured 64 MiB request limit.",
            );
        }
    };

    if parts.method == Method::POST && service_path == "git-receive-pack" {
        if let Err(status) =
            enforce_branch_protection(&state, &repository_id, &storage_path, &request_body).await
        {
            let message = if status == StatusCode::FORBIDDEN {
                "Branch protection rejected this direct Git update."
            } else if status == StatusCode::BAD_REQUEST {
                "Malformed Git receive-pack command list."
            } else {
                "Branch protection could not be evaluated."
            };
            return plain_response(status, message);
        }
    }

    let path_info = format!("/{storage_key}/{service_path}");
    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let path_env = env::var("PATH").unwrap_or_default();

    let mut command = Command::new(&state.git_bin);
    command
        .arg("http-backend")
        .env_clear()
        .env("PATH", path_env)
        .env("GIT_PROJECT_ROOT", &state.storage_root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &state.empty_git_config)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("PATH_INFO", path_info)
        .env("REQUEST_METHOD", parts.method.as_str())
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", content_type)
        .env("CONTENT_LENGTH", request_body.len().to_string())
        .env("REMOTE_USER", &actor.username)
        .env("SERVER_PROTOCOL", "HTTP/1.1")
        .env("SERVER_NAME", "riotgit")
        .env("SERVER_PORT", "8787")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    for (source, target) in [
        ("git-protocol", "HTTP_GIT_PROTOCOL"),
        ("host", "HTTP_HOST"),
        ("user-agent", "HTTP_USER_AGENT"),
        ("accept", "HTTP_ACCEPT"),
    ] {
        if let Some(value) = parts
            .headers
            .get(source)
            .and_then(|value| value.to_str().ok())
        {
            command.env(target, value);
        }
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            error!(%error, "failed to start native Git HTTP backend");
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Git transport is unavailable.",
            );
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = stdin.write_all(&request_body).await {
            error!(%error, "failed to pass Git request body to backend");
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Git transport request failed.",
            );
        }
    }

    let output = match timeout(Duration::from_secs(120), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            error!(%error, "Git HTTP backend process failed");
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Git transport process failed.",
            );
        }
        Err(_) => {
            return plain_response(
                StatusCode::GATEWAY_TIMEOUT,
                "Git transport exceeded the 120 second execution limit.",
            );
        }
    };

    if !output.status.success() {
        warn!(status = ?output.status, "Git HTTP backend returned a failure status");
    }

    match cgi_response(&output.stdout) {
        Ok(response) => response,
        Err(error) => {
            error!(%error, "Git HTTP backend returned malformed CGI headers");
            plain_response(
                StatusCode::BAD_GATEWAY,
                "Git backend returned an invalid response.",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{valid_operation, valid_segment};
    use axum::http::Method;

    #[test]
    fn accepts_git_http_operations() {
        assert!(valid_operation(
            &Method::GET,
            "info/refs",
            "service=git-upload-pack"
        ));
        assert!(valid_operation(&Method::POST, "git-upload-pack", ""));
        assert!(valid_operation(&Method::POST, "git-receive-pack", ""));
    }

    #[test]
    fn rejects_unsupported_git_http_operations() {
        assert!(!valid_operation(
            &Method::GET,
            "info/refs",
            "service=git-upload-pack&other=value"
        ));
        assert!(!valid_operation(
            &Method::POST,
            "git-receive-pack",
            "other=value"
        ));
        assert!(!valid_operation(&Method::DELETE, "git-upload-pack", ""));
        assert!(!valid_segment("../outside"));
    }
}
