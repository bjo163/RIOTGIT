CREATE TABLE IF NOT EXISTS pull_requests (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    number INTEGER NOT NULL CHECK (number > 0),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    body TEXT,
    state TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'closed', 'merging', 'merged')),
    head_ref TEXT NOT NULL CHECK (length(head_ref) BETWEEN 1 AND 200),
    base_ref TEXT NOT NULL CHECK (length(base_ref) BETWEEN 1 AND 200),
    head_sha TEXT NOT NULL CHECK (length(head_sha) BETWEEN 40 AND 64),
    base_sha TEXT NOT NULL CHECK (length(base_sha) BETWEEN 40 AND 64),
    author_id TEXT NOT NULL REFERENCES users(id),
    merged_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    merge_commit TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    closed_at TEXT,
    merged_at TEXT,
    UNIQUE(repository_id, number)
);

CREATE INDEX IF NOT EXISTS idx_pull_requests_repository_state_number
    ON pull_requests(repository_id, state, number DESC);

CREATE INDEX IF NOT EXISTS idx_pull_requests_author
    ON pull_requests(author_id, created_at DESC);

CREATE TABLE IF NOT EXISTS pull_request_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    pull_request_id TEXT NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    reviewer_id TEXT NOT NULL REFERENCES users(id),
    state TEXT NOT NULL CHECK (state IN ('comment', 'approved', 'changes_requested')),
    body TEXT,
    submitted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_pull_request_reviews_history
    ON pull_request_reviews(pull_request_id, reviewer_id, submitted_at DESC);
