# R!OTGIT — Product Narrative

## One-line positioning

**R!OTGIT is the underground code network: a sovereign, self-hosted Git forge that keeps normal Git workflows intact while making collaboration, automation, and distributed development feel like one coherent product.**

## Product promise

Own the repository. Own the workflow. Choose where collaboration happens.

R!OTGIT gives an individual developer, small team, or infrastructure operator a compact code home that can grow from a single machine into a multi-node network without forcing a rewrite of repositories or developer habits.

## Who it serves

1. **Independent builders** who want a private Git home without heavyweight platform operations.
2. **Small engineering teams** that need repositories, reviews, issues, releases, and automation in one place.
3. **Self-hosting operators** who need portable storage, backups, clear permissions, and predictable deployments.
4. **Distributed communities** that want mirrors and eventually peer-to-peer collaboration without surrendering local ownership.
5. **AI-assisted teams** that want controlled, opt-in agents operating through auditable repository permissions.

## What it is — and is not

R!OTGIT is:
- a Git-compatible forge and collaboration layer;
- local-first and self-hostable;
- modular, with a lean default installation;
- designed for ordinary Git clients as well as web and terminal users;
- extensible through explicit transport, runner, federation, and AI adapters.

R!OTGIT is not:
- a replacement for Git's object model or wire protocols;
- a promise of "zero server" operation in the first release;
- a general-purpose untrusted-code execution environment;
- a drop-in implementation of every GitHub Actions feature;
- a P2P network until the security, identity, discovery, and replication model is verified.

## Product pillars

### 1. Git without friction
Clone, fetch, push, branch, tags, and normal Git tooling must work. The platform must never make repository data dependent on a proprietary database representation.

### 2. Forge with attitude
A coherent code browser, commit history, diffs, pull requests, reviews, issues, releases, and discussions. Extreme visuals must improve hierarchy, not obscure code.

### 3. Small by default
One API process, one web app, one embedded database, and local repository storage should be enough for a single-node evaluation. Optional capabilities must be separately deployable.

### 4. Network when you need it
Support private mirrors and backup replication first. Add federation and P2P only through versioned adapters, with clear trust and visibility controls.

### 5. Automation with guardrails
Webhooks and jobs are first-class. CI execution is isolated from the API and repository storage, with explicit limits and no privileged host access by default.

### 6. AI under owner control
AI providers are pluggable and opt-in. Repository context is scoped, secrets are redacted, access is audited, and no private source is transmitted without explicit policy.

## Product experience map

| Experience | Primary task | Success signal |
| --- | --- | --- |
| Home / dashboard | Understand activity and jump into work | Useful first screen, fast navigation |
| Repository | Read code and understand project state | Reliable browser, graph, files, commits |
| Pull request | Review and merge changes safely | Clear diff, checks, approvals, audit trail |
| Issues | Track work and decisions | Simple lifecycle, labels, milestones |
| Terminal | Operate without the browser | Scriptable CLI and SSH workflows |
| Admin | Operate a sovereign forge | Transparent health, storage, backup, policy |
| Network | Replicate and collaborate across nodes | Explicit trust, verified sync, recoverability |

## Experience principles

- **Show state, not decoration:** branch, permissions, health, CI, and sync state are visible.
- **Progressive power:** simple workflows first; advanced network and automation controls remain discoverable.
- **Keyboard-respectful:** command palette, focus states, shortcuts, and accessible forms.
- **Code is the hero:** typography, diff colors, syntax highlighting, and spacing favor readability.
- **Portable by design:** users can inspect and back up bare repositories without R!OTGIT-specific tooling.
- **Honest capability labels:** experimental features are visibly marked; no "P2P" badge for a plain mirror.

## MVP boundary

The first usable release is a single-node forge:
- local account bootstrap and sessions;
- organization/team/repository metadata;
- bare Git repository creation and safe deletion;
- authenticated Git over SSH and smart HTTP;
- code browser, commit list, and diff view;
- issues and pull requests in the web UI;
- backups and restore validation.

Not required for the first usable release:
- P2P or federation;
- Actions compatibility;
- AI agent execution;
- package registry;
- multi-region deployment.

## Non-functional product targets

Targets must be measured rather than assumed:
- low idle memory on a small single-node install;
- fast repository listing and navigation for typical small-team repositories;
- no repository corruption after interrupted pushes or process restarts;
- deterministic backup and restore;
- explicit limits for upload size, archive generation, diff rendering, and background jobs;
- safe degradation when optional integrations are offline.

## Definition of product readiness

A feature is ready only when:
1. user flow and permission model are documented;
2. happy path and failure states are implemented;
3. tests cover authorization boundaries and malformed inputs;
4. audit/logging behavior is defined;
5. docs explain configuration and recovery;
6. the feature is verified against the supported deployment profile.
