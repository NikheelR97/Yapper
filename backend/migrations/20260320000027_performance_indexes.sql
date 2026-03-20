-- Performance optimizations: denormalized counters, materialized timestamps, partial indexes.
-- Eliminates correlated subqueries on hot paths (list_my_servers, list_conversations, live_servers).

-- ─── 1. Denormalize member_count on servers ──────────────────────────────────

ALTER TABLE servers ADD COLUMN IF NOT EXISTS member_count BIGINT NOT NULL DEFAULT 0;

-- Backfill from current data
UPDATE servers s
SET member_count = (
    SELECT COUNT(*) FROM server_memberships sm WHERE sm.server_id = s.id
);

-- Trigger: increment on INSERT
CREATE OR REPLACE FUNCTION trg_server_member_count_insert() RETURNS TRIGGER AS $$
BEGIN
    UPDATE servers SET member_count = member_count + 1 WHERE id = NEW.server_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER server_member_count_insert
    AFTER INSERT ON server_memberships
    FOR EACH ROW EXECUTE FUNCTION trg_server_member_count_insert();

-- Trigger: decrement on DELETE
CREATE OR REPLACE FUNCTION trg_server_member_count_delete() RETURNS TRIGGER AS $$
BEGIN
    UPDATE servers SET member_count = GREATEST(member_count - 1, 0) WHERE id = OLD.server_id;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER server_member_count_delete
    AFTER DELETE ON server_memberships
    FOR EACH ROW EXECUTE FUNCTION trg_server_member_count_delete();

-- ─── 2. Denormalize last_message_at on dm_conversations ──────────────────────

ALTER TABLE dm_conversations ADD COLUMN IF NOT EXISTS last_message_at TIMESTAMPTZ;

-- Backfill from current data
UPDATE dm_conversations dc
SET last_message_at = (
    SELECT MAX(m.created_at) FROM messages m WHERE m.conversation_id = dc.id
);

-- Index for sorting conversations by recency
CREATE INDEX IF NOT EXISTS idx_dm_conversations_last_message
    ON dm_conversations (last_message_at DESC NULLS LAST);

-- Trigger: update last_message_at on new message insert
CREATE OR REPLACE FUNCTION trg_dm_last_message_at() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.conversation_id IS NOT NULL THEN
        UPDATE dm_conversations
        SET last_message_at = GREATEST(last_message_at, NEW.created_at)
        WHERE id = NEW.conversation_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER dm_last_message_at_insert
    AFTER INSERT ON messages
    FOR EACH ROW EXECUTE FUNCTION trg_dm_last_message_at();

-- ─── 3. Partial index for undeleted messages (live_servers query) ─────────────

CREATE INDEX IF NOT EXISTS idx_messages_channel_created_not_deleted
    ON messages (channel_id, created_at DESC)
    WHERE deleted_at IS NULL AND channel_id IS NOT NULL;

-- ─── 4. Partial index for undelivered DM envelopes ───────────────────────────

CREATE INDEX IF NOT EXISTS idx_dm_envelopes_undelivered
    ON dm_message_envelopes (recipient_device_id, created_at DESC)
    WHERE delivered_at IS NULL;
