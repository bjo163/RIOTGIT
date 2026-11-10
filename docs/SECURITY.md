# R!OTGIT — Security Baseline

## Security posture

R!OTGIT is a network-facing service that stores source code and may eventually execute untrusted build instructions. Treat repository content, Git configuration, diffs, archives, webhook destinations, package metadata, and CI jobs as hostile input.

Security work is a release requirement, not a post-MVP polish task.

## Assets to protect

- repository contents and Git history;
- user credentials, sessions, SSH keys, and access tokens;
- organization and repository permissions;
- branch protection and review decisions;
- webhook secrets and delivery records;
- runner host, credentials, network, and artifacts;
- backups and encryption keys;
- federation/P2P identities and trust state.

## Trust boundaries

1. Internet/client to reverse proxy.
2. Reverse proxy to web/API.
3. API to Git transport process.
4. API to filesystem and database.
5. API/worker to outbound webhook targets.
6. Scheduler to isolated CI runner.
7. Node to node for mirrors/federation/P2P.
8. AI gateway to external model providers.

Every boundary requires authentication where appropriate, authorization, input validation, bounded resource use, and auditability.

## Mandatory controls

### Identity and sessions
- Hash passwords with a current, memory-hard password hashing algorithm.
- Store sessions server-side or use short-lived signed tokens with revocation strategy.
- Secure, HttpOnly, SameSite cookies; CSRF protection for cookie-authenticated state changes.
- Rate-limit login, token creation, password reset, and SSH key operations.
- Redact credentials and tokens from logs.

### Authorization
- Deny by default.
- Enforce permission checks at the service boundary, not only in UI routes.
- Test owner, collaborator, team, organization, public, private, and archived states.
- Re-check authorization at Git transport invocation time.
- Branch protection cannot be bypassed by webhooks or AI agents.

### Filesystem and process safety
- Derive paths from opaque repository IDs and canonicalize under the configured storage root.
- Reject path traversal, symlink escape, unexpected file types, and race-prone path assumptions.
- Use process APIs with argument arrays; never use `sh -c` with user-controlled values.
- Fix executable paths and use a sanitized environment.
- Apply timeouts, memory/output caps, and cancellation.
- Run API under a dedicated unprivileged OS account.
- Do not execute repository hooks or CI jobs in the API process.

### Git-specific safety
- Use isolated bare repositories.
- Disable or explicitly allowlist dangerous Git configuration and hooks.
- Validate repository state and object integrity.
- Bound packfile size, request size, concurrent pushes, archive generation, diff size, and tree traversal.
- Never trust filenames, refs, commit messages, patch content, or submodule URLs.
- Prevent archive path traversal and symlink extraction.
- Treat LFS pointers and external object URLs as untrusted.

### Webhook and outbound request safety
- Defend against SSRF: block loopback, link-local, private, metadata, and disallowed schemes unless explicitly allowed by admin policy.
- Re-resolve and validate DNS at connection time; defend against DNS rebinding.
- Sign webhook payloads; rotate and redact secrets.
- Use bounded retries, request timeouts, and delivery audit.
- Do not follow redirects to forbidden destinations.

### CI runner safety
- Runner is a separate trust boundary and should run with no privileged container mode.
- Use ephemeral workspaces and short-lived scoped credentials.
- Default-deny network egress; allowlist required endpoints.
- Apply CPU, memory, disk, process, and runtime limits.
- Do not mount API secrets, host Docker socket, or repository storage write access into jobs.
- Destroy workspaces after execution and define artifact scanning/retention policy.
- Untrusted PR workflows require stronger isolation than trusted branch workflows.

### AI safety
- AI is disabled until configured and opted in.
- Never include secrets, environment values, or private files outside the approved scope.
- Treat repository text as untrusted prompt content; system policy is not overridden by source comments.
- Agent changes are patches/branches and go through normal authorization, checks, and review.
- Record provider, model, repository scope, actor, and policy decision without retaining unnecessary code.

### Backups
- Encrypt off-host backups.
- Keep repository data and metadata snapshots consistent.
- Document key recovery and access controls.
- Test restore regularly; an untested backup is not a verified recovery path.

## Release gates

A release must not enable a capability unless:
- threat model is documented;
- authorization tests cover positive and negative cases;
- malformed input and resource exhaustion tests exist;
- secrets are absent from logs and errors;
- dependency and container scanning pass;
- backup/restore has a tested runbook;
- known limitations are disclosed.

## Security incident response

- Provide an admin kill switch for Git transport, runners, outbound webhooks, and network adapters.
- Revoke user tokens and SSH keys.
- Preserve audit and relevant forensic records.
- Rotate webhook, database, and signing secrets where required.
- Publish a clear security advisory and remediation path for confirmed vulnerabilities.
