-- Phase 1.1: link usage events to the broker session/bundle that authorized them.
-- Nullable because existing lease-path events never had this attribution, and
-- future lease requests will continue to leave these columns NULL.

ALTER TABLE usage_events ADD COLUMN session_id TEXT;
ALTER TABLE usage_events ADD COLUMN bundle_id TEXT;
