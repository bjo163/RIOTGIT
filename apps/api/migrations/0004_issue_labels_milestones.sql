CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 50),
    color TEXT NOT NULL CHECK (length(color) = 6 AND color NOT GLOB '*[^0-9A-Fa-f]*'),
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(repository_id, name)
);

CREATE INDEX IF NOT EXISTS idx_labels_repository_name
    ON labels(repository_id, name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS issue_labels (
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    added_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY(issue_id, label_id)
);

CREATE TABLE IF NOT EXISTS milestones (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    number INTEGER NOT NULL CHECK (number > 0),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 100),
    description TEXT,
    state TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'closed')),
    due_on TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(repository_id, number)
);

CREATE INDEX IF NOT EXISTS idx_milestones_repository_state_number
    ON milestones(repository_id, state, number DESC);

ALTER TABLE issues
    ADD COLUMN milestone_id TEXT REFERENCES milestones(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_issues_milestone
    ON issues(milestone_id);
