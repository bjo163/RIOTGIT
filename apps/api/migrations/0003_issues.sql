CREATE TABLE IF NOT EXISTS issues (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    number INTEGER NOT NULL CHECK (number > 0),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    body TEXT,
    state TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'closed')),
    author_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    closed_at TEXT,
    UNIQUE (repository_id, number)
);

CREATE INDEX IF NOT EXISTS idx_issues_repository_state_number
    ON issues(repository_id, state, number DESC);

CREATE INDEX IF NOT EXISTS idx_issues_author
    ON issues(author_id, created_at DESC);

CREATE TABLE IF NOT EXISTS issue_comments (
    id TEXT PRIMARY KEY NOT NULL,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES users(id),
    body TEXT NOT NULL CHECK (length(body) BETWEEN 1 AND 10000),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_issue_comments_issue_created
    ON issue_comments(issue_id, created_at ASC);
