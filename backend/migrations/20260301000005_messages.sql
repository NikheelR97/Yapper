-- E2EE messages (ciphertext only — server never holds plaintext for E2EE messages)

CREATE TABLE dm_conversations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE dm_participants (
    conversation_id UUID NOT NULL REFERENCES dm_conversations(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX idx_dm_participants_user ON dm_participants (user_id);

CREATE TABLE messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Exactly one of channel_id or conversation_id must be set
    channel_id      UUID REFERENCES channels(id) ON DELETE CASCADE,
    conversation_id UUID REFERENCES dm_conversations(id) ON DELETE CASCADE,
    sender_id       UUID NOT NULL REFERENCES users(id),
    -- E2EE: ciphertext is the Signal-encrypted payload (includes media keys embedded within)
    -- Bot messages: ciphertext is NULL, plaintext is set instead (bots are NOT E2EE)
    ciphertext      BYTEA,
    plaintext       TEXT,
    -- X3DH ephemeral key — present only on the first message of a new session
    ek_public       BYTEA,
    opk_id          INTEGER,  -- OPK used for this X3DH exchange (so recipient can identify it)
    message_type    TEXT NOT NULL DEFAULT 'text'
                        CHECK (message_type IN ('text', 'yap', 'clip', 'system')),
    delivered       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ,  -- soft delete

    -- Enforce E2EE: either ciphertext or plaintext must be present
    CONSTRAINT msg_has_content CHECK (ciphertext IS NOT NULL OR plaintext IS NOT NULL),
    -- Must belong to exactly one of: channel or DM conversation
    CONSTRAINT msg_single_target CHECK (
        (channel_id IS NOT NULL AND conversation_id IS NULL) OR
        (channel_id IS NULL AND conversation_id IS NOT NULL)
    )
);

-- Critical indexes for message pagination (every chat view hits these)
CREATE INDEX idx_messages_channel_created ON messages (channel_id, created_at DESC)
    WHERE channel_id IS NOT NULL;
CREATE INDEX idx_messages_conversation_created ON messages (conversation_id, created_at DESC)
    WHERE conversation_id IS NOT NULL;
CREATE INDEX idx_messages_undelivered ON messages (sender_id, delivered)
    WHERE delivered = FALSE;

CREATE TABLE message_read_receipts (
    message_id  UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    read_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (message_id, user_id)
);

CREATE INDEX idx_read_receipts_message ON message_read_receipts (message_id);
