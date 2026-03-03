-- User settings: username change cooldown tracking + privacy preferences

-- Add username change cooldown tracking to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS username_changed_at TIMESTAMPTZ;

-- Privacy settings per user
-- dm_permission: who can DM the user  — 'everyone' | 'friends' | 'nobody'
-- friend_request_permission: who can send friend requests — 'everyone' | 'friends_of_friends' | 'nobody'
-- search_visible: whether user appears in search results
-- show_last_seen: whether last_seen_at is visible to others
CREATE TABLE user_privacy_settings (
    user_id                     UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    dm_permission               TEXT NOT NULL DEFAULT 'everyone'
                                    CHECK (dm_permission IN ('everyone', 'friends', 'nobody')),
    friend_request_permission   TEXT NOT NULL DEFAULT 'everyone'
                                    CHECK (friend_request_permission IN ('everyone', 'friends_of_friends', 'nobody')),
    search_visible              BOOLEAN NOT NULL DEFAULT TRUE,
    show_last_seen              BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
