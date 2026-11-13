# R!OTGIT — Implementation Status

Last updated: 2026-11-14

## Verified baseline

The current main-branch commit `35d5c21e5feb188f5678ef77dc8e44c21083e003` passed the complete CI pipeline:
https://github.com/bjo163/RIOTGIT/actions/runs/43494098100

The run passed Web checks, Rust checks, and the OpenSSH Git transport smoke test. This verifies the CI path only; it does not replace a separate-host restore drill, deployment-specific SSH host-key review, or production security assessment. This document intentionally separates committed implementation from tested/production-ready capability.

## Implemented and committed

### Product and engineering foundation
- [x] Product narrative, positioning, brand identity, SVG logo assets, and product design system.
- [x] Architecture, threat model, roadmap, contribution standards, issue templates, and PR template.
- [x] Rust workspace and Axum API.
- [x] Structured error responses and liveness/readiness routes.
- [x] SQLite initialization with versioned migrations.

### Identity and access
- [x] First-run administrator bootstrap.
- [x] Argon2 password hashing and verification.
- [x] User login, revocable seven-day bearer sessions, and logout.
- [x] Admin-managed user creation with admin/member roles.
- [x] Repository-level read/write/admin grants.
- [x] Read/write permissions applied to Git Smart HTTP.
- [x] Per-username login failure throttle (10 failed attempts in 15 minutes).
- [x] SSH public-key registration/list/revocation API with key type/blob validation and SHA-256 fingerprints.
- [x] Optional OpenSSH gateway with database-backed authorized keys, key-bound forced command, restricted Git-only services, and a protected receive-pack hook; CI verifies clone, push, branch-protection denial, no interactive shell, and key revocation.
- [x] Same-origin guard for mutating Next.js web API routes.
- [x] Emergency instance-admin token.

### Git core and collaboration
- [x] SQLite-backed repository metadata and native bare repository storage.
- [x] Opaque storage IDs, staged creation, validation, and safe repository-name checks.
- [x] Authenticated Git Smart HTTP discovery, upload-pack, and receive-pack.
- [x] Bounded Smart HTTP request bodies (64 MiB) and backend timeouts (120 seconds).
- [x] Git CLI smoke round-trip for create, push, clone, and read-only permission checks.
- [x] Branch protection API/UI and Smart HTTP receive-pack enforcement for protected direct pushes, deletion, and non-fast-forward updates.
- [x] Repository tree/blob browser, commit history, and bounded base/head diffs.
- [x] Issues, comments, labels, milestones, assignment/clear operations, and web tracker.
- [x] Pull request create/list/close/reopen, reviews, approval-gated merge, conflict/stale-ref checks, compare-and-swap ref updates, and merge-recovery state.

### Web and terminal clients
- [x] Next.js App Router application with R!OTGIT design tokens and responsive dashboard.
- [x] Login/logout/session UI with HttpOnly SameSite cookie handling.
- [x] Live repository listing and repository creation.
- [x] Repository browser, issues, PR reviews/merge, labels/milestones, and branch protection settings pages.
- [x] Rust CLI scaffold: login, whoami, repository list/create/clone, SSH key list/add/remove, and logout.
- [x] CLI stores its local session file with restrictive Unix permissions; OS credential-vault integration and TUI remain future work.

### Operations
- [x] Rust and web Dockerfiles plus single-node Compose profile.
- [x] Non-root runtime users, read-only container root filesystems, loopback host bindings, persistent data volume, and health checks.
- [x] GitHub Actions Rust/web CI, Git Smart HTTP smoke coverage, and OpenSSH end-to-end smoke coverage (latest run passed).
- [x] Backup and restore scripts for the complete single-node data volume.
- [x] Backup/restore runbook with pre-restore safety snapshot and checksum instructions.

## Remaining work

### Git and security
- [ ] Add explicit SSH read-only-key denial coverage, broader OpenSSH/Git interoperability fixtures, and deployment-specific host-key/security review.
- [ ] Large-pack streaming/backpressure; Smart HTTP currently buffers bounded request/response bodies.
- [ ] Git LFS.
- [ ] IP-aware API rate limiting, reverse-proxy trust configuration, token rotation, security review, and full CSRF/proxy deployment review.
- [ ] Validate SSH key parser against a full adversarial key fixture set.
- [ ] PostgreSQL adapter and cross-database integration suite.

### Forge collaboration
- [ ] Blame, syntax highlighting, archive downloads, large-file pagination.
- [ ] Inline review comments, required-check/status integration, multi-approval policy configuration, and richer PR activity timeline.
- [ ] Assignees, notifications, richer issue timeline, organization/team hierarchy, and SSO.
- [ ] Interactive TUI and OS credential-vault integration.

### Automation and network
- [ ] Durable background queue/outbox and signed webhook delivery with SSRF protection.
- [ ] Isolated CI runner and a documented workflow subset.
- [ ] Mirrors, federation contracts, and optional Radicle/P2P adapter.
- [ ] Opt-in AI provider gateway and repository-scoped agent execution.

### Production readiness
- [ ] Commit Cargo/npm lockfiles and switch CI/container installs to frozen/locked mode (P0 #17; currently unresolved).
- [ ] Run an actual restore drill on a separate test volume/host; scripts are committed but not yet verified by a restore exercise.
- [ ] Validate upgrade/rollback against data snapshots.
- [x] API and SSH container build/start/health/transport smoke test in CI.
- [ ] Add full web container startup smoke test in CI.
- [ ] Load, soak, disk-full, crash-recovery, and large-pack tests.
- [ ] Full security review and release checklist.
- [ ] Verify SSH gateway host-key persistence, authorized-key revocation, no-shell restrictions, and hook execution on the target host.

## Important limitations

- Git Smart HTTP and the optional SSH clone/push path are covered by the latest successful CI smoke run; deployment-specific host-key persistence and broader compatibility still need review.
- Branch protection is covered on both Smart HTTP and SSH receive-pack paths by the current CI smoke checks; broader shared-policy regression coverage is still required.
- Login throttling is per normalized username, not IP-aware. It reduces repeated guessing but can be used to temporarily throttle a known username.
- The current CI covers Rust/web checks, Smart HTTP round-trip, and SSH clone/push/security smoke checks. CLI behavior is compiled and checked by Rust workspace CI; backup/restore still needs an actual isolated restore drill.
- The CLI is an initial terminal interface, not a TUI or full feature-complete admin client.
- Backup and restore scripts are present, but a successful restore drill has not yet been recorded.
- R!OTGIT is not yet a fully hardened production Git forge. PostgreSQL parity, durable jobs/webhooks, isolated CI runners, P2P/federation, and AI agent execution remain unfinished.

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
npm install
npm run dev
```

CLI:

```bash
cargo run -p riotgit -- --help
```

See [API README](../apps/api/README.md), [CLI guide](../apps/cli/README.md), [Deployment](DEPLOYMENT.md), and [Backup/Restore](BACKUP-RESTORE.md).
