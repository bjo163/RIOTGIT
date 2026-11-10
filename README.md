# R!OTGIT

> **THE UNDERGROUND CODE NETWORK**  
> Break the rules. Own your code.

R!OTGIT is a self-hosted, local-first and extensible code collaboration platform. It combines the dependable Git workflow developers already know with a bold web forge, terminal-first tooling, automation, and an optional path toward peer-to-peer collaboration.

**R!OTGIT is not a new Git implementation.** Git compatibility is a product invariant. Native Git repositories and protocols remain the source of truth; platform services add identity, collaboration, policy, automation, and discovery around them.

![Project status](https://img.shields.io/badge/status-foundation%20phase-ccff33?style=flat-square)
![Architecture](https://img.shields.io/badge/architecture-modular%20monolith-ff5a1f?style=flat-square)
![License](https://img.shields.io/badge/license-TBD-lightgrey?style=flat-square)

## The idea

- **Sovereign:** self-host your code and control your data.
- **Compatible:** use normal Git clients over SSH and HTTPS.
- **Local-first:** keep repositories useful and portable outside the web app.
- **Lean:** start as one API process, one web app, one database, and filesystem storage.
- **Extensible:** add runners, mirrors, federation, P2P, and AI through explicit adapters.
- **Human-scale:** make the terminal, API, and browser equal entry points.

## Product surface

| Surface | Purpose |
| --- | --- |
| Web forge | Repositories, code browser, commits, diffs, pull requests, issues, releases |
| Git transport | Native Git over SSH and smart HTTP |
| CLI / TUI | Fast repository, remote, and node workflows |
| Automation | Webhooks, background jobs, isolated CI runners |
| Network | Mirrors first; federation and P2P behind adapters |
| AI gateway | Opt-in provider integrations and bounded repository context |

## Architecture decisions

1. Native Git is authoritative for commits, refs, objects, hooks, and repository integrity.
2. Rust + Axum is the platform API and domain service boundary.
3. Next.js App Router is the web application. Use an active Node.js LTS release.
4. SQLite is the default small-install database; PostgreSQL is the production scale-out option.
5. Filesystem storage is the default for bare repositories. Object storage is for artifacts and backups.
6. Git transport is isolated from the browser/API authorization layer. No shell interpolation or arbitrary command execution.
7. Radicle is an optional integration, not a forked storage model. Keep it disabled until compatibility and security reviews pass.
8. Start as a modular monolith. Split workers/runners only when isolation or scale requires it.

See [Architecture](docs/ARCHITECTURE.md), [Product](docs/PRODUCT.md), [Brand](docs/BRAND.md), and [Design System](docs/DESIGN-SYSTEM.md).

## Repository map

```text
apps/
  api/              Rust Axum API (foundation)
  web/              Next.js web forge (planned)
crates/             Shared Rust domain and infrastructure crates (planned)
docs/               Product, brand, architecture, security, roadmap
assets/brand/       Source SVG logo assets
deploy/             Compose, reverse proxy, backup and service examples (planned)
```

## Development status

The repository currently contains the product foundation and a minimal API bootstrap. Feature work is tracked in GitHub Issues. Documentation is a specification, not evidence that a feature is already implemented.

## Quick start

The initial API scaffold requires Rust stable and Cargo:

```bash
cargo run -p riotgit-api
```

The API binds to `127.0.0.1:8787` by default. Override with `RIOTGIT_BIND=127.0.0.1:8787`.

The web app, Git transport, database, CLI, runners, and network adapters are tracked as implementation tasks and are not claimed as operational yet.

## Engineering rules

- Keep main buildable; small, reviewable commits.
- Define contracts before parallel implementation.
- Never store Git objects or repository contents in the relational database.
- Use parameterized SQL and safe process APIs; never concatenate user input into shell commands.
- Treat repository paths, hooks, archives, diffs, LFS objects, and CI artifacts as untrusted input.
- Default to private networking and least privilege.
- Every feature issue must include spec, UX/design rules, security notes, acceptance criteria, tests, and explicit non-goals.
- Do not advertise P2P, Actions compatibility, or production readiness until verified by integration tests.

## Brand

**R!OTGIT — THE UNDERGROUND CODE NETWORK**  
**BREAK THE RULES. OWN YOUR CODE.**

The extreme visual system is intentionally loud, raw, high-contrast, and accessible. See [Brand](docs/BRAND.md) and [Design System](docs/DESIGN-SYSTEM.md).

## License

License selection is intentionally open pending a repository-level decision. Do not add a license header or claim open-source licensing until the owner selects and commits a license.
