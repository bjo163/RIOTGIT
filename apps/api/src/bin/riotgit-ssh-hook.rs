mod ssh_common;

use std::{
    env,
    io::{self, BufRead},
    process::Command,
};

use anyhow::{Context, Result, bail};
use tokio::runtime::Runtime;

fn main() {
    if let Err(error) = Runtime::new().expect("Tokio runtime").block_on(run()) {
        eprintln!("R!OTGIT branch policy rejected the push: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let repository_id = env::var("RIOTGIT_SSH_REPOSITORY_ID")
        .context("Repository security context is missing")?;
    let actor_id =
        env::var("RIOTGIT_SSH_ACTOR_ID").context("Actor security context is missing")?;
    let git_dir =
        env::var("GIT_DIR").context("Git receive-pack did not provide GIT_DIR")?;
    let git_bin = ssh_common::git_binary();
    let pool = ssh_common::open_pool().await?;

    let stdin = io::stdin();
    let mut updates = Vec::new();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.len() > 512 {
            bail!("Receive-pack ref update line is too long");
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 3 {
            bail!("Receive-pack ref update is malformed");
        }

        let (old_oid, new_oid, reference) = (fields[0], fields[1], fields[2]);
        if !valid_oid(old_oid) || !valid_oid(new_oid) {
            bail!("Receive-pack object ID is malformed");
        }

        updates.push((old_oid.to_owned(), new_oid.to_owned(), reference.to_owned()));
    }

    for (old_oid, new_oid, reference) in updates {
        let Some(branch_name) = reference.strip_prefix("refs/heads/") else {
            continue;
        };
        let Some(rule) =
            ssh_common::branch_rule(&pool, &repository_id, branch_name).await?
        else {
            continue;
        };

        let old_zero = old_oid.bytes().all(|byte| byte == b'0');
        let new_zero = new_oid.bytes().all(|byte| byte == b'0');

        if rule.require_pull_request {
            record_denial(
                &pool,
                &actor_id,
                &repository_id,
                branch_name,
                "pull_request_required",
            )
            .await;
            bail!(
                "Direct pushes to protected branch '{branch_name}' are blocked; open a pull request."
            );
        }

        if new_zero && rule.prevent_deletion {
            record_denial(
                &pool,
                &actor_id,
                &repository_id,
                branch_name,
                "branch_deletion_blocked",
            )
            .await;
            bail!("Deletion of protected branch '{branch_name}' is blocked.");
        }

        if !old_zero && !new_zero && rule.prevent_force_push {
            let status = Command::new(&git_bin)
                .arg("--git-dir")
                .arg(&git_dir)
                .arg("merge-base")
                .arg("--is-ancestor")
                .arg(&old_oid)
                .arg(&new_oid)
                .status()
                .context("Could not validate fast-forward update")?;
            if !status.success() {
                record_denial(
                    &pool,
                    &actor_id,
                    &repository_id,
                    branch_name,
                    "force_push_blocked",
                )
                .await;
                bail!("Non-fast-forward update to protected branch '{branch_name}' is blocked.");
            }
        }
    }

    Ok(())
}

fn valid_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

async fn record_denial(
    pool: &sqlx::SqlitePool,
    actor_id: &str,
    repository_id: &str,
    branch_name: &str,
    reason: &str,
) {
    let metadata = serde_json::json!({
        "transport": "ssh",
        "branch": branch_name,
        "reason": reason
    })
    .to_string();

    let _ = sqlx::query(
        "INSERT INTO audit_events (id, actor_user_id, action, resource_type, resource_id, metadata_json)
         VALUES (?, ?, 'git.ssh.branch_policy_denied', 'repository', ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(actor_id)
    .bind(repository_id)
    .bind(metadata)
    .execute(pool)
    .await;
}
