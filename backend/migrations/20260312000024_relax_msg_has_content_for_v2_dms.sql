-- v2 DM messages store per-device ciphertext in dm_message_envelopes, not in messages.ciphertext.
-- Relax the msg_has_content constraint so that DM conversation messages can have NULL ciphertext
-- and NULL plaintext (the content is enforced at the API level and in dm_message_envelopes).

ALTER TABLE messages
    DROP CONSTRAINT IF EXISTS msg_has_content;

ALTER TABLE messages
    ADD CONSTRAINT msg_has_content CHECK (
        -- Channel messages and v1 DM messages must have ciphertext or plaintext directly.
        -- v2 DM messages (conversation_id IS NOT NULL) store ciphertext in dm_message_envelopes.
        ciphertext IS NOT NULL
        OR plaintext IS NOT NULL
        OR (conversation_id IS NOT NULL AND channel_id IS NULL)
    );
