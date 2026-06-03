-- Reversible account deactivation.
--
-- Distinct from `deleted_at` (the GDPR soft-delete + retention hold): a disabled
-- account keeps all of its data and is reactivated automatically on the next
-- successful login (see auth::v2::login). Disabling revokes all sessions so the
-- user is signed out on every device.
ALTER TABLE users ADD COLUMN IF NOT EXISTS disabled_at TIMESTAMPTZ;
