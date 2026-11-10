use std::{
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{
    FromRow, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct SshActor {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct SshRepository {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub visibility: String,
    pub state: String,
    pub storage_key: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct BranchRule {
    pub branch_name: String,
    pub require_pull_request: bool,
    pub prevent_force_push: bool,
    pub prevent_deletion: bool,
}

pub fn database_path() -> Result<PathBuf> {
    let value = env::var("RIOTGIT_DATABASE_PATH")
        .unwrap_or_else(|_| "/var/lib/riotgit/riotgit.db".to_owned());
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        anyhow::bail!("RIOTGIT_DATABASE_PATH must be absolute");
    }
    Ok(path)
}

pub async fn open_pool() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(database_path()?)
        .create_if_missing(false)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    Ok(SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?)
}

pub fn git_binary() -> String {
    env::var("RIOTGIT_GIT_BIN").unwrap_or_else(|_| "git".to_owned())
}

pub fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value != "."
        && value != ".."
        && !value.ends_with('.')
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
}

pub fn key_fingerprint(key_type: &str, encoded_blob: &str) -> Result<String> {
    if !matches!(
        key_type,
        "ssh-ed25519"
            | "ecdsa-sha2-nistp256"
            | "sk-ssh-ed25519@openssh.com"
            | "sk-ecdsa-sha2-nistp256@openssh.com"
            | "ssh-rsa"
    ) {
        anyhow::bail!("Unsupported SSH key type");
    }

    let blob = STANDARD
        .decode(encoded_blob)
        .context("Malformed SSH public key base64")?;
    if blob.len() < 8 {
        anyhow::bail!("SSH public key blob is too short");
    }

    let length = u32::from_be_bytes(blob[0..4].try_into()?) as usize;
    if length != key_type.len() || blob.get(4..4 + length) != Some(key_type.as_bytes()) {
        anyhow::bail!("SSH public key type does not match its blob");
    }

    Ok(format!(
        "SHA256:{}",
        STANDARD_NO_PAD.encode(Sha256::digest(blob))
    ))
}

pub fn fingerprint_from_auth_info(path: &Path) -> Result<String> {
    let metadata =
        std::fs::metadata(path).context("SSH authentication info is unavailable")?;
    if !metadata.is_file() || metadata.len() > 16 * 1024 {
        anyhow::bail!("SSH authentication info file is invalid");
    }

    let contents = std::fs::read_to_string(path)?;
    let mut fingerprints = Vec::new();

    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("publickey") {
            continue;
        }
        let key_type = fields
            .next()
            .context("Missing SSH key type in auth info")?;
        let encoded_blob = fields
            .next()
            .context("Missing SSH key blob in auth info")?;
        fingerprints.push(key_fingerprint(key_type, encoded_blob)?);
    }

    fingerprints.sort();
    fingerprints.dedup();
    if fingerprints.len() != 1 {
        anyhow::bail!("Expected exactly one public key in SSH authentication info");
    }

    Ok(fingerprints.remove(0))
}

pub async fn actor_for_fingerprint(
    pool: &SqlitePool,
    fingerprint: &str,
) -> Result<Option<SshActor>> {
    Ok(sqlx::query_as::<_, SshActor>(
        "SELECT u.id, u.username, u.role
         FROM ssh_keys k
         JOIN users u ON u.id = k.user_id
         WHERE k.fingerprint = ? AND u.status = 'active'
         LIMIT 1",
    )
    .bind(fingerprint)
    .fetch_optional(pool)
    .await?)
}

pub async fn repository(
    pool: &SqlitePool,
    owner: &str,
    name: &str,
) -> Result<Option<SshRepository>> {
    Ok(sqlx::query_as::<_, SshRepository>(
        "SELECT id, owner, name, visibility, state, storage_key
         FROM repositories
         WHERE owner = ? COLLATE NOCASE AND name = ? COLLATE NOCASE
         LIMIT 1",
    )
    .bind(owner)
    .bind(name)
    .fetch_optional(pool)
    .await?)
}

pub async fn can_access(
    pool: &SqlitePool,
    actor: &SshActor,
    repository: &SshRepository,
    write: bool,
) -> Result<bool> {
    if actor.role == "admin" || actor.username.eq_ignore_ascii_case(&repository.owner) {
        return Ok(true);
    }
    if !write && matches!(repository.visibility.as_str(), "public" | "internal") {
        return Ok(true);
    }

    let permission = sqlx::query_scalar::<_, String>(
        "SELECT permission FROM repository_access
         WHERE repository_id = ? AND user_id = ?",
    )
    .bind(&repository.id)
    .bind(&actor.id)
    .fetch_optional(pool)
    .await?;

    Ok(match permission {
        Some(value) if write => matches!(value.as_str(), "write" | "admin"),
        Some(_) => true,
        None => false,
    })
}

pub fn storage_path(storage_key: &str) -> Result<PathBuf> {
    if storage_key.len() != 36
        || !storage_key.ends_with(".git")
        || !storage_key[..32]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        anyhow::bail!("Repository storage key is invalid");
    }

    let configured = env::var("RIOTGIT_STORAGE_ROOT")
        .unwrap_or_else(|_| "/var/lib/riotgit/repositories".to_owned());
    let root = std::fs::canonicalize(configured)?;
    let path = std::fs::canonicalize(root.join(storage_key))?;
    if !path.starts_with(&root) || !path.is_dir() {
        anyhow::bail!("Repository storage path is invalid");
    }

    Ok(path)
}

#[derive(Serialize)]
struct AuditMetadata<'a> {
    transport: &'a str,
    operation: &'a str,
}

pub async fn audit(
    pool: &SqlitePool,
    actor_id: &str,
    action: &str,
    repository_id: &str,
    operation: &str,
) -> Result<()> {
    let metadata = serde_json::to_string(&AuditMetadata {
        transport: "ssh",
        operation,
    })?;

    sqlx::query(
        "INSERT INTO audit_events (id, actor_user_id, action, resource_type, resource_id, metadata_json)
         VALUES (?, ?, ?, 'repository', ?, ?)",
    )
    .bind(Uuid::new_v4().simple().to_string())
    .bind(actor_id)
    .bind(action)
    .bind(repository_id)
    .bind(metadata)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn branch_rule(
    pool: &SqlitePool,
    repository_id: &str,
    branch_name: &str,
) -> Result<Option<BranchRule>> {
    Ok(sqlx::query_as::<_, BranchRule>(
        "SELECT branch_name, require_pull_request, prevent_force_push, prevent_deletion
         FROM branch_protection_rules
         WHERE repository_id = ? AND branch_name = ?",
    )
    .bind(repository_id)
    .bind(branch_name)
    .fetch_optional(pool)
    .await?)
}
