// The shared helper module is compiled into separate binaries; each uses a different subset.
#[allow(dead_code)]
#[path = "ssh_common/mod.rs"]
mod ssh_common;

use std::env;

use anyhow::Result;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(error) = run().await {
        eprintln!("R!OTGIT authorized-keys lookup failed: {error}");
    }
    Ok(())
}

async fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "git" {
        return Ok(());
    }

    let fingerprint = &args[2];
    if !fingerprint.starts_with("SHA256:")
        || !fingerprint[7..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
    {
        return Ok(());
    }

    let pool: SqlitePool = ssh_common::open_pool().await?;
    let key = sqlx::query_as::<_, (String, String, String)>(
        "SELECT k.key_type, k.key_blob, k.comment
         FROM ssh_keys k
         JOIN users u ON u.id = k.user_id
         WHERE k.fingerprint = ? AND u.status = 'active'
         LIMIT 1",
    )
    .bind(fingerprint)
    .fetch_optional(&pool)
    .await?;

    if let Some((key_type, key_blob, comment)) = key {
        let safe_comment = comment
            .chars()
            .filter(|character| !character.is_control())
            .take(256)
            .collect::<String>();
        println!("restrict {key_type} {key_blob} {safe_comment}");
    }

    Ok(())
}
