CREATE TABLE credentials (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    secret_ref TEXT NOT NULL,
    environment TEXT NOT NULL,
    owner TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_used_at TEXT
);

CREATE UNIQUE INDEX idx_credentials_provider_label
ON credentials(provider, label);

CREATE TABLE credential_fields (
    credential_id TEXT NOT NULL,
    field_name TEXT NOT NULL,
    value_ref TEXT NOT NULL,
    PRIMARY KEY (credential_id, field_name),
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

CREATE TABLE profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    default_project TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE profile_bindings (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    credential_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_profile_bindings_profile_provider
ON profile_bindings(profile_id, provider);

CREATE TABLE leases (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    project TEXT,
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    session_token_hash TEXT NOT NULL,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TABLE usage_events (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    credential_id TEXT NOT NULL,
    lease_id TEXT,
    agent_name TEXT NOT NULL,
    project TEXT,
    mode TEXT NOT NULL,
    operation TEXT NOT NULL,
    endpoint TEXT,
    model TEXT,
    request_count INTEGER NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    estimated_cost_usd REAL,
    status_code INTEGER,
    success INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    error_text TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE,
    FOREIGN KEY (lease_id) REFERENCES leases(id) ON DELETE SET NULL
);

CREATE INDEX idx_usage_events_provider_time
ON usage_events(provider, created_at);

CREATE INDEX idx_usage_events_credential_time
ON usage_events(credential_id, created_at);
