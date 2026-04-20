-- Phase 1: Broker core domain model (connectors, capabilities, grants, bundles, sessions).
-- All existing tables remain untouched.

CREATE TABLE connectors (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL,
    credential_id TEXT NOT NULL,
    base_url TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

CREATE TABLE capabilities (
    id TEXT PRIMARY KEY NOT NULL,
    connector_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
    created_at TEXT NOT NULL,
    FOREIGN KEY (connector_id) REFERENCES connectors(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_capabilities_connector_name
ON capabilities(connector_id, name);

CREATE TABLE grants (
    id TEXT PRIMARY KEY NOT NULL,
    agent_name TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    ttl_minutes INTEGER,
    max_requests INTEGER,
    require_confirmation INTEGER NOT NULL DEFAULT 0 CHECK(require_confirmation IN (0,1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
    created_at TEXT NOT NULL,
    expires_at TEXT,
    FOREIGN KEY (capability_id) REFERENCES capabilities(id) ON DELETE CASCADE
);

CREATE INDEX idx_grants_agent ON grants(agent_name);

CREATE TABLE bundles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    source_profile_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_profile_id) REFERENCES profiles(id) ON DELETE SET NULL
);

CREATE TABLE bundle_grants (
    bundle_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    PRIMARY KEY (bundle_id, grant_id),
    FOREIGN KEY (bundle_id) REFERENCES bundles(id) ON DELETE CASCADE,
    FOREIGN KEY (grant_id) REFERENCES grants(id) ON DELETE CASCADE
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY NOT NULL,
    bundle_id TEXT,
    agent_name TEXT NOT NULL,
    project TEXT,
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    session_token_hash TEXT NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (bundle_id) REFERENCES bundles(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX idx_sessions_token_hash ON sessions(session_token_hash);

CREATE TABLE session_grants (
    session_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    PRIMARY KEY (session_id, grant_id),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (grant_id) REFERENCES grants(id) ON DELETE CASCADE
);
