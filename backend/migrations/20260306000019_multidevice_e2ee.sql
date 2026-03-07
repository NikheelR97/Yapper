-- Multi-device E2EE foundation.
-- Preserves existing v1 integer Signal device ids while introducing
-- durable UUID-based device records for auth, trust, and sync flows.

CREATE TABLE IF NOT EXISTS devices (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    signal_device_id  INTEGER NOT NULL,
    installation_id   UUID,
    platform          TEXT NOT NULL,
    label             TEXT NOT NULL,
    trust_state       TEXT NOT NULL DEFAULT 'pending_trust'
                        CHECK (trust_state IN ('trusted', 'pending_trust', 'revoked')),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at      TIMESTAMPTZ,
    approved_at       TIMESTAMPTZ,
    approved_by       UUID REFERENCES devices(id) ON DELETE SET NULL,
    revoked_at        TIMESTAMPTZ,
    UNIQUE (user_id, signal_device_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_user_installation
    ON devices (user_id, installation_id)
    WHERE installation_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_devices_user_state
    ON devices (user_id, trust_state)
    WHERE revoked_at IS NULL;

ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS device_id UUID REFERENCES devices(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_sessions_device_id ON sessions (device_id);

ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS sender_device_id UUID REFERENCES devices(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_messages_sender_device_id ON messages (sender_device_id)
    WHERE sender_device_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS dm_message_envelopes (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id           UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    recipient_user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_device_id  UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    sender_user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sender_device_id     UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    ciphertext           BYTEA NOT NULL,
    ek_public            BYTEA,
    opk_id               INTEGER,
    delivered_at         TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (message_id, recipient_device_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_envelopes_recipient_device
    ON dm_message_envelopes (recipient_device_id, delivered_at, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_dm_envelopes_message_id
    ON dm_message_envelopes (message_id);

ALTER TABLE sender_key_distributions
    ADD COLUMN IF NOT EXISTS from_device_id UUID REFERENCES devices(id) ON DELETE CASCADE;

ALTER TABLE sender_key_distributions
    ADD COLUMN IF NOT EXISTS to_device_id UUID REFERENCES devices(id) ON DELETE CASCADE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'sender_key_distributions_channel_id_from_user_to_user_key'
    ) THEN
        ALTER TABLE sender_key_distributions
            DROP CONSTRAINT sender_key_distributions_channel_id_from_user_to_user_key;
    END IF;
END$$;

ALTER TABLE sender_key_distributions
    ADD CONSTRAINT sender_key_distributions_channel_from_to_device_key
    UNIQUE (channel_id, from_user, from_device_id, to_user, to_device_id);

CREATE INDEX IF NOT EXISTS idx_skd_pending_device
    ON sender_key_distributions (to_device_id, delivered)
    WHERE delivered = FALSE AND to_device_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS device_trust_requests (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_device_id   UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at        TIMESTAMPTZ,
    approved_by        UUID REFERENCES devices(id) ON DELETE SET NULL,
    UNIQUE (target_device_id)
);

CREATE INDEX IF NOT EXISTS idx_device_trust_requests_user
    ON device_trust_requests (user_id, approved_at);

CREATE TABLE IF NOT EXISTS device_sync_events (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_device_id   UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    source_device_id   UUID REFERENCES devices(id) ON DELETE CASCADE,
    event_type         TEXT NOT NULL
                        CHECK (event_type IN (
                            'device_trust_request',
                            'device_trust_approved',
                            'device_sync_chunk',
                            'device_sync_complete',
                            'device_revoked'
                        )),
    payload            JSONB NOT NULL DEFAULT '{}'::jsonb,
    delivered_at       TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_device_sync_events_target
    ON device_sync_events (target_device_id, delivered_at, created_at);

CREATE TABLE IF NOT EXISTS device_key_backups (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id          UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    source_device_id   UUID REFERENCES devices(id) ON DELETE SET NULL,
    encrypted_blob     BYTEA NOT NULL,
    version            INTEGER NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at         TIMESTAMPTZ,
    UNIQUE (device_id, version)
);

CREATE INDEX IF NOT EXISTS idx_device_key_backups_user_device
    ON device_key_backups (user_id, device_id, revoked_at);
