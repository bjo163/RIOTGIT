# R!OTGIT — Implementation Status

Last updated: 2026-11-20

## Verified baseline and current main snapshot

The last known fully green main-branch baseline is commit `35d5c21e5feb188f5678ef77dc8e44c21083e003` (CI run #233):
https://github.com/bjo163/RIOTGIT/actions/runs/43494098100

The current main snapshot inspected on 2026-11-20 is commit `d9e6257d4072a7b23917a04068d207fea4042cb8` (CI run #243):
https://github.com/bjo163/RIOTGIT/actions/runs/43497718230

Run #243 is **red**: Web checks passed; Rust checks stopped at `cargo fmt --all -- --check` because two expressions in `apps/cli/src/main.rs` were not rustfmt-formatted; the OpenSSH smoke job failed its final revocation assertion. Code inspection found that the smoke script initially exported the administrator `GIT_SSH_COMMAND`, then reassigned that exported variable to the reader identity. Bash preserved the export attribute, so later administrator-only branch-protection and revocation assertions could run with the reader key. The stabilization branch keeps administrator and reader commands separate and explicitly selects the identity for each Git operation.

The first full stabilization verification passed in CI run #245 on PR #18: Rust format/check/clippy/tests/build and Smart HTTP round-trip passed; Web typecheck/lint/build passed; API/web/SSH images built and started; web readiness passed; and OpenSSH clone/push, read-only denial, branch-protection denial, no-shell, and key revocation passed. This is branch/PR evidence, not a green result on `main`. PR CI #253 on commit `315dc2a8b2382efc497b9bd5babc6570aa79086a` passed all three jobs. In addition to the #245 coverage, #253 verified Cargo `--locked`, npm `ci`, exact Node/npm pins, API/Web/SSH container build and startup, same-node data + SSH host-key backup/restore, API/Web readiness after restore, SSH host-key continuity, and a second SSH clone/push/revocation smoke. This is branch/PR evidence, not a green result on `main`.

This is CI evidence only; it is not a substitute for deployment-specific SSH host-key review, an off-host restore drill, upgrade/rollback validation, or a production security assessment. This document intentionally separates committed implementation from tested/production-ready capability.

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
- [x] Optional OpenSSH gateway with database-backed authorized keys, key-bound forced command, restricted Git-only services, and a protected receive-pack hook; PR CI #245 verifies clone/push, read-only push denial, branch protection, no interactive shell, and revocation with isolated admin/reader SSH identities.
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
- [x] GitHub Actions Rust/web CI, Git Smart HTTP smoke coverage, OpenSSH end-to-end smoke coverage, API/web/SSH image build/start, web readiness, and same-node backup/restore smoke coverage are implemented. PR CI #253 passed all three jobs; this does not change the failing state of main until PR #18 is merged.
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
- [x] Commit `Cargo.lock`, `apps/web/package-lock.json`, and pin Node/npm versions on the stabilization branch.
- [x] Same-node Compose backup/restore smoke passed in PR CI #253, including data and SSH host-key volumes, post-restore API/Web readiness, host-key continuity, and SSH smoke. A separate-host/off-host restore drill is still required.
- [ ] Validate upgrade/rollback against data snapshots.
- [x] API and SSH container build/start/health/transport smoke test in CI.
- [x] Build and start the web container in CI and wait for `/login` readiness (passed in PR CI #253).
- [ ] Load, soak, disk-full, crash-recovery, and large-pack tests.
- [ ] Full security review and release checklist.
- [ ] Verify SSH gateway host-key persistence, authorized-key revocation, no-shell restrictions, and hook execution on the target host.

## Important limitations

- Git Smart HTTP and the optional SSH path passed PR CI #253 after the SSH smoke test was corrected to use separate admin/reader identities and unique per-invocation fixtures. The current main branch has not yet received that fix.
- Branch protection is covered on both Smart HTTP and SSH receive-pack paths by the current CI smoke checks; broader shared-policy regression coverage is still required.
- Login throttling is per normalized username, not IP-aware. It reduces repeated guessing but can be used to temporarily throttle a known username.
- PR CI #253 covers locked Rust checks, npm ci/typecheck/lint/build, Docker image build/start, Smart HTTP round-trip, SSH clone/push/security tests, and same-node backup/restore with host-key continuity.
- The CLI is an initial terminal interface, not a TUI or full feature-complete admin client.
- Same-node data and SSH host-key backup/restore passed PR CI #253. A separate-host/off-host restore and upgrade/rollback drill have not yet been completed.
- R!OTGIT is not yet a fully hardened production Git forge. PostgreSQL parity, durable jobs/webhooks, isolated CI runners, P2P/federation, and AI agent execution remain unfinished.

## Local run

API:

```bash
cp .env.example .env
# Set RIOTGIT_API_TOKEN, RIOTGIT_ADMIN_USERNAME and RIOTGIT_ADMIN_PASSWORD.
set -a && . ./.env && set +a
cargo run --locked -p riotgit-api
```

Web (another terminal):

```bash
cd apps/web
npm ci
npm run dev
```

CLI:

```bash
cargo run --locked -p riotgit -- --help
```

See [API README](../apps/api/README.md), [CLI guide](../apps/cli/README.md), [Deployment](DEPLOYMENT.md), and [Backup/Restore](BACKUP-RESTORE.md).

## P0–P2 delivery snapshot (2026-11-14)

| Priority / issue | Current state | Evidence / remaining gate |
| --- | --- | --- |
| P0 #1 API contracts and boundaries | Foundation exists; not closed | API error conventions and architecture docs exist; typed config, request IDs, and complete module extraction still need verification. |
| P0 #2 metadata/data access | SQLite foundation implemented; not closed | Versioned SQLite migrations exist; PostgreSQL adapter and cross-database tests are not implemented. |
| P0 #3 repository lifecycle | Core path implemented; not closed | Native bare repo lifecycle exists; quota/size accounting, symlink adversarial tests, archive/delete/maintenance coverage remain. |
| P0 #4 SSH transport | Implemented; PR CI verified | CI #253 passed clone/push, read-only denial, branch-protection denial, no-shell, and revocation using isolated identities; deployment host-key review remains. |
| P0 #17 dependency locking | Locked path verified on stabilization branch | Cargo.lock/package-lock.json are committed; CI and Docker passed Cargo `--locked` and npm `ci` with Node 26.13.1/npm 11.20.0 in PR CI #253. Dependency audit/update automation still needs completion. |
| P1 #5–#10 forge and identity | Partially implemented | HTTP, browser, collaboration, branch protection, issues, and SSH key registry exist; large-pack streaming, org/team RBAC, inline review, richer timeline, UI completeness, and E2E coverage remain. |
| P1 #16 operations | Partial; same-node recovery verified | API/web/SSH build/start, health/readiness, backup/restore of data and SSH host-key volumes, post-restore health, and host-key continuity passed in CI #253; separate-host restore and upgrade/rollback remain. |
| P2 #11 CLI | Initial implementation exists | Core login/repository/SSH-key commands exist; profiles, OS credential vault, JSON output, status, and dedicated CLI integration tests remain. |
| P2 #12 jobs/webhooks | Not implemented | No durable outbox/worker or signed webhook delivery; SSRF-safe delivery must be designed before enabling outbound requests. |
| P2 #13 CI runner | Not implemented | No isolated workflow runner; must remain a separate execution boundary from the API. |

P2 is **not complete**. Main is still at the failing CI snapshot #243 until PR #18 is merged. The stabilization branch has a fully green CI run #253, including locked dependency installs and a same-node recovery drill. Separate-host recovery, upgrade/rollback, dependency audit automation, and the remaining P0–P3 roadmap items are still open. Do not close issues until their full acceptance criteria and verification gates pass.