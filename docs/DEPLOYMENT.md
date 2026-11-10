# R!OTGIT — Single-node Deployment

## Current scope

The Compose profile runs the current Rust API and Next.js cockpit on one machine. It is a development/early-preview deployment profile, not yet a production Git forge.

The API currently supports:
- liveness and readiness checks;
- product metadata;
- SQLite-backed repository metadata;
- repository creation that initializes a native bare Git directory;
- authenticated repository list and lookup.

It does **not** yet provide Git-over-SSH or Git smart HTTP clone/push/fetch transport, full user accounts/RBAC, collaboration, CI runners, or P2P. The web cockpit still contains sample preview records. Do not expose it as a completed service for team source hosting until the remaining P0/P1 security and transport issues are closed.

## Requirements

- Docker Engine and Docker Compose plugin.
- A reverse proxy with TLS for remote access.
- A long random API token.
- Persistent volume storage and a tested backup destination.

## Start

From repository root:

```bash
cp .env.example .env
```

Set `RIOTGIT_API_TOKEN` to a unique random value. For example, generate one with:

```bash
openssl rand -hex 32
```

Then:

```bash
docker compose -f deploy/compose/docker-compose.yml --env-file .env up --build -d
```

The services bind to loopback only by default:
- Web cockpit: `http://127.0.0.1:3000`
- API: `http://127.0.0.1:8787`

Put a trusted TLS reverse proxy in front for remote browser access. Do not expose port 8787 directly to the public internet.

## Smoke test

```bash
curl --fail http://127.0.0.1:8787/health/live
curl --fail http://127.0.0.1:8787/health/ready
curl --fail http://127.0.0.1:8787/api/v1/meta

curl -X POST http://127.0.0.1:8787/api/v1/repositories \
  -H "Authorization: Bearer $RIOTGIT_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"owner":"builder","name":"hello-world","visibility":"private"}'
```

Use `docker compose -f deploy/compose/docker-compose.yml logs -f api web` to inspect service logs. Never post logs containing credentials into public issues.

## Persistent data

The `riotgit-data` volume stores:
- `riotgit.db` — SQLite metadata;
- `repositories/` — bare Git repositories and storage housekeeping directories.

Do not edit or delete these files while the service is running. A backup must include the database and repository storage in a consistent snapshot. A backup/restore runbook and automated restore test remain part of Issue #16.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `RIOTGIT_API_TOKEN` | Required | Instance-level admin bearer token |
| `RIOTGIT_API_PORT` | `8787` | Loopback host port |
| `RIOTGIT_WEB_PORT` | `3000` | Loopback host port |
| `RIOTGIT_BIND` | `127.0.0.1:8787` locally | API listener |
| `RIOTGIT_DATABASE_PATH` | `data/riotgit.db` locally | SQLite path |
| `RIOTGIT_STORAGE_ROOT` | `data/repositories` locally | Bare repository storage |
| `RIOTGIT_GIT_BIN` | `git` | Git executable name or absolute path |

## Upgrade and rollback

There is no automatic upgrade/rollback orchestration yet. Before upgrading:
1. Stop writes.
2. Snapshot the database and repository volume together.
3. Pull/build the reviewed commit.
4. Start the new version and verify health.
5. Perform clone/fsck/restore checks once Git transport is implemented.
6. Retain the previous image and snapshot until validation finishes.

Do not roll back only the database or only repository storage.

## Security notes

- API token is a temporary single-instance credential, not per-user RBAC.
- Keep services behind a trusted proxy and firewall.
- Keep the named data volume private and persistent.
- The container runs as a non-root user and uses a read-only root filesystem.
- CI runner and P2P services are intentionally absent from this profile.
