mod ssh_common;

use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy)]
enum GitService {
    UploadPack,
    ReceivePack,
}

impl GitService {
    fn write(self) -> bool {
        matches!(self, Self::ReceivePack)
    }

    fn operation(self) -> &'static str {
        match self {
            Self::UploadPack => "git-upload-pack",
            Self::ReceivePack => "git-receive-pack",
        }
    }

    fn git_subcommand(self) -> &'static str {
        match self {
            Self::UploadPack => "upload-pack",
            Self::ReceivePack => "receive-pack",
        }
    }
}

fn parse_original_command(value: &str) -> Result<(GitService, String, String)> {
    if value.len() > 1024
        || value
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\0' | '\\' | ';' | '$' | '|' | '&'))
    {
        bail!("Invalid Git command");
    }

    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 2 {
        bail!("Only Git upload-pack and receive-pack are supported");
    }

    let service = match parts[0] {
        "git-upload-pack" => GitService::UploadPack,
        "git-receive-pack" => GitService::ReceivePack,
        _ => bail!("Only Git upload-pack and receive-pack are supported"),
    };

    let raw_path = parts[1];
    let path = if raw_path.len() >= 2
        && ((raw_path.starts_with('\'') && raw_path.ends_with('\''))
            || (raw_path.starts_with('"') && raw_path.ends_with('"')))
    {
        &raw_path[1..raw_path.len() - 1]
    } else {
        raw_path
    };

    if path.starts_with('/')
        || path.starts_with('~')
        || path
            .chars()
            .any(|character| matches!(character, '\'' | '"' | '\\'))
        || !path.ends_with(".git")
    {
        bail!("Repository path is invalid");
    }

    let without_suffix = &path[..path.len() - 4];
    let mut segments = without_suffix.split('/');
    let owner = segments.next().unwrap_or_default();
    let name = segments.next().unwrap_or_default();
    if segments.next().is_some()
        || !ssh_common::valid_segment(owner)
        || !ssh_common::valid_segment(name)
    {
        bail!("Repository path is invalid");
    }

    Ok((service, owner.to_owned(), name.to_owned()))
}

fn main() {
    if let Err(error) = Runtime::new().expect("Tokio runtime").block_on(run()) {
        eprintln!("R!OTGIT SSH: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let original_command =
        env::var("SSH_ORIGINAL_COMMAND").context("Interactive shell access is disabled")?;
    let auth_file = env::var("SSH_USER_AUTH")
        .context("OpenSSH authentication information is unavailable")?;
    let fingerprint = ssh_common::fingerprint_from_auth_info(Path::new(&auth_file))?;
    let (service, owner, name) = parse_original_command(&original_command)?;

    let pool = ssh_common::open_pool().await?;
    let actor = ssh_common::actor_for_fingerprint(&pool, &fingerprint)
        .await?
        .context("SSH key is no longer registered or account is disabled")?;
    let repository = ssh_common::repository(&pool, &owner, &name)
        .await?
        .context("Repository not found")?;

    if repository.state != "ready" {
        bail!("Repository is not ready");
    }
    if !ssh_common::can_access(&pool, &actor, &repository, service.write()).await? {
        bail!("Repository permission denied");
    }

    let repository_path = ssh_common::storage_path(&repository.storage_key)?;
    let git_bin = ssh_common::git_binary();
    let storage_root = env::var("RIOTGIT_STORAGE_ROOT")
        .unwrap_or_else(|_| "/var/lib/riotgit/repositories".to_owned());

    let mut command = Command::new(&git_bin);
    command
        .arg(service.git_subcommand())
        .arg(repository_path)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("HOME", "/var/lib/riotgit")
        .env(
            "RIOTGIT_DATABASE_PATH",
            ssh_common::database_path()?,
        )
        .env("RIOTGIT_STORAGE_ROOT", storage_root)
        .env("RIOTGIT_SSH_REPOSITORY_ID", &repository.id)
        .env("RIOTGIT_SSH_ACTOR_ID", &actor.id)
        .env("RIOTGIT_GIT_BIN", &git_bin)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "/usr/local/lib/riotgit/hooks")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Ok(protocol) = env::var("GIT_PROTOCOL") {
        if protocol.len() <= 64 && protocol.bytes().all(|byte| byte.is_ascii_graphic()) {
            command.env("GIT_PROTOCOL", protocol);
        }
    }

    let status = command
        .status()
        .context("Could not execute native Git transport")?;
    let action = if status.success() {
        "git.ssh.success"
    } else {
        "git.ssh.failure"
    };

    if let Err(error) =
        ssh_common::audit(&pool, &actor.id, action, &repository.id, service.operation()).await
    {
        eprintln!("R!OTGIT SSH audit write failed: {error}");
    }

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GitService, parse_original_command};

    #[test]
    fn accepts_only_supported_git_services_and_repository_paths() {
        let (service, owner, name) =
            parse_original_command("git-upload-pack 'builder/project.git'").unwrap();
        assert!(matches!(service, GitService::UploadPack));
        assert_eq!(owner, "builder");
        assert_eq!(name, "project");

        let (service, _, _) =
            parse_original_command("git-receive-pack 'builder/project.git'").unwrap();
        assert!(matches!(service, GitService::ReceivePack));
    }

    #[test]
    fn rejects_shells_commands_and_path_traversal() {
        assert!(parse_original_command("bash").is_err());
        assert!(parse_original_command("git-upload-pack '../outside.git'").is_err());
        assert!(parse_original_command("git-upload-pack 'builder/project.git;id'").is_err());
        assert!(parse_original_command("git-upload-pack '/tmp/project.git'").is_err());
    }
}
