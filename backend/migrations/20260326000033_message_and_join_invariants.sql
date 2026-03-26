-- Enforce DM pair uniqueness, parental pending deduplication, and join-time invite invariants.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM (
            SELECT conversation_id
            FROM dm_participants
            GROUP BY conversation_id
            HAVING COUNT(*) <> 2
        ) invalid_conversations
    ) THEN
        RAISE EXCEPTION
            'Cannot backfill dm_conversation_pairs: existing conversations must each have exactly two participants';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM (
            SELECT MIN(user_id) AS user_low, MAX(user_id) AS user_high, COUNT(*) AS row_count
            FROM dm_participants
            GROUP BY conversation_id
        ) conversation_pairs
        GROUP BY user_low, user_high
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'Cannot backfill dm_conversation_pairs: duplicate conversations already exist for the same participant pair';
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS dm_conversation_pairs (
    conversation_id UUID PRIMARY KEY REFERENCES dm_conversations(id) ON DELETE CASCADE,
    user_low UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_high UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT dm_conversation_pairs_ordered CHECK (user_low < user_high),
    UNIQUE (user_low, user_high)
);

INSERT INTO dm_conversation_pairs (conversation_id, user_low, user_high)
SELECT conversation_id, MIN(user_id) AS user_low, MAX(user_id) AS user_high
FROM dm_participants
GROUP BY conversation_id
ON CONFLICT (conversation_id) DO NOTHING;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM (
            SELECT child_user_id, requester_id, COUNT(*) AS row_count
            FROM pending_friend_requests
            WHERE status = 'pending'
            GROUP BY child_user_id, requester_id
            HAVING COUNT(*) > 1
        ) duplicate_friend_requests
    ) THEN
        RAISE EXCEPTION
            'Cannot add pending friend-request dedup index: duplicate pending rows already exist';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM (
            SELECT child_user_id, server_id, COUNT(*) AS row_count
            FROM pending_server_joins
            WHERE status = 'pending'
            GROUP BY child_user_id, server_id
            HAVING COUNT(*) > 1
        ) duplicate_server_joins
    ) THEN
        RAISE EXCEPTION
            'Cannot add pending server-join dedup index: duplicate pending rows already exist';
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_pending_friend_requests_unique_pending
    ON pending_friend_requests (child_user_id, requester_id)
    WHERE status = 'pending';

CREATE UNIQUE INDEX IF NOT EXISTS idx_pending_server_joins_unique_pending
    ON pending_server_joins (child_user_id, server_id)
    WHERE status = 'pending';
