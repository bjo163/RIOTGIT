# R!OTGIT — Dependency Policy

This repository commits lockfiles for reproducible Rust and Web dependency resolution. Dependency automation opens reviewed pull requests; it does not auto-merge dependencies.

## Pinned toolchains

| Tool | Pin | Source |
| --- | --- | --- |
| Rust | 1.99 | `rust-toolchain.toml` |
| Node.js | 26.13.1 | `.nvmrc` |
| npm | 11.20.0 | `apps/web/.npm-version` |
| Rust dependency graph | locked | `Cargo.lock` |
| Web dependency graph | npm lockfile v3 | `apps/web/package-lock.json` |

Rust CI/API Docker use Cargo `--locked`; Web CI/Docker use `npm ci`. A manifest/lockfile mismatch must fail instead of silently regenerating a lockfile.

## Update policy

Dependabot checks Cargo, npm, and GitHub Actions weekly. Patch/minor updates are grouped by ecosystem; major updates remain separate for explicit review. Dependency PRs are not auto-merged.

## Security and license inventory

A parallel CI lane runs RustSec `cargo-audit` pinned to 0.23.0, `npm audit --audit-level=high`, and a machine-readable Cargo/npm license inventory. JSON audit output and the TSV license inventory are uploaded as a 30-day artifact.

Fix advisories or record narrowly justified exceptions; do not hide findings by raising severity thresholds. The product repository is currently marked `UNLICENSED`; the inventory is evidence for a future license decision, not an assertion of redistribution approval. Review unknown license metadata and incompatible licenses before selecting a project license.
