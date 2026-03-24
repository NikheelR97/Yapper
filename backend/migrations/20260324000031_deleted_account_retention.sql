ALTER TABLE users
    ADD COLUMN IF NOT EXISTS deletion_hold_until TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deletion_retention_basis TEXT
        CHECK (
            deletion_retention_basis IS NULL
            OR deletion_retention_basis IN (
                'pending_purge_review',
                'retained_referential_integrity'
            )
        );

UPDATE users
SET deletion_hold_until = COALESCE(deletion_hold_until, deleted_at + INTERVAL '30 days'),
    deletion_retention_basis = COALESCE(deletion_retention_basis, 'pending_purge_review')
WHERE deleted_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_users_deleted_hold_until
    ON users (deletion_hold_until)
    WHERE deleted_at IS NOT NULL AND deletion_hold_until IS NOT NULL;
