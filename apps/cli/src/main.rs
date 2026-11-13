use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Parser)]
#[command(
    name = "riotgit",
    version,
    about = "R!OTGIT — The Underground Code Network",
    long_about = "Operate your R!OTGIT server from the terminal. Your repositories stay standard Git repositories."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Authenticate against a R!OTGIT server and store a local session.
    Login {
        /// Server origin, for example https://git.example.com
        #[arg(long)]
        server: String,
        /// R!OTGIT username
        #[arg(long)]
        username: String,
    },
    /// Show the authenticated identity.
    Whoami,
    /// Check server readiness and the current authenticated session.
    Status,
    /// Repository operations.
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
    /// Manage the SSH public keys registered to your account.
    SshKey {
        #[command(subcommand)]
        command: SshKeyCommands,
    },
    /// Revoke the current session and remove local credentials.
    Logout,
}

#[derive(Debug, Subcommand)]
enum RepoCommands {
    /// List repositories visible to the current account.
    List,
    /// Create a repository under your own username.
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, default_value = "private", value_parser = ["private", "public", "internal"])]
        visibility: String,
    },
    /// Clone a repository using standard Git over HTTPS.
    Clone {
        owner: String,
        name: String,
        /// Target directory; defaults to the repository name.
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum SshKeyCommands {
    /// List SSH public keys registered to your account.
    List,
    /// Register a one-line OpenSSH public key file.
    Add {
        /// Path to a .pub file.
        path: PathBuf,
    },
    /// Revoke a key by its R!OTGIT key ID.
    Remove { id: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct LocalConfig {
    server: String,
    access_token: String,
    expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    access_token: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct UserSummary {
    username: String,
    display_name: String,
    role: String,
}

struct Api {
    config: LocalConfig,
    client: Client,
}

impl Api {
    fn load() -> Result<Self> {
        let path = config_path()?;
        let content = fs::read_to_string(&path).with_context(|| {
            format!(
                "No local session found at {}. Run riotgit login first.",
                path.display()
            )
        })?;
        let config: LocalConfig = serde_json::from_str(&content)
            .with_context(|| format!("Could not parse {}", path.display()))?;
        Ok(Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent(concat!("riotgit-cli/", env!("CARGO_PKG_VERSION")))
                .build()?,
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.config.server, path)
    }

    fn authorized(&self, request: RequestBuilder) -> RequestBuilder {
        request.bearer_auth(&self.config.access_token)
    }

    fn get(&self, path: &str) -> Result<Value> {
        self.send(self.authorized(self.client.get(self.endpoint(path))))
    }

    fn post(&self, path: &str, payload: &Value) -> Result<Value> {
        self.send(
            self.authorized(self.client.post(self.endpoint(path)))
                .json(payload),
        )
    }

    fn delete(&self, path: &str) -> Result<()> {
        let response = self
            .authorized(self.client.delete(self.endpoint(path)))
            .send()?;
        if !response.status().is_success() {
            return Err(response_error(response));
        }
        Ok(())
    }

    fn send(&self, request: RequestBuilder) -> Result<Value> {
        let response = request.send()?;
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if !status.is_success() {
            let message = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| {
                    value
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| format!("R!OTGIT API returned {status}."));
            bail!("{message}");
        }
        if body.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&body).context("R!OTGIT API returned invalid JSON")
    }

    fn me(&self) -> Result<UserSummary> {
        Ok(serde_json::from_value(self.get("/api/v1/auth/me")?)?)
    }

    fn health(&self) -> Result<String> {
        let response = self.client.get(self.endpoint("/health/ready")).send()?;
        if !response.status().is_success() {
            bail!("R!OTGIT server is not ready: {}", response.status());
        }
        let value: Value = response.json()?;
        Ok(value.get("status").and_then(Value::as_str).unwrap_or("unknown").to_owned())
    }
}

fn response_error(response: reqwest::blocking::Response) -> anyhow::Error {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("R!OTGIT API returned {status}."));
    anyhow::anyhow!(message)
}

fn config_path() -> Result<PathBuf> {
    let root =
        dirs::config_dir().context("Could not resolve the operating-system config directory")?;
    Ok(root.join("riotgit").join("config.json"))
}

fn write_config(config: &LocalConfig) -> Result<()> {
    let path = config_path()?;
    let parent = path
        .parent()
        .context("Config path has no parent directory")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join("config.json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(config)?)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }

    fs::rename(&temporary, &path)
        .with_context(|| format!("Could not write local session to {}", path.display()))?;
    Ok(())
}

fn normalize_server(value: &str) -> Result<String> {
    let parsed =
        reqwest::Url::parse(value.trim()).context("Server must be a valid absolute URL")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("Server must be an http(s) origin without embedded credentials, query, or fragment");
    }
    let mut normalized = parsed.to_string();
    while normalized.ends_with('/') {
        normalized.pop();
    }
    Ok(normalized)
}

fn login(server: String, username: String) -> Result<()> {
    let server = normalize_server(&server)?;
    let password = rpassword::prompt_password("Password: ")?;
    if password.is_empty() {
        bail!("Password cannot be empty");
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(concat!("riotgit-cli/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let response = client
        .post(format!("{server}/api/v1/auth/login"))
        .json(&json!({ "username": username.trim(), "password": password }))
        .send()?;
    let status = response.status();
    let body = response.text().unwrap_or_default();
    if !status.is_success() {
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| format!("Sign-in failed: {status}"));
        bail!("{message}");
    }
    let session: LoginResponse = serde_json::from_str(&body)?;
    write_config(&LocalConfig {
        server,
        access_token: session.access_token,
        expires_at: Some(session.expires_at),
    })?;
    println!("Signed in. Session saved in the user config directory.");
    Ok(())
}

fn print_repositories(value: &Value) {
    let Some(items) = value.as_array() else {
        println!("{value:#}");
        return;
    };
    if items.is_empty() {
        println!("No repositories visible to this account.");
        return;
    }
    println!(
        "{:<36} {:<10} {:<10} DESCRIPTION",
        "REPOSITORY", "VISIBILITY", "STATE"
    );
    for item in items {
        let full_name = format!(
            "{}/{}",
            item.get("owner").and_then(Value::as_str).unwrap_or("?"),
            item.get("name").and_then(Value::as_str).unwrap_or("?")
        );
        println!(
            "{:<36} {:<10} {:<10} {}",
            full_name,
            item.get("visibility")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            item.get("state").and_then(Value::as_str).unwrap_or("?"),
            item.get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
    }
}

fn clone_repository(api: &Api, owner: &str, name: &str, path: Option<PathBuf>) -> Result<()> {
    let target = path.unwrap_or_else(|| PathBuf::from(name));
    if target.exists()
        && fs::read_dir(&target)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true)
    {
        bail!(
            "Target path {} already exists and is not empty",
            target.display()
        );
    }
    let url = format!("{}/git/{owner}/{name}.git", api.config.server);
    let mut command = ProcessCommand::new("git");
    command
        .arg("clone")
        .arg(url)
        .arg(&target)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "http.extraHeader")
        .env(
            "GIT_CONFIG_VALUE_0",
            format!("Authorization: Bearer {}", api.config.access_token),
        );
    let status = command
        .status()
        .context("Could not start Git. Install Git and ensure it is on PATH.")?;
    if !status.success() {
        bail!("git clone failed with {status}");
    }
    println!("Cloned {owner}/{name} into {}", target.display());
    Ok(())
}

fn add_ssh_key(api: &Api, path: &Path) -> Result<()> {
    let public_key = fs::read_to_string(path)
        .with_context(|| format!("Could not read SSH public key {}", path.display()))?;
    let result = api.post(
        "/api/v1/user/ssh-keys",
        &json!({ "public_key": public_key.trim() }),
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Login { server, username } => login(server, username),
        Commands::Logout => {
            let api = Api::load()?;
            api.post("/api/v1/auth/logout", &json!({}))?;
            let path = config_path()?;
            if path.exists() {
                fs::remove_file(&path)?;
            }
            println!("Signed out.");
            Ok(())
        }
        Commands::Whoami => {
            let api = Api::load()?;
            let user = api.me()?;
            println!("{} ({}) — {}", user.display_name, user.username, user.role);
            Ok(())
        }
        Commands::Status => {
            let api = Api::load()?;
            let health = api.health()?;
            let user = api.me()?;
            println!("Server: {} ({health})", api.config.server);
            println!("Account: {} ({}) — {}", user.display_name, user.username, user.role);
            if let Some(expires_at) = api.config.expires_at {
                println!("Session expires: {expires_at}");
            }
            Ok(())
        }
        Commands::Repo { command } => {
            let api = Api::load()?;
            match command {
                RepoCommands::List => print_repositories(&api.get("/api/v1/repositories")?),
                RepoCommands::Create {
                    name,
                    description,
                    visibility,
                } => {
                    let user = api.me()?;
                    let result = api.post(
                        "/api/v1/repositories",
                        &json!({
                            "owner": user.username,
                            "name": name,
                            "description": description,
                            "visibility": visibility
                        }),
                    )?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                RepoCommands::Clone { owner, name, path } => {
                    clone_repository(&api, &owner, &name, path)?
                }
            }
            Ok(())
        }
        Commands::SshKey { command } => {
            let api = Api::load()?;
            match command {
                SshKeyCommands::List => println!(
                    "{}",
                    serde_json::to_string_pretty(&api.get("/api/v1/user/ssh-keys")?)?
                ),
                SshKeyCommands::Add { path } => add_ssh_key(&api, &path)?,
                SshKeyCommands::Remove { id } => {
                    api.delete(&format!("/api/v1/user/ssh-keys/{id}"))?;
                    println!("SSH key revoked.");
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_server;

    #[test]
    fn normalizes_server_origins() {
        assert_eq!(
            normalize_server("https://git.example.test/").unwrap(),
            "https://git.example.test"
        );
    }

    #[test]
    fn rejects_embedded_credentials_and_non_http_protocols() {
        assert!(normalize_server("https://user:pass@git.example.test").is_err());
        assert!(normalize_server("file:///tmp/repo").is_err());
    }
}
