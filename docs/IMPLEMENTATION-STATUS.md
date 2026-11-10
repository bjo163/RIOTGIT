# R!OTGIT — Implementation Status

Last updated: 2026-11-11

## Verified implementation

The following commit passed the full GitHub Actions pipeline:
https://github.com/bjo163/RIOTGIT/actions/runs/42879655870

- [x] Rust formatting: `cargo fmt --all -- --check`.
- [x] Rust compile: `cargo check --workspace --all-targets`.
- [x] Clippy: `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] Rust tests: `cargo test --workspace`.
- [x] API build: `cargo build -p riotgit-api`.
- [x] Web dependency install, TypeScript typecheck, ESLint, and Next.js production build.
- [x] End-to-end auth/Git HTTP smoke test.

## Implemented and committed

### Product and engineering foundation
- [x] Product narrative, positioning, brand identity, logo SVGs, and product design system.
- [x] Architecture, threat model, roadmap, contribution standards, issue templates, and pull request template.
- [x] Rust workspace and Axum API.
- [x] Structured error responses and liveness/readiness routes.
- [x] SQLite initialization with versioned migrations.

### Identity and access
- [x] First-run administrator bootstrap from environment variables.
- [x] Argon2 password hashing and verification.
- [x] User login and 7-day opaque bearer sessions.
- [x] Session revocation via logout.
- [x] Admin-managed user creation with `admin` and `member` roles.
- [x] Repository-level `read`, `write`, and `admin` grants with grant/revoke API.
- [x] Read/write access checks applied to Git Smart HTTP.
- [x] Instance token retained as an emergency global-admin credential.

### Git core
- [x] SQLite-backed repository metadata.
- [x] Bare Git repository initialization using native Git.
- [x] Opaque storage IDs and staged activation under the configured repository root.
- [x] Duplicate owner/name conflict response.
- [x] Name validation against traversal and unsafe characters.
- [x] Authenticated Git Smart HTTP discovery, upload-pack, and receive-pack.
- [x] Request body cap (64 MiB) and backend execution timeout (120 seconds).
- [x] Git CLI round-trip: create repository → push → clone → compare file.
- [x] Permission smoke: read-only clone succeeds, push is denied, revoked grant blocks later clone.

### Web application
- [x] Next.js App Router application and R!OTGIT design system.
- [x] Login, logout, and session lookup through server-side route handlers.
- [x] HttpOnly, SameSite=Lax browser session cookie; bearer token is not stored in local storage.
- [x] Live user and repository dashboard, permission-filtered list, and honest empty states.
- [x] Repository creation screen backed by the API.
- [x] Docker Compose internal API routing.

### Operations
- [x] Dockerfiles and single-node Docker Compose profile.
- [x] Non-root API runtime, read-only root filesystem, loopback host port bindings, persistent volume, and health checks.
- [x] GitHub Actions CI covering Rust, web, and Git/auth smoke workflows.

## Remaining work

### Git and security
- [ ] OpenSSH forced-command Git transport and SSH key registration/revocation.
- [ ] Protected branch rules and server-side receive-pack enforcement.
- [ ] Large pack streaming/backpressure; current HTTP gateway buffers bounded request/response bodies.
- [ ] Git LFS.
- [ ] Login/API rate limiting, origin/CSRF defense review, token rotation, and security hardening.
- [ ] PostgreSQL adapter and cross-database integration suite.

### Forge collaboration
- [x] Repository tree/blob browser and commit history.
- [ ] Blame and diff view.
- [ ] Pull requests, reviews, merge checks, merge engine, and branch protection UI.
- [x] Repository Issues: create/list/update/close, comments, repository access checks, and browser tracker UI.
- [ ] Labels, milestones, notifications, and issue detail timeline.
- [ ] CLI/TUI.

### Automation and network
- [ ] Durable background queue and signed webhook delivery.
- [ ] Isolated CI runner and a clearly scoped workflow subset.
- [ ] Mirrors, federation contracts, and Radicle/P2P adapter.
- [ ] Opt-in AI gateway and repository-scoped agents.

### Production readiness
- [ ] Committed Cargo/npm lockfiles and frozen CI installs.
- [ ] Automated database + repository backup and restore drill.
- [ ] Upgrade/rollback runbook validated against data snapshots.
- [ ] Container build/compose smoke test in CI.
- [ ] Load, soak, disk-full, and recovery tests.
- [ ] Full security review and release checklist.

## Important limitations

- Smart HTTP is implemented; SSH is not.
- Identity and repository-level grants work, but there is no organization/team model yet.
- Protected-branch policy is not enforced yet.
- Web supports live repository data, tree/blob browsing, commit history, bounded diffs, and basic repository issue/comment workflows; pull requests and advanced collaboration are not implemented.
- CI success verifies this foundation and smoke paths only. It does not make R!OTGIT a complete or production-hardened Git forge.
- PostgreSQL, P2P/federation, runners, and AI remain planned.

## Local run

API:

```bash
cp .env.example .env
# Set RIOTGIT_API_TOKEN, RIOTGIT_ADMIN_USERNAME and RIOTGIT_ADMIN_PASSWORD.
set -a && . ./.env && set +a
cargo run -p riotgit-api
```

Web (another terminal):

```bash
cd apps/web
cp .env.example .env.local
npm install
npm run dev
```

See [API README](../apps/api/README.md) and [Deployment](DEPLOYMENT.md).
