-- Normalize user email storage and move linked-provider identity data out of the overloaded users.discord_id column.

-- Email normalization:
-- 1. Lower-case existing emails so the table is in a canonical state.
-- 2. Add a CHECK constraint so future writes must already be lower-case.
-- 3. Keep the existing UNIQUE(email) constraint; with canonical storage it becomes case-insensitive in practice.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM (
            SELECT lower(email) AS normalized_email, COUNT(*) AS row_count
            FROM users
            GROUP BY lower(email)
            HAVING COUNT(*) > 1
        ) duplicate_emails
    ) THEN
        RAISE EXCEPTION
            'Cannot normalize users.email: case-insensitive duplicates already exist';
    END IF;
END $$;

UPDATE users
SET email = lower(email)
WHERE email <> lower(email);

ALTER TABLE users
    ADD CONSTRAINT users_email_lowercase CHECK (email = lower(email));

-- Normalized linked identities for OAuth / import flows.
CREATE TABLE user_linked_identities (
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider         TEXT NOT NULL CHECK (provider IN ('discord', 'google', 'apple')),
    provider_subject TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, provider),
    UNIQUE (provider, provider_subject)
);

CREATE INDEX idx_user_linked_identities_user_id ON user_linked_identities (user_id);

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM (
            SELECT
                CASE
                    WHEN u.discord_id LIKE 'google:%' THEN 'google'
                    WHEN u.discord_id LIKE 'apple:%' THEN 'apple'
                    WHEN u.discord_id LIKE 'discord:%' THEN 'discord'
                    ELSE 'discord'
                END AS provider,
                CASE
                    WHEN u.discord_id LIKE '%:%' THEN split_part(u.discord_id, ':', 2)
                    ELSE u.discord_id
                END AS provider_subject,
                COUNT(*) AS row_count
            FROM users u
            WHERE u.discord_id IS NOT NULL
              AND u.deleted_at IS NULL
            GROUP BY 1, 2
            HAVING COUNT(*) > 1
        ) duplicate_identities
    ) THEN
        RAISE EXCEPTION
            'Cannot normalize users.discord_id: duplicate linked identities already exist';
    END IF;
END $$;

-- Backfill from the legacy overloaded users.discord_id column.
INSERT INTO user_linked_identities (user_id, provider, provider_subject, created_at)
SELECT
    u.id,
    CASE
        WHEN u.discord_id LIKE 'google:%' THEN 'google'
        WHEN u.discord_id LIKE 'apple:%' THEN 'apple'
        WHEN u.discord_id LIKE 'discord:%' THEN 'discord'
        ELSE 'discord'
    END AS provider,
    CASE
        WHEN u.discord_id LIKE '%:%' THEN split_part(u.discord_id, ':', 2)
        ELSE u.discord_id
    END AS provider_subject,
    u.created_at
FROM users u
WHERE u.discord_id IS NOT NULL
  AND u.deleted_at IS NULL;
