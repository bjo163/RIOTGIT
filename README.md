# R!OTGIT

> **THE UNDERGROUND CODE NETWORK**  
> Break the rules. Own your code.

R!OTGIT is a self-hosted, local-first Git forge foundation with a loud punk identity and a deliberately boring core: standard Git repositories, native Git transport, explicit access control, and portable filesystem storage.

**R!OTGIT is not a new Git implementation.** Native Git repositories and protocols remain the source of truth. R!OTGIT adds identity, permissions, repository management, and collaboration around them.

![Project status](https://img.shields.io/badge/status-single--node%20MVP%20foundation-ccff33?style=flat-square)
![Architecture](https://img.shields.io/badge/architecture-modular%20monolith-ff5a1f?style=flat-square)
![License](https://img.shields.io/badge/license-TBD-lightgrey?style=flat-square)

## What works today

- Rust + Axum API with SQLite metadata and versioned SQL migrations.
- First-run administrator bootstrap, Argon2 password hashes, revocable bearer sessions, and admin-managed users.
- Repository creation, listing, and access-filtered lookup.
- Repository grants: `read`, `write`, and `admin`.
- Native bare Git repository initialization on the filesystem.
- Repository tree/blob browsing, commit history, and bounded base/head diff view.
- Issues, comments, labels, and milestones in the repository workspace.
- Pull request create/review/merge workflow with protected-branch approval checks.
- Server-side branch protection for Git Smart HTTP receive-pack, with a settings UI.
- Authenticated Git Smart HTTP clone/fetch/push via native `git http-backend`.
- Next.js login/logout, HttpOnly session cookie, live repository dashboard, and repository creation.
- Docker Compose single-node profile and GitHub Actions checks.
- Rust CLI for login, identity, repository list/create/clone, SSH-key management, and logout.
- SSH public-key register/list/revoke API and per-username login failure throttling.
- Same-origin guard for mutating Next.js web API routes.
- Docker-volume backup/restore scripts and recovery runbook (restore drill still required).

See [Implementation Status](docs/IMPLEMENTATION-STATUS.md) and [the latest successful CI run](https://github.com/bjo163/RIOTGIT/actions/runs/42893061450).

## Terminal CLI

Build:

```bash
cargo build --release -p riotgit
```

Sign in and use common repository workflows:

```bash
riotgit login --server https://git.example.com --username builder
riotgit whoami
riotgit repo list
riotgit repo create --name notes --visibility private
riotgit repo clone builder notes
riotgit ssh-key add ~/.ssh/id_ed25519.pub
```

See [CLI guide](apps/cli/README.md). SSH key registration is available; OpenSSH clone/push transport remains a separate unfinished feature.

## Backup and recovery

```bash
bash scripts/backup.sh
bash scripts/restore.sh ./backups/riotgit-data-YYYYMMDDTHHMMSSZ.tar.gz
```

Scripts snapshot the complete single-node data volume and create a pre-restore safety archive. A separate-host restore drill is still required before calling recovery production-verified. See [Backup & Restore](docs/BACKUP-RESTORE.md).

## Product pillars

- **Sovereign:** self-host your code and control your data.
- **Compatible:** use standard Git clients over authenticated Smart HTTP and an optional restricted OpenSSH gateway.
- **Local-first:** bare repositories remain portable outside the web app.
- **Lean:** modular monolith first, optional workers later.
- **Extensible:** mirrors, federation, P2P, runners, and AI are adapter-based future capabilities.
- **Human-scale:** browser, API, and terminal are intended as equal entry points.

## Technology baseline

| Area | Current choice |
| --- | --- |
| API | Rust stable + Axum |
| Web | Next.js App Router + TypeScript |
| Database | SQLite default; PostgreSQL adapter is future work |
| Metadata | SQLx + versioned migrations |
| Git storage | Native bare repositories on filesystem |
| Git Smart HTTP | Native `git http-backend` behind authenticated API route |
| Git SSH | Optional OpenSSH gateway with DB-backed authorized keys and forced-command transport |
| Identity | Argon2 password hashes, revocable bearer sessions, SSH key registry |
| Browser session | HttpOnly, SameSite=Lax cookie managed by Next.js server routes |
| Deployment | Docker Compose single-node profile, volume backup/restore scripts |

Native Git stays canonical. gitoxide and Radicle are research/integration directions, not required dependencies of the core MVP.

## Quick start — API

Requirements: Rust stable with edition 2024 support, Git CLI, and a writable data directory.

From repository root:

```bash
cp .env.example .env
```

Set these values in `.env`:
- `RIOTGIT_API_TOKEN`: long random emergency administrator secret.
- `RIOTGIT_ADMIN_USERNAME`: first local admin username.
- `RIOTGIT_ADMIN_PASSWORD`: first local admin password (12+ bytes).

The bootstrap user is created only when the database has no users. Blank bootstrap variables are ignored after first setup.

Start the API:

```bash
set -a
. ./.env
set +a
cargo run -p riotgit-api
```

Default API: `http://127.0.0.1:8787`.

## Quick start — Web

In another terminal:

```bash
cd apps/web
cp .env.example .env.local
npm install
npm run dev
```

Open `http://localhost:3000/login`. Sign in with the first administrator credentials. The web app calls the API server-side and stores the session in an HttpOnly cookie; it does not keep the bearer token in browser local storage.

## Docker Compose

Set the same API token and first-admin bootstrap values in the root `.env`, then:

```bash
docker compose -f deploy/compose/docker-compose.yml --env-file .env up --build -d
```

The web and API host ports bind to loopback by default. Place a trusted TLS reverse proxy in front for remote access. See [Deployment](docs/DEPLOYMENT.md).

## Git over SSH (optional)

Enable the restricted gateway with the Compose SSH profile:

```bash
docker compose -f deploy/compose/docker-compose.yml --env-file .env --profile ssh up --build -d api ssh
```

It listens on port 2222 by default (loopback-bound). Configure firewall/port exposure deliberately for remote access. Register a key with `riotgit ssh-key add ~/.ssh/id_ed25519.pub`, then clone with:

```bash
git clone ssh://git@your-host.example:2222/your-user/your-repo.git
```

The SSH account is always `git`; the registered key identifies the application user. Interactive shell and forwarding are disabled. See [SSH gateway guide](apps/ssh/README.md). This gateway is experimental until the SSH CI integration and deployment-specific host-key/security checks pass.

## Git Smart HTTP

The current Git transport is HTTP, not SSH. Example with a session token:

```bash
export RIOTGIT_TOKEN="your-session-token"
git -c http.extraHeader="Authorization: Bearer $RIOTGIT_TOKEN" clone \
  http://127.0.0.1:8787/git/your-user/your-repo.git
```

To push, the session must have repository `write` or `admin` permission. A `read` grant allows clone/fetch but denies push.

## Product and engineering documentation

- [Product narrative](docs/PRODUCT.md)
- [Brand identity](docs/BRAND.md)
- [Design system](docs/DESIGN-SYSTEM.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Security baseline](docs/SECURITY.md)
- [Delivery roadmap](docs/ROADMAP.md)
- [Implementation status](docs/IMPLEMENTATION-STATUS.md)
- [API guide](apps/api/README.md)
- [Deployment guide](docs/DEPLOYMENT.md)
- [Contribution standards](CONTRIBUTING.md)

## Not implemented yet

Additional SSH interoperability testing and hardening, durable jobs/webhooks, isolated CI runners, mirrors/federation/P2P, AI agent execution, PostgreSQL parity, large-pack streaming/LFS, organization/team RBAC, advanced PR review checks, and automated restore drills remain tracked work. Branch protection currently applies to Git Smart HTTP only. The CLI is an initial terminal client, not a TUI or full admin client. Do not treat R!OTGIT as a fully production-hardened forge until the remaining security, scale, and recovery gates pass.

## Engineering rules

- Keep `main` buildable; use small, reviewable commits.
- Define contracts before parallel implementation.
- Never store Git objects in relational tables.
- Use parameterized SQL and process argument arrays; never interpolate user input into shell commands.
- Treat repository paths, refs, hooks, archives, diffs, LFS objects, and CI artifacts as untrusted.
- Deny by default and enforce permissions server-side.
- Do not advertise Actions compatibility, P2P, SSH, or production readiness until their integration tests pass.

## Brand

**R!OTGIT — THE UNDERGROUND CODE NETWORK**  
**BREAK THE RULES. OWN YOUR CODE.**

The visual system is extreme, raw, high-contrast, and accessible. Code, diffs, logs, and forms remain readable.

## License

License selection is intentionally open pending a repository-level decision. Do not claim open-source licensing until the owner selects and commits a license.
