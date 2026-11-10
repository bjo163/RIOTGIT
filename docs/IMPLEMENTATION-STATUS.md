# R!OTGIT — Implementation Status

Last updated: 2026-11-11

## Implemented in repository

- [x] Product narrative and positioning.
- [x] Extreme visual identity, color tokens, typography roles, logo direction.
- [x] Initial symbol and wordmark SVG source assets.
- [x] Architecture baseline and security baseline.
- [x] Delivery roadmap and contribution standards.
- [x] Rust workspace with minimal Axum API service.
- [x] API process liveness and readiness endpoints.
- [x] API metadata endpoint.
- [x] Next.js App Router project scaffold.
- [x] Responsive R!OTGIT cockpit design preview.
- [x] Reduced-motion CSS behavior and system font fallbacks.
- [x] GitHub Actions quality workflow.
- [x] Issue and pull request templates.
- [x] 16 implementation issues created.

## Not yet implemented

- [ ] Database connectivity and schema migrations.
- [ ] Accounts, sessions, tokens, SSH key registration, RBAC.
- [ ] Repository metadata and bare Git storage lifecycle.
- [ ] Authenticated SSH and smart HTTP transport.
- [ ] Live repository data in the web app; current cockpit uses clearly labeled sample data.
- [ ] Code browser, commits, diffs, issues, pull requests, reviews.
- [ ] CLI/TUI.
- [ ] Durable background queue and webhooks.
- [ ] Isolated CI runners.
- [ ] Mirror, federation, and Radicle/P2P adapter.
- [ ] AI provider gateway and scoped agents.
- [ ] Production Compose deployment, backup/restore, and operational runbooks.

## Verification status

The source files and GitHub workflow have been committed. A successful build/test run has not yet been confirmed in this work session. Do not describe the API or web UI as production-ready until CI passes and the integration/security gates in the relevant issues are complete.

## Current starting commands

API:

```bash
cargo run -p riotgit-api
```

Web:

```bash
cd apps/web
npm install
npm run dev
```

The API binds to `127.0.0.1:8787` by default. The web page is currently a design preview; live API integration is tracked in Issue #6.
