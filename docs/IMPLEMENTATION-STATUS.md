# R!OTGIT — Implementation Status

Last updated: 2026-11-11

## Verified implementation

The current implementation baseline passed the full GitHub Actions pipeline:
https://github.com/bjo163/RIOTGIT/actions/runs/42893061450

- [x] Rust formatting: `cargo fmt --all -- --check`.
- [x] Rust compile: `cargo check --workspace --all-targets`.
- [x] Clippy: `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] Rust tests: `cargo test --workspace`.
- [x] API build: `cargo build -p riotgit-api`.
- [x] Web dependency install, TypeScript typecheck, ESLint, and Next.js production build.
- [x] End-to-end auth, issue collaboration, Git HTTP, branch protection, and diff smoke test.

## Implemented and committed

### Product and engineering foundation
- [x] Product narrative, positioning, brand identity, logo SVGs, and product design system.
- [x] Architecture, threat model, roadmap, contribution standards, issue templates, and pull request template.
- [x] Rust workspace and Axum API.
- [x] Structured error responses and liveness/readiness routes.
- [x] SQLite initialization with versioned migrations.

### Identity and access
- [x] First-run administrator bootstrap from environment variables.
- [x] SSH public key registration/list/revocation API with key-blob validation and SHA-256 fingerprints.
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
- [x] Receive-pack branch policy: PR-required direct push denial, branch deletion denial, and non-fast-forward denial.
- [x] Branch protection rules management API and repository settings UI.
- [x] Pull request create/list/close/reopen, review history, approval-gated merge, conflict check, stale-ref rejection, compare-and-swap update, and merge recovery state.
- [x] Permission smoke: read-only clone succeeds, push is denied, revoked grant blocks later clone.

### Web application
- [x] Next.js App Router application and R!OTGIT design system.
- [x] Login, logout, and session lookup through server-side route handlers.
- [x] HttpOnly, SameSite=Lax browser session cookie; bearer token is not stored in local storage.
- [x] Live user and repository dashboard, permission-filtered list, and honest empty states.
- [x] Repository creation screen backed by the API.
- [x] Docker Compose internal API routing.
- [x] Repository tree/blob browser, commit history, bounded diff view, issue tracker, label/milestone controls, and branch-protection settings page.

### Operations
- [x] Dockerfiles and single-node Docker Compose profile.
- [x] Non-root API runtime, read-only root filesystem, loopback host port bindings, persistent volume, and health checks.
- [x] GitHub Actions CI covering Rust, web, and Git/auth smoke workflows.

## Remaining work

### Git and security
- [ ] OpenSSH forced-command Git transport and sshd deployment (key registration/revocation API is implemented).
- [x] Branch protection rules API and server-side receive-pack enforcement for Git Smart HTTP (PR-required direct pushes, deletion guard, and non-fast-forward guard).
- [ ] Large pack streaming/backpressure; current HTTP gateway buffers bounded request/response bodies.
- [ ] Git LFS.
- [x] Per-username login failure throttling (10 failures / 15 minutes).
- [ ] IP-aware API rate limiting, web Origin/CSRF defense, token rotation, and full security hardening.
- [ ] PostgreSQL adapter and cross-database integration suite.

### Forge collaboration
- [x] Repository tree/blob browser and commit history.
- [x] Bounded base/head diff view.
- [ ] Blame, syntax highlighting, archive downloads, and large-file pagination.
- [x] Branch protection settings UI for Smart HTTP.
- [ ] Inline review comments, required-check/status integration, multi-approval policy configuration, and polished PR activity timeline.
- [x] Repository Issues: create/list/update/close, comments, repository access checks, and browser tracker UI.
- [x] Repository labels and issue milestones in API and UI.
- [ ] Assignees, notifications, and a richer issue timeline.
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

- Smart HTTP and server-side branch protection are implemented and smoke-tested. SSH is not. Branch rules requiring a pull request intentionally block direct Smart HTTP pushes until a reviewed merge workflow exists.
- Identity and repository-level grants work, but there is no organization/team model yet.
- Branch protection is enforced only for Git Smart HTTP receive-pack; it does not yet cover SSH because SSH transport is not implemented.
- Web supports live repository data, tree/blob browsing, commit history, bounded diffs, issues/comments, labels/milestones, and branch protection settings; pull requests and advanced collaboration are not implemented.
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
