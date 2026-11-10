# R!OTGIT API

The API is a Rust/Axum modular monolith with SQLite metadata, local identity, repository-level access grants, native bare Git storage, and authenticated Git Smart HTTP.

## Requirements
- Rust stable with edition 2024 support.
- Git CLI available on PATH.
- A writable local data directory.

## Run locally

From the repository root:

```bash
cp .env.example .env
# Set RIOTGIT_API_TOKEN, RIOTGIT_ADMIN_USERNAME, and RIOTGIT_ADMIN_PASSWORD.
set -a && . ./.env && set +a
cargo run -p riotgit-api
```

PowerShell:

```powershell
Copy-Item .env.example .env
$env:RIOTGIT_API_TOKEN = "replace-with-a-long-random-secret"
$env:RIOTGIT_ADMIN_USERNAME = "admin"
$env:RIOTGIT_ADMIN_PASSWORD = "replace-with-a-unique-password-of-12-plus-characters"
cargo run -p riotgit-api
```

The API listens on `127.0.0.1:8787` by default.

## Routes

- `GET /health/live` — process liveness.
- `GET /health/ready` — database and storage readiness.
- `GET /api/v1/meta` — public product/API metadata.
- `POST /api/v1/auth/login` — create a bearer session.
- `GET /api/v1/auth/me` — current identity.
- `POST /api/v1/auth/logout` — revoke current session.
- `POST /api/v1/admin/users` — create a user (admin only).
- `GET /api/v1/repositories` — authenticated, permission-filtered repository listing.
- `POST /api/v1/repositories` — create a bare repository.
- `GET /api/v1/repositories/{owner}/{name}` — authorized repository lookup.\n- `GET /api/v1/repositories/{owner}/{name}/issues` — list issues (optional `state=open|closed`).\n- `GET /api/v1/repositories/{owner}/{name}/diff?base=main&head=HEAD` — bounded, no-external-diff comparison of two refs.\n- `POST /api/v1/repositories/{owner}/{name}/issues` — create an issue (repository write permission).\n- `PUT /api/v1/repositories/{owner}/{name}/issues/{number}` — update title/body or open/close an issue (author or repository writer).\n- `GET /api/v1/repositories/{owner}/{name}/issues/{number}/comments` — list comments.\n- `POST /api/v1/repositories/{owner}/{name}/issues/{number}/comments` — add a comment to an open issue (repository read access).
- `GET /api/v1/repositories/{owner}/{name}/labels` and `POST .../labels` — list and create repository labels.
- `GET /api/v1/repositories/{owner}/{name}/issues/{number}/labels` — list labels attached to an issue.
- `PUT /api/v1/repositories/{owner}/{name}/issues/{number}/labels/{label_id}` and `DELETE ...` — attach or remove an issue label.
- `GET /api/v1/repositories/{owner}/{name}/milestones` and `POST .../milestones` — list and create milestones.
- `PUT /api/v1/repositories/{owner}/{name}/issues/{number}/milestone` — assign an open milestone.
- `DELETE /api/v1/repositories/{owner}/{name}/issues/{number}/milestone` — clear the milestone assignment.
- `GET /api/v1/repositories/{owner}/{name}/branch-protection` — list branch rules.
- `POST /api/v1/repositories/{owner}/{name}/branch-protection` — create or update a rule (repository admin).
- `DELETE /api/v1/repositories/{owner}/{name}/branch-protection?branch_name=main` — remove a rule (repository admin).

Branch protection is currently enforced on Git Smart HTTP receive-pack. SSH transport is not implemented yet.

Pull request routes:
- `GET /api/v1/repositories/{owner}/{name}/pulls` — list pull requests.
- `POST .../pulls` — create a pull request from two local branches; head/base commit SHAs are captured at creation.
- `GET .../pulls/{number}` and `PUT .../pulls/{number}` — read, close, or reopen a pull request.
- `GET .../pulls/{number}/reviews` and `POST .../pulls/{number}/reviews` — list and submit review decisions.
- `POST .../pulls/{number}/merge` — merge a clean pull request using a merge commit and compare-and-swap ref update.

