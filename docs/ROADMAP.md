# R!OTGIT — Delivery Roadmap

## Delivery strategy

Ship a useful single-node forge first. Preserve compatibility and portability. Add network distribution and AI only after the core permission and repository lifecycle are reliable.

## Phase 0 — Foundation
- Product narrative, brand system, architecture, threat model.
- Repository conventions, Rust workspace, web app scaffold, CI checks.
- API health endpoints and configuration contract.

**Exit:** reproducible local development and green quality gates.

## Phase 1 — Git Core
- SQLite metadata schema.
- Repository create/list/get/archive lifecycle.
- Bare repository storage with safe path derivation.
- Native Git clone/fetch/push over authenticated transport.
- Repository integrity and backup primitives.

**Exit:** standard Git clients can clone, push, fetch, and recover repositories.

## Phase 2 — Identity and access
- Local account bootstrap.
- Login/session/token lifecycle.
- SSH public key management.
- Organizations, teams, roles, repository grants.
- Branch rules and audit events.

**Exit:** web, SSH, and HTTP enforce the same permission model.

## Phase 3 — Web forge
- Application shell and design tokens.
- Dashboard and repository pages.
- Source tree/blob viewer, commits, refs, diff.
- Search and keyboard navigation.
- Dark/light themes and responsive states.

**Exit:** user can discover and inspect repository data without leaving the browser.

## Phase 4 — Collaboration
- Issues, labels, milestones, comments.
- Pull requests, patch comparison, review submissions.
- Merge strategies and branch protection.
- Notifications and activity feed.

**Exit:** normal team code review workflow works end-to-end.

## Phase 5 — Developer experience
- CLI authentication and repository commands.
- SSH/TUI workflows.
- API and webhook documentation.
- Import/export and mirror support.

**Exit:** common operations are scriptable and terminal-friendly.

## Phase 6 — Automation
- Durable job queue.
- Webhook signing, retries, delivery inspector.
- CI workflow subset and isolated runners.
- Artifacts, logs, retention, resource policies.

**Exit:** approved workflows execute within tested isolation boundaries.

## Phase 7 — Distributed network
- Read-only and push mirrors.
- Peer/node identity and trust model.
- Radicle compatibility spike.
- Federation contract and sync policies.
- Conflict, revoke, recovery, and offline tests.

**Exit:** replication behavior is observable, recoverable, and secure; P2P remains optional.

## Phase 8 — AI-native workflows
- Provider gateway and repository context index.
- Opt-in policy and redaction.
- AI issue planning and code review suggestions.
- Agent-created branches and patch workflow.
- Audit and cost limits.

**Exit:** agents can propose changes without bypassing standard controls.

## Phase 9 — Production hardening
- Upgrade and rollback strategy.
- Observability, metrics, traces, audit export.
- Backup/restore drills.
- Rate limits, quotas, load and soak testing.
- Security review and release packaging.

**Exit:** supported deployment profile passes documented release gates.

## Parallelization rules

- Work in parallel only after module contracts and ownership are defined.
- UI may use mock fixtures while API contracts stabilize.
- Git transport and repository storage must share the same authorization and path-safety contract.
- P2P research must not block single-node delivery.
- CI runner work must not be merged without an isolation review.
- Keep one integration owner responsible for cross-module compatibility.

## Definition of done

Every phase requires:
1. implementation merged into `main`;
2. unit and integration tests;
3. updated docs and examples;
4. security and permission checks;
5. no unresolved blocking regressions;
6. a reproducible local verification command;
7. explicit list of deferred work.
