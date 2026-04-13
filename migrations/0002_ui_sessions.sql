CREATE TABLE ui_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL UNIQUE,
    pin_hash TEXT NOT NULL,
    session_token_hash TEXT,
    csrf_token TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_ui_sessions_challenge_id
ON ui_sessions(challenge_id);

CREATE INDEX idx_ui_sessions_session_token_hash
ON ui_sessions(session_token_hash);
