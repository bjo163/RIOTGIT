# R!OTGIT — Single-node Deployment

## Current scope

The Compose profile runs the Rust API and Next.js web application on one machine. It now includes SQLite metadata, local identity/sessions, repository-level permissions, native bare Git storage, authenticated Git Smart HTTP, browser code views, Issues/labels/milestones, Pull Requests/reviews/merge, and branch protection for Smart HTTP pushes.

This is an early self-hosted release foundation, not a claim of complete production hardening. A restricted OpenSSH gateway is available through the optional `ssh` Compose profile; it requires the separate SSH integration smoke test and deployment-specific host-key/firewall review. Large-pack streaming, Git LFS, PostgreSQL, CI runners, federation/P2P, and AI agent execution are not enabled.

## Requirements

- Docker Engine and Docker Compose plugin.
- A trusted reverse proxy with TLS for remote browser access.
- A long random emergency API token.
- First-run administrator username/password (configured together).
- Persistent Docker volume and encrypted off-host backup destination.

## Start

From repository root:

```bash
cp .env.example .env
```

Set `RIOTGIT_API_TOKEN` to a unique random secret. For first startup, uncomment and set `RIOTGIT_ADMIN_USERNAME` and `RIOTGIT_ADMIN_PASSWORD`; the password must contain at least 12 bytes. Bootstrap runs only when no users exist. The API token remains an emergency instance-admin credential.

Generate a token:

```bash
openssl rand -hex 32
```

Start the stack:

```bash
docker compose -f deploy/compose/docker-compose.yml --env-file .env up --build -d
```

Host ports bind to loopback by default:
- Web: `http://127.0.0.1:3000`
- API: `http://127.0.0.1:8787`
- Optional SSH: `127.0.0.1:2222` (enable the `ssh` profile)

Next.js talks to the API over the internal Compose network (`http://api:8787`). Put a trusted TLS reverse proxy in front of the web service. Do not expose the API port directly to the public internet. The SSH port is loopback-bound by default; remote Git-over-SSH requires an explicit firewall/port publication change. Verify the server host-key fingerprint through a trusted channel before accepting it.

## Smoke test

```bash
curl --fail http://127.0.0.1:8787/health/live
curl --fail http://127.0.0.1:8787/health/ready
curl --fail http://127.0.0.1:8787/api/v1/meta
```

Open `http://127.0.0.1:3000/login`. Create repositories and collaborate through the web interface or standard Git over authenticated HTTPS. To enable SSH, run `docker compose -f deploy/compose/docker-compose.yml --env-file .env --profile ssh up --build -d api ssh`; see [SSH gateway guide](../apps/ssh/README.md). See also [API README](../apps/api/README.md) and [CLI guide](../apps/cli/README.md).

## Persistent data

The `riotgit-data` volume stores both:
- `riotgit.db` — SQLite metadata, users, sessions, repository grants, issue/PR metadata, and audit records;
- `repositories/` — bare Git repositories and Git storage housekeeping directories.

Do not edit or delete these files while services are running. A recovery point must capture the full data volume, not only the database or repository directory.

## Backup and restore

Use the maintained scripts from the repository root:

```bash
bash scripts/backup.sh
bash scripts/restore.sh ./backups/riotgit-data-YYYYMMDDTHHMMSSZ.tar.gz
```

The backup script stops API/web while archiving the complete volume and writes a checksum. The restore script validates basic archive paths, requests explicit confirmation, creates a pre-restore safety snapshot, replaces the volume, then restarts services. Backups should be copied to encrypted off-host storage. A backup is not proven until a restore drill succeeds on a separate test environment. See [Backup & Restore Runbook](BACKUP-RESTORE.md).

## Configuration

| Variable | Meaning |
| --- | --- |
| `RIOTGIT_API_TOKEN` | Required emergency instance-admin bearer token |
| `RIOTGIT_ADMIN_USERNAME` | First local administrator username; used only on first bootstrap |
| `RIOTGIT_ADMIN_PASSWORD` | First local administrator password (12+ bytes) |
| `RIOTGIT_API_PORT` | Loopback API host port (default 8787) |
| `RIOTGIT_WEB_PORT` | Loopback web host port (default 3000) |
| `RIOTGIT_API_URL` | Internal web-to-API URL (Compose sets `http://api:8787`) |
| `RIOTGIT_BIND` | API listener (Compose sets `0.0.0.0:8787` in container) |
| `RIOTGIT_DATABASE_PATH` | SQLite metadata database path |
| `RIOTGIT_STORAGE_ROOT` | Bare repository storage root |
| `RIOTGIT_GIT_BIN` | Git executable name or absolute path |

## Upgrade and rollback

Before upgrading:
1. Stop accepting new writes.
2. Create and verify a full-volume backup.
3. Pull/build a reviewed commit.
4. Start the new version and verify readiness.
5. Test login, repository listing, clone/fetch/push, and one collaboration workflow.
6. Retain the previous image and pre-upgrade backup until validation is complete.

Do not roll back only the database or only repository storage. Migrations are versioned and applied on API startup; check migration compatibility before using a downgraded image.

## Security notes

- Treat `RIOTGIT_API_TOKEN` as a root secret; it grants emergency instance-admin access.
- Local account passwords use Argon2 hashes; bearer sessions are revocable and expire after seven days.
- Browser sessions use HttpOnly/SameSite cookies and mutating web API calls are guarded by the Next.js same-origin proxy.
- Login failures are throttled per normalized username; IP-aware rate limiting and proxy-trust hardening remain outstanding.
- SSH public keys can be registered, listed, and revoked. The optional gateway applies a forced command and disables interactive shell/forwarding. Keep the SSH profile disabled until its end-to-end smoke test passes; validate the persisted host-key fingerprint before public exposure.
- Branch protection is enforced for Git Smart HTTP receive-pack; do not assume it covers SSH until SSH transport is implemented.
- Containers run as non-root with read-only root filesystems and loopback host bindings.
- CI runner, federation/P2P, and AI execution services are intentionally absent from this Compose profile.
