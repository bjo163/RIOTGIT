# R!OTGIT — System Architecture

## 1. Architectural north star

Build a Git-compatible, self-hostable forge with a lean single-node baseline and optional distributed capabilities.

**Core invariant:** a standard bare Git repository is the canonical source of code history. R!OTGIT metadata describes ownership, permissions, collaboration, automation, and network relationships; it never replaces Git's object database.

## 2. Deployment shape

Start with a modular monolith plus isolated execution processes:

```text
Developer clients
  ├─ Git CLI / IDE ── SSH ───────────────┐
  ├─ Git CLI / IDE ── Smart HTTP ────────┤
  ├─ R!OTGIT CLI / TUI ── HTTPS API ─────┤
  └─ Browser ── Next.js Web ── HTTPS API ┤
                                          ▼
                              Reverse proxy / TLS
                                          │
                                Rust + Axum API
     ┌────────────────────────────────────┼─────────────────────────────────┐
     │ Identity / RBAC / Organizations    │ Repository metadata             │
     │ Git transport authorization        │ Pull requests / Reviews         │
     │ Issues / Discussions               │ Webhooks / Audit                 │
     │ Search / Indexing                  │ Network adapters                 │
     └────────────────────────────────────┼─────────────────────────────────┘
                                          │
            ┌─────────────────────────────┼────────────────────────┐
            ▼                             ▼                        ▼
      Native Git CLI                SQLite / PostgreSQL       Job queue
      bare repositories             metadata only             workers
            │                                                      │
      filesystem / volumes                                   isolated runners
            │
      backup / mirror adapter
            │
      optional Radicle adapter (disabled until security review)
```

## 3. Technology baseline

| Area | Decision | Constraint |
| --- | --- | --- |
| API | Rust stable + Axum | Modular monolith first |
| Web | Next.js App Router + TypeScript | Node.js active LTS |
| Database | SQLite default; PostgreSQL supported | Metadata, not Git objects |
| Data access | SQLx or equivalent typed SQL layer | Explicit queries, transactions |
| Git engine | Native Git executable initially | Exact args, no shell, controlled environment |
| Git library | gitoxide incrementally | Use only for verified operations |
| SSH | OpenSSH integration / forced command boundary | No arbitrary shell |
| HTTP Git | Git smart HTTP backend behind authenticated gateway | Strict request validation |
| Storage | Local filesystem | Bare repositories and staging |
| Artifacts | Local volume first; S3-compatible adapter later | Separate from Git object store |
| Realtime | SSE first; WebSocket when bidirectional need exists | Authenticated event stream |
| CLI | Rust CLI, TUI optional | Same API contracts |
| P2P | Radicle adapter | Feature-flagged, reviewed, optional |
| CI | Separate runner process/container | Never execute jobs in API process |

Version numbers are chosen during implementation from official release channels and pinned by lockfiles. Avoid copying stale version pins from examples.

## 4. Bounded modules

Keep module boundaries explicit even while deployed as one process.

- **identity:** users, sessions, tokens, key registration.
- **access:** organizations, teams, roles, repository grants, branch rules.
- **repositories:** metadata, ownership, visibility, lifecycle.
- **git_transport:** authentication and authorization before invoking Git transport.
- **git_storage:** path resolution, bare repository initialization, fsck, pack maintenance.
- **code_view:** tree, blob, commit, diff, archive, blame read models.
- **collaboration:** pull requests, reviews, comments, issues, labels, milestones.
- **automation:** webhooks, delivery attempts, durable job records.
- **audit:** security-relevant and administrative events.
- **network:** mirrors, federation adapters, P2P adapter interfaces.
- **ai_gateway:** opt-in providers, policy checks, context assembly, audit.
- **admin:** configuration, health, backup, restore, operational controls.

Modules communicate through typed domain interfaces, not direct database table access across boundaries.

## 5. Git storage and transport

### Repository identity
- Store an opaque repository ID in metadata.
- Derive storage path from the ID, never directly from user-supplied owner/name.
- Keep owner/name as mutable metadata with uniqueness constraints.
- Store a canonical filesystem root outside the web root.

### Repository lifecycle
1. Authorize create request.
2. Create metadata row in a pending state.
3. Create a temporary bare repository under a controlled staging directory.
4. Initialize with safe, explicit Git configuration.
5. Validate repository with `git fsck`.
6. Atomically move to the final ID-derived path.
7. Mark metadata ready and emit an audit event.
8. On failure, clean staging data and mark/rollback the pending record.

