# R!OTGIT — Dependency Policy

This repository commits lockfiles for reproducible Rust and Web dependency resolution. Dependency automation opens reviewed pull requests; it does not auto-merge dependencies.

## Pinned toolchains

| Tool | Pin | Source |
| --- | --- | --- |
| Rust | 1.99 (workspace MSRV 1.94) | `rust-toolchain.toml` / `Cargo.toml` |
| Node.js | 26.13.1 | `.nvmrc` |
| npm | 11.20.0 | `apps/web/.npm-version` |
| Rust dependency graph | locked | `Cargo.lock` |
| Web dependency graph | npm lockfile v3 | `apps/web/package-lock.json` |

SQLx is pinned to 0.9.1 with SQLite-only features; the MySQL RSA feature is not enabled. The committed lockfile resolves `libsqlite3-sys` 0.40.0 (SQLite 3.54.1) and no longer contains `rsa`. Rust CI/API Docker use Cargo `--locked`; Web CI/Docker use `npm ci`. A manifest/lockfile mismatch must fail instead of silently regenerating a lockfile.

## Update policy

Dependabot checks Cargo, npm, and GitHub Actions weekly. Patch/minor updates are grouped by ecosystem; major updates remain separate for explicit review. Dependency PRs are not auto-merged.

## Security and license inventory

A parallel CI lane runs RustSec `cargo-audit` pinned to 0.23.0, `npm audit --audit-level=high`, and a machine-readable Cargo/npm license inventory. CI #265 passed both audits and published the JSON/TSV reports as a 30-day artifact.

Fix advisories or record narrowly justified exceptions; do not hide findings by raising severity thresholds. The product repository is currently marked `UNLICENSED`; the inventory is evidence for a future license decision, not an assertion of redistribution approval. Review unknown license metadata and incompatible licenses before selecting a project license.
