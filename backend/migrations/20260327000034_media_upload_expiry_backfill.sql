-- Backfill expires_at for existing media_uploads rows.
-- Rows older than 7 days become immediately eligible for next retention cycle.
UPDATE media_uploads
SET expires_at = created_at + INTERVAL '7 days'
WHERE expires_at IS NULL;
