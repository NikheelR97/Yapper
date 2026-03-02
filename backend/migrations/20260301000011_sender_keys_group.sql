-- Group E2EE: Sender Key distributions + msg_num on messages

-- Ensure msg_num column exists (channels service stores the ratchet iteration here)
ALTER TABLE messages ADD COLUMN IF NOT EXISTS msg_num INTEGER;

-- Sender Key distributions: one row per (channel, sender, recipient).
-- The ciphertext holds the sender's SenderKey material ECIES-encrypted for the recipient.
-- Server never sees the plaintext chain key or signing key.
CREATE TABLE sender_key_distributions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    from_user   UUID NOT NULL REFERENCES users(id),
    to_user     UUID NOT NULL REFERENCES users(id),
    -- ECIES payload: AES-256-GCM encrypted SenderKey material
    ciphertext  BYTEA NOT NULL,
    -- X25519 ephemeral public key used for ECIES DH
    ek_public   BYTEA NOT NULL,
    delivered   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Only one distribution per (channel, sender, recipient) at a time
    UNIQUE (channel_id, from_user, to_user)
);

-- Fast lookup for pending distributions on connect / key_dist fetch
CREATE INDEX idx_skd_pending ON sender_key_distributions (to_user, delivered)
    WHERE delivered = FALSE;
CREATE INDEX idx_skd_channel_from ON sender_key_distributions (channel_id, from_user);
