# R!OTGIT — Single-node Deployment

## Current scope

The Compose profile runs the Rust API and Next.js web app on one machine. It is a development/early-preview profile; it is not yet a production-hardened Git forge.

The API supports SQLite metadata, local users and sessions, repository creation, repository-level read/write/admin grants, and authenticated Git Smart HTTP clone/fetch/push. The web app supports login/logout, a live repository dashboard, and repository creation. The activity feed remains an honest placeholder because the events/PR/issue modules are not built.

It does **not** yet provide Git-over-SSH, protected-branch enforcement, organization/team RBAC, repository code browsing, PR/issues, CI runners, P2P, or automated backup/restore. Do not expose this as a fully featured production forge until the relevant security and recovery gates are complete.

## Requirements

- Docker Engine and Docker Compose plugin.
- A reverse proxy with TLS for remote access.
- A long random emergency API token.
- First-run administrator username and password (both set together).
- Persistent volume storage and an independently tested backup destination.

## Start

From repository root:

```bash
cp .env.example .env
```

Set `RIOTGIT_API_TOKEN` to a unique random value. For the first startup, uncomment and set `RIOTGIT_ADMIN_USERNAME` and `RIOTGIT_ADMIN_PASSWORD` in `.env`. The password must contain at least 12 bytes. These bootstrap values are ignored when blank and are used only if the database has no users. The instance token remains a global emergency-admin credential.

Generate secrets, for example:

```bash
openssl rand -hex 32
```

Then:

```bash
docker compose -f deploy/compose/docker-compose.yml --env-file .env up --build -d
```

The services bind to loopback only by default:
- Web: `http://127.0.0.1:3000`
- API: `http://127.0.0.1:8787`

The Next.js server connects to the API over the internal Compose network using `http://api:8787`. Put a trusted TLS reverse proxy in front for remote browser access. Do not expose port 8787 directly to the public internet.

## Smoke test

```bash
curl --fail http://127.0.0.1:8787/health/live
curl --fail http://127.0.0.1:8787/health/ready
curl --fail http://127.0.0.1:8787/api/v1/meta
```

Open `http://127.0.0.1:3000/login` and sign in with the first administrator credentials. Use the web interface to create repositories and manage your own repository workspace. For Git HTTP CLI examples and permissions, see [API README](../apps/api/README.md).

Use `docker compose -f deploy/compose/docker-compose.yml logs -f api web` to inspect service logs. Never post logs containing credentials into public issues.

## Persistent data

The `riotgit-data` volume stores:
- `riotgit.db` — SQLite metadata, users, sessions, grants, and audit records;
- `repositories/` — bare Git repositories and storage housekeeping directories.

Do not edit or delete these files while the service is running. A consistent backup must include the SQLite database and repository volume together. The automated backup/restore runbook is still outstanding (Issue #16).

## Configuration

| Variable | Meaning |
| --- | --- |
| `RIOTGIT_API_TOKEN` | Required emergency instance-admin bearer token |
| `RIOTGIT_ADMIN_USERNAME` | First local administrator username; optional after initial bootstrap |
| `RIOTGIT_ADMIN_PASSWORD` | First local administrator password (12+ bytes); optional after initial bootstrap |
| `RIOTGIT_API_PORT` | Loopback host port for API (default 8787) |
| `RIOTGIT_WEB_PORT` | Loopback host port for web (default 3000) |
| `RIOTGIT_API_URL` | Internal server-side web-to-API URL (Compose sets `http://api:8787`) |
| `RIOTGIT_BIND` | API listener (Compose sets `0.0.0.0:8787` inside container) |
| `RIOTGIT_DATABASE_PATH` | SQLite path inside API volume |
| `RIOTGIT_STORAGE_ROOT` | Bare repository storage path |
| `RIOTGIT_GIT_BIN` | Git executable name or absolute path |

## Upgrade and rollback

There is no automated upgrade/rollback orchestration yet. Before upgrading:
1. Stop writes.
2. Snapshot the database and repository volume together.
3. Pull/build the reviewed commit.
4. Start the new version and verify health.
5. Exercise session login and Git clone/push checks.
6. Retain the previous image and snapshot until validation finishes.

Do not roll back only the database or only repository storage.

## Security notes

- Treat `RIOTGIT_API_TOKEN` like a root secret; it grants emergency instance-admin access.
- Local accounts use Argon2 password hashes and revocable seven-day bearer sessions.
- Browser sessions use HttpOnly and SameSite=Lax cookies managed by Next.js server routes.
- Repository grants support `read`, `write`, and `admin`; protected-branch enforcement remains outstanding.
- Login rate limiting and full security hardening remain outstanding; use a trusted reverse proxy and firewall.
- Keep the named data volume private and persistent.
- Containers use a non-root user and a read-only root filesystem.
- CI runner and P2P services are intentionally absent from this profile.
