-- Migration: tighten message content invariants without breaking v2 DM parents.
--
-- Direct-content rows must store exactly one payload:
--   * ciphertext only for E2EE messages
--   * plaintext only for bot/plaintext messages
--
-- Multi-device v2 DM parent rows intentionally store no direct payload in
-- `messages`; their ciphertext lives in `dm_message_envelopes`, so those
-- conversation-only rows remain exempt.

DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM messages
    WHERE (ciphertext IS NOT NULL AND plaintext IS NOT NULL)
       OR (
         ciphertext IS NULL
         AND plaintext IS NULL
         AND NOT (conversation_id IS NOT NULL AND channel_id IS NULL)
       )
  ) THEN
    RAISE EXCEPTION 'Pre-migration data violation: messages table contains rows with invalid ciphertext/plaintext combinations for direct-content rows.';
  END IF;
END $$;

ALTER TABLE messages DROP CONSTRAINT IF EXISTS msg_has_content;
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_content_check;
ALTER TABLE messages DROP CONSTRAINT IF EXISTS check_message_content;

ALTER TABLE messages
  ADD CONSTRAINT messages_ciphertext_xor_plaintext
  CHECK (
    (ciphertext IS NOT NULL AND plaintext IS NULL)
    OR (ciphertext IS NULL AND plaintext IS NOT NULL)
    OR (conversation_id IS NOT NULL AND channel_id IS NULL AND ciphertext IS NULL AND plaintext IS NULL)
  );

COMMENT ON CONSTRAINT messages_ciphertext_xor_plaintext ON messages IS
  'Direct-content messages must store exactly one of ciphertext/plaintext; v2 DM parent rows remain envelope-backed and store neither.';
