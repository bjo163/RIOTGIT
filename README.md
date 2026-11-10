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
- Authenticated Git Smart HTTP clone/fetch/push via native `git http-backend`.
- Next.js login/logout, HttpOnly session cookie, live repository dashboard, and repository creation.
- Docker Compose single-node profile and GitHub Actions checks.
- Automated Git/auth smoke test: bootstrap → login → grant read → clone → reject push → revoke → reject clone → logout.

See [Implementation Status](docs/IMPLEMENTATION-STATUS.md) and [the latest successful CI run](https://github.com/bjo163/RIOTGIT/actions/runs/42879655870).

## Product pillars

- **Sovereign:** self-host your code and control your data.
- **Compatible:** use standard Git clients over HTTP today; SSH transport is planned.
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
| Identity | Argon2 password hashes + revocable bearer sessions |
| Browser session | HttpOnly, SameSite=Lax cookie managed by Next.js server routes |
| Deployment | Docker Compose single-node profile |

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

SSH transport/key lifecycle, protected branches, code browser/commit diff views, pull requests/reviews/issues, CLI/TUI, durable jobs/webhooks, isolated CI runners, mirrors/federation/P2P, AI agents, PostgreSQL parity, and automated backup/restore drills remain tracked work. Do not treat R!OTGIT as a production-ready multi-user forge until those relevant security and recovery gates pass.

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
