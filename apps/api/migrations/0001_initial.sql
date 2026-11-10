PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY NOT NULL,
    owner TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    visibility TEXT NOT NULL CHECK (visibility IN ('public', 'private', 'internal')),
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'archived', 'failed')),
    storage_key TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (owner, name)
);

CREATE INDEX IF NOT EXISTS idx_repositories_owner_created
    ON repositories(owner, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_repositories_state
    ON repositories(state);
