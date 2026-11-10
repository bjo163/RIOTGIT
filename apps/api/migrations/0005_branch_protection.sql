CREATE TABLE IF NOT EXISTS branch_protection_rules (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    branch_name TEXT NOT NULL CHECK (length(branch_name) BETWEEN 1 AND 200),
    require_pull_request INTEGER NOT NULL DEFAULT 1 CHECK (require_pull_request IN (0, 1)),
    prevent_force_push INTEGER NOT NULL DEFAULT 1 CHECK (prevent_force_push IN (0, 1)),
    prevent_deletion INTEGER NOT NULL DEFAULT 1 CHECK (prevent_deletion IN (0, 1)),
    created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (repository_id, branch_name)
);

CREATE INDEX IF NOT EXISTS idx_branch_protection_repository
    ON branch_protection_rules(repository_id, branch_name);
