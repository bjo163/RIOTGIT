# Contributing to R!OTGIT

## Working agreement

Build boldly. Keep the core boring. Never trade repository integrity or user trust for a shortcut.

## Before coding
1. Read `README.md` and the relevant architecture/design/security document.
2. Find or create a GitHub Issue with scope, UX rules, security notes, acceptance criteria, tests, dependencies, and non-goals.
3. Confirm the module owner and API/data contract.
4. Identify migration, compatibility, and recovery implications.

## Branches and commits
- Keep `main` releasable.
- Use short-lived branches: `feat/<issue>-short-name`, `fix/<issue>-short-name`, `docs/<issue>-short-name`.
- Prefer Conventional Commit subjects: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `build:`, `ci:`.
- One commit should represent one coherent change.
- Do not rewrite shared history without explicit coordination.

## Code standards
- Rust: typed errors, small modules, no panics on user input, explicit configuration, tracing without secrets.
- TypeScript: strict mode, explicit component contracts, accessible semantics, no implicit `any`.
- API: versioned contracts, stable error shapes, authorization at service boundaries.
- Database: versioned migrations, indexes for query patterns, short transactions.
- Git/process: argument arrays, fixed executable path, sanitized environment, bounded execution.
- UI: design tokens, dark/light support, keyboard access, reduced motion, semantic status.
- Tests: include negative authorization cases and malformed-input cases for boundary code.

## Pull request checklist
- [ ] Linked issue and scope are clear.
- [ ] Acceptance criteria are met.
- [ ] Tests added or updated.
- [ ] Formatting, lint, type checks, and build pass.
- [ ] Database migrations are reversible or have a recovery note.
- [ ] Secrets and private repository contents are not logged.
- [ ] Docs/configuration/examples are updated.
- [ ] Accessibility and responsive states are checked for UI work.
- [ ] Security impact and failure behavior are documented.
- [ ] No unrelated refactor or generated noise.

## Review rules
- Review behavior, permissions, data integrity, operational safety, and maintainability.
- Request changes when acceptance criteria or security boundaries are not met.
- Do not approve based solely on a green build.
