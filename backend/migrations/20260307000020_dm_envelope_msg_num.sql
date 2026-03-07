ALTER TABLE dm_message_envelopes
    ADD COLUMN IF NOT EXISTS msg_num INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_dm_envelopes_message_device_order
    ON dm_message_envelopes (recipient_device_id, created_at DESC, msg_num DESC);