When the target branch has a `require_pull_request` protection rule, merge requires at least one current approval from a reviewer other than the pull request author. Head/base refs must still match the captured SHAs; stale pull requests and conflicts are rejected. Merge state is journaled as `merging` so a later read/merge request can reconcile a process interruption. This is a first-party R!OTGIT API, not a claim of GitHub REST compatibility.
- `PUT /api/v1/repositories/{owner}/{name}/access/{username}` — grant `read`, `write`, or `admin`.
- `DELETE /api/v1/repositories/{owner}/{name}/access/{username}` — revoke a grant.
- `GET /git/{owner}/{repo}.git/info/refs?service=git-upload-pack` — fetch/clone discovery.
- `GET /git/{owner}/{repo}.git/info/refs?service=git-receive-pack` — push discovery.
- `POST /git/{owner}/{repo}.git/git-upload-pack` — Git clone/fetch.
- `POST /git/{owner}/{repo}.git/git-receive-pack` — Git push.

## First administrator

Set both `RIOTGIT_ADMIN_USERNAME` and `RIOTGIT_ADMIN_PASSWORD` before the first API start. The password must be 12–1024 bytes. Bootstrap is performed only if the users table is empty; blank values are ignored. The environment bootstrap values are not a routine password-reset mechanism.

`RIOTGIT_API_TOKEN` is an emergency instance-wide admin credential. Store it like a root secret. Normal web/API access should use a local user session.

## Login and user creation

```bash
ADMIN_SESSION="$(curl --fail --silent -X POST http://127.0.0.1:8787/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"YOUR_BOOTSTRAP_PASSWORD"}' | jq -r .access_token)"

curl --fail --silent -X POST http://127.0.0.1:8787/api/v1/admin/users \
  -H "Authorization: Bearer $ADMIN_SESSION" \
  -H 'Content-Type: application/json' \
  -d '{"username":"builder","display_name":"Builder","password":"A-Unique-Password-123!","role":"member"}'
```

Bearer sessions currently expire after seven days and are revoked by `POST /api/v1/auth/logout`. The web app keeps its browser session in an HttpOnly cookie and forwards the bearer token server-side.

## Create and grant a repository

A member may create repositories under their own username. Admins may create repositories under any owner. Example after logging in as `builder`:

```bash
BUILDER_SESSION="your-builder-session-token"
curl --fail --silent -X POST http://127.0.0.1:8787/api/v1/repositories \
  -H "Authorization: Bearer $BUILDER_SESSION" \
  -H 'Content-Type: application/json' \
  -d '{"owner":"builder","name":"hello-world","description":"First R!OTGIT repository","visibility":"private"}'

curl --fail --silent -X PUT http://127.0.0.1:8787/api/v1/repositories/builder/hello-world/access/reader \
  -H "Authorization: Bearer $BUILDER_SESSION" \
  -H 'Content-Type: application/json' \
  -d '{"permission":"read"}'
```

A `read` grant allows clone/fetch but does not allow push. `write` and `admin` grants allow push. Repository owner and instance admin can grant/revoke permissions.

## Git Smart HTTP

Use a user session token with a standard Git client:

```bash
export RIOTGIT_TOKEN="your-session-token"
git -c http.extraHeader="Authorization: Bearer $RIOTGIT_TOKEN" clone \
  http://127.0.0.1:8787/git/builder/hello-world.git
```

The transport delegates to native `git http-backend`. Request body size is capped at 64 MiB and backend execution at 120 seconds. The CI smoke test covers administrator bootstrap, builder/reader sessions, read-only clone, push denial, grant revocation, and session logout.

SSH transport, protected-branch rules, rate limiting for login, large-pack streaming, and Git LFS are not implemented yet.

## Tests

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p riotgit-api
bash scripts/smoke-git-http.sh
```

## Configuration

See the root `.env.example`. Keep the database and repository volumes persistent and outside the public web root.
