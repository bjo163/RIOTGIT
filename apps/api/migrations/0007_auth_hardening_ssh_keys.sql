CREATE TABLE IF NOT EXISTS login_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    username_digest TEXT NOT NULL,
    succeeded INTEGER NOT NULL DEFAULT 0 CHECK (succeeded IN (0, 1)),
    attempted_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_login_attempts_window
    ON login_attempts(username_digest, attempted_at DESC, succeeded);

CREATE TABLE IF NOT EXISTS ssh_keys (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_type TEXT NOT NULL,
    key_blob TEXT NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    comment TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_ssh_keys_user_created
    ON ssh_keys(user_id, created_at DESC);
