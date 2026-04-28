-- Phase 1.1 policy enforcement: per-grant request counter for max_requests quota.
-- sessions.request_count tracks session-wide volume, but Grant.max_requests is
-- a per-grant cap — a session carrying multiple grants needs independent
-- counters to enforce each grant's quota.

ALTER TABLE session_grants ADD COLUMN request_count INTEGER NOT NULL DEFAULT 0;
