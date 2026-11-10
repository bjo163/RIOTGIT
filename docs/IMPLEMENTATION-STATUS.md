# R!OTGIT — Implementation Status

Last updated: 2026-11-11

## Implemented and committed

- [x] Product narrative, positioning, brand identity, logo SVGs, and product design system.
- [x] Architecture baseline, threat model, delivery roadmap, contribution standards, and issue templates.
- [x] Rust workspace and Axum API bootstrap.
- [x] SQLite database initialization with versioned migration.
- [x] Database-backed repository metadata.
- [x] Native Git bare-repository initialization with generated opaque storage IDs and staging activation.
- [x] Bearer-token guard for repository administration and Git smart HTTP (instance-wide admin credential).
- [x] Repository create, list, and lookup API.
- [x] Liveness and database/storage readiness endpoints.
- [x] Next.js App Router cockpit design preview.
- [x] GitHub Actions Rust and web quality workflow.

## Verified at foundation stage

The starter UI/API commit passed GitHub Actions:
- Rust formatting, check, Clippy, and tests.
- Web dependency install, TypeScript, ESLint, and Next.js production build.

See the CI run: https://github.com/bjo163/RIOTGIT/actions/runs/42872536139

## Implemented in the current Git Core increment

Verified by CI run https://github.com/bjo163/RIOTGIT/actions/runs/42876569801 on commit `a55d7670890b80b6e5dc8a301b37ddcc83a64ff3`:
- SQLite-backed repository metadata.
- Native Git bare repository creation via controlled argument-based process invocation.
- Duplicate repository conflict response.
- Name validation against traversal and unsafe path characters.
- Configurable storage/database/Git executable.
- API token guard for repository management.

## Not yet implemented

- [ ] Per-user authentication, sessions, SSH key lifecycle, organization/team RBAC.
- [ ] Git SSH forced-command transport.
- [ ] Per-user/read-only Git permissions and branch-protection checks at receive-pack.
- [ ] Large-pack streaming and bounded memory behavior for very large repositories.
- [ ] SSH forced-command transport and key revocation.
- [ ] Authenticated Git HTTP integration test for read-only vs write permissions (current smoke test uses the instance-admin token).
- [ ] Git LFS.
- [ ] Live web API integration and real repository views.
- [ ] Code tree/blob/commit/diff browser.
- [ ] Issues, pull requests, reviews, branch protections, merge engine.
- [ ] CLI/TUI.
- [ ] Durable jobs, webhooks, isolated CI runners.
- [ ] Mirrors, federation, Radicle/P2P.
- [ ] AI gateway and scoped agents.
- [ ] Production deployment, backup/restore drills, upgrade/rollback runbook.
- [ ] Reproducible committed lockfiles.

The API token is a temporary single-instance bootstrap guard, not a substitute for user sessions or granular authorization. The web cockpit remains a clearly labeled design preview and does not show live repository data yet.

## Current API run

```bash
cp .env.example .env
# Set RIOTGIT_API_TOKEN to a long random secret.
cargo run -p riotgit-api
```

See [API README](../apps/api/README.md) for PowerShell instructions, routes, and examples.

## Status language

Do not describe R!OTGIT as a complete Git forge or production-ready product until authenticated Git transport, identity/RBAC, collaboration workflows, backup/restore, and the corresponding integration/security gates pass.
