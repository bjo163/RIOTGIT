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
- `POST /api/v1/auth/login` — create a bearer session.\n- `GET /api/v1/auth/me` — current identity.\n- `POST /api/v1/auth/logout` — revoke current session.\n- `POST /api/v1/admin/users` — create a user (admin only).\n- `PUT/DELETE /api/v1/repositories/{owner}/{name}/access/{username}` — grant/revoke repository access.\n- `GET /api/v1/repositories` — authenticated, permission-filtered repository listing.
- `GET /api/v1/repositories/{owner}/{name}` — authenticated repository lookup.
- `POST /api/v1/repositories` — authenticated bare repository creation.
- `GET /git/{owner}/{repo}.git/info/refs?service=git-upload-pack` — Git fetch discovery.
- `POST /git/{owner}/{repo}.git/git-upload-pack` — Git clone/fetch.
- `POST /git/{owner}/{repo}.git/git-receive-pack` — Git push.

Local users now authenticate with Argon2 password hashes and opaque bearer sessions (7-day expiry, revocable). Set both `RIOTGIT_ADMIN_USERNAME` and `RIOTGIT_ADMIN_PASSWORD` on first run to bootstrap the first administrator; the password must be at least 12 bytes. Admins create users with `POST /api/v1/admin/users`. The instance-level `RIOTGIT_API_TOKEN` remains an emergency global-admin credential. Repository owners can grant `read`, `write`, or `admin` via `PUT /api/v1/repositories/{owner}/{name}/access/{username}` and revoke with `DELETE` on the same route. Git Smart HTTP accepts Bearer or Basic credentials (token as Basic password) and enforces read vs write grants.

## Create a repository

```bash
curl -X POST http://127.0.0.1:8787/api/v1/repositories \
  -H "Authorization: Bearer $RIOTGIT_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"owner":"builder","name":"hello-world","description":"First R!OTGIT repository","visibility":"private"}'
```

This reserves metadata, initializes a bare Git repository in a staging directory using the native Git executable, then atomically activates its storage path.

Git smart HTTP is routed through native `git http-backend` after an authenticated repository lookup. Requests are capped at 64 MiB and Git execution at 120 seconds. The CI smoke test creates a repository, pushes a commit with the normal Git CLI, clones it over HTTP, and compares the resulting file. See [the successful CI run](https://github.com/bjo163/RIOTGIT/actions/runs/42876569801). SSH transport, per-user identities, read-only access, branch protection, and large-pack streaming are not implemented yet.

## Tests

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
``

## Configuration
See the root `.env.example`. Keep the database and repository volumes persistent and outside the public web root.
