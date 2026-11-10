# R!OTGIT API

The API is a Rust/Axum modular-monolith bootstrap with SQLite metadata and native Git repository initialization.

## Requirements
- Rust stable (edition 2024 support).
- Git CLI available on PATH.
- A writable local data directory.

## Run locally

From the repository root:

```bash
cp .env.example .env
# Set RIOTGIT_API_TOKEN to a unique random secret before starting.
set -a && . ./.env && set +a
cargo run -p riotgit-api
```

PowerShell:

```powershell
Copy-Item .env.example .env
# Set RIOTGIT_API_TOKEN in the current environment.
$env:RIOTGIT_API_TOKEN = "replace-with-a-long-random-secret"
cargo run -p riotgit-api
```

The API listens on `127.0.0.1:8787` by default.

## Routes

- `GET /health/live` — process liveness.
- `GET /health/ready` — database and storage readiness.
- `GET /api/v1/meta` — public product/API metadata.
- `GET /api/v1/repositories` — authenticated repository listing.
- `GET /api/v1/repositories/{owner}/{name}` — authenticated repository lookup.
- `POST /api/v1/repositories` — authenticated bare repository creation.

Repository administration currently uses one instance-level bearer token. It is a bootstrap guard, **not** the finished user/session/RBAC system. Do not share one token among untrusted users; user identity and scoped grants are tracked separately.

## Create a repository

```bash
curl -X POST http://127.0.0.1:8787/api/v1/repositories \
  -H "Authorization: Bearer $RIOTGIT_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"owner":"builder","name":"hello-world","description":"First R!OTGIT repository","visibility":"private"}'
```

This reserves metadata, initializes a bare Git repository in a staging directory using the native Git executable, then atomically activates its storage path.

Git clone/push/fetch transport is not implemented by this API bootstrap yet. It is a separate P0 issue and must not be inferred from repository creation support.

## Tests

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
``

## Configuration
See the root `.env.example`. Keep the database and repository volumes persistent and outside the public web root.