### Transport rules
- SSH uses a forced command or equivalent restricted transport entry point; interactive shell access is never granted to Git-only keys.
- Smart HTTP authenticates the request before passing the exact service operation to the Git backend.
- Only `git-upload-pack` and `git-receive-pack` are exposed as intended operations.
- Push authorization checks repository write permission and branch protection before receive-pack proceeds.
- Never interpolate a path, username, ref, or service name into a shell command.
- Git execution uses fixed executable paths, argument arrays, controlled working directories, sanitized environment, timeouts, and output limits.
- Disable unsafe repository config and hooks by default. Server-side hooks are trusted platform code, not user-uploaded scripts.

### Git compatibility
The first release should delegate transport and object handling to native Git. gitoxide may accelerate parsing, indexing, and selected server-side operations, but cannot become a compatibility dependency until conformance tests pass.

## 6. Metadata model

Initial entities:
- User, Session, SSHKey, AccessToken
- Organization, Team, TeamMember, RoleBinding
- Repository, RepositoryCollaborator, BranchRule
- PullRequest, PullRequestReview, PullRequestComment
- Issue, IssueComment, Label, Milestone
- Webhook, WebhookDelivery
- AuditEvent, BackgroundJob
- Mirror, NetworkPeer (later)
- AIProviderConfig, AIRequestAudit (later; secrets stored separately)

Keep Git commits, trees, blobs, tags, and refs in Git itself. Store indexed read models only when they can be rebuilt from the repository.

## 7. Database strategy

### SQLite profile
- One database file on persistent local storage.
- WAL mode where supported and operationally validated.
- Foreign keys enabled.
- Busy timeout configured.
- Single-node writes are kept short; do not hold database transactions while invoking Git or a remote service.

### PostgreSQL profile
- Same logical schema and migration sequence.
- Use PostgreSQL for multi-worker or concurrent write-heavy deployments.
- No application behavior may rely on SQLite-only SQL semantics unless isolated behind a repository adapter.

### Migrations
Use versioned, forward-only schema migrations for relational metadata. Do not claim "no migrations"; schema evolution must be reproducible. Git repository storage is not migrated with SQL schema.

## 8. Optional network modes

### Phase A — mirrors
- One-way or explicitly configured bidirectional Git mirror.
- Show last successful sync and conflict/error state.
- Never treat a mirror as an authoritative collaboration node by default.

### Phase B — federation
- Versioned service-to-service contract.
- Node identity, trust, visibility, sync direction, replay protection, and revocation are explicit.
- Keep federation distinct from P2P semantics.

### Phase C — P2P adapter
- Integrate with a supported Radicle release instead of inventing a new gossip or identity protocol.
- Preserve separate ownership and trust semantics.
- Gate all network exposure behind configuration and a kill switch.
- Verify current protocol security notices and transport protections before enabling.
- The platform must continue working with P2P disabled.

## 9. AI integration boundary

AI is a provider adapter behind the API:
- explicit per-organization and per-repository opt-in;
- least-privilege scoped tokens;
- context size and file allowlists;
- secret and credential redaction;
- no CI secrets in model context;
- request/response audit metadata without unnecessary source retention;
- no code transmission to remote providers unless policy permits it;
- agent changes arrive as branches/patches and must pass normal review rules.

## 10. Operational model

- `api`: web/API and metadata operations.
- `web`: Next.js frontend; can be colocated or deployed separately.
- `git-transport`: restricted SSH/HTTP boundary, may begin within API deployment but must be isolated by process/permissions.
- `worker`: webhook, indexing, and maintenance jobs.
- `runner`: isolated CI execution; separate host/container security boundary.
- `storage`: persistent repository and artifact volumes.
- `db`: SQLite volume or PostgreSQL service.

Health endpoints:
- `/health/live`: process is alive.
- `/health/ready`: required dependencies are available.
Never expose secrets, absolute paths, or internal topology in health responses.

## 11. Architecture decision log

| ID | Decision | Status |
| --- | --- | --- |
| ADR-001 | Native Git is canonical; no custom Git format | Accepted |
| ADR-002 | Rust Axum modular monolith for API | Accepted |
| ADR-003 | Next.js App Router for web | Accepted |
| ADR-004 | SQLite first, PostgreSQL supported | Accepted |
| ADR-005 | Radicle through optional adapter, not fork | Accepted |
| ADR-006 | Runners execute outside API process | Accepted |
| ADR-007 | Versioned SQL migrations for metadata | Accepted |

## 12. Architecture exit criteria

- Single-node install can create, clone, push, fetch, and read a repository using normal Git.
- Authorization is applied consistently across web, SSH, and HTTP transport.
- Repository paths cannot escape the storage root.
- Database restore and repository restore are documented and tested together.
- Optional workers and network integrations can be disabled without breaking core Git hosting.
