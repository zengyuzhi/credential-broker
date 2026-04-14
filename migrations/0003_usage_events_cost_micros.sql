-- 0003_usage_events_cost_micros.sql
--
-- SE-09: switch monetary accumulation from f64 to integer microdollars.
--
-- One microdollar = $0.000001. i64 microdollars covers ±$9.2 quadrillion, so
-- there is no realistic overflow risk. SQL SUMs accumulate INTEGER exactly,
-- whereas REAL (f64) drifts after enough additions.
--
-- Backfill converts pre-existing REAL values; the old column is dropped.
-- Requires SQLite >= 3.35 for `ALTER TABLE DROP COLUMN` (macOS 15 ships 3.43+).

ALTER TABLE usage_events ADD COLUMN estimated_cost_micros INTEGER;

UPDATE usage_events
   SET estimated_cost_micros = CAST(estimated_cost_usd * 1000000 AS INTEGER)
 WHERE estimated_cost_usd IS NOT NULL;

ALTER TABLE usage_events DROP COLUMN estimated_cost_usd;
