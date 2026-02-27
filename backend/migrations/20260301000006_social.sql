-- Social graph: friends + followers

-- Friendships — user_id_1 < user_id_2 invariant enforced by CHECK
-- to prevent duplicate rows for the same pair in opposite order
CREATE TABLE friendships (
    user_id_1   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_id_2   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status      TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'accepted', 'blocked')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id_1, user_id_2),
    CONSTRAINT friendship_order CHECK (user_id_1 < user_id_2)
);

CREATE INDEX idx_friendships_user1 ON friendships (user_id_1);
CREATE INDEX idx_friendships_user2 ON friendships (user_id_2);

-- Followers (separate from friends — public follow without mutual accept)
CREATE TABLE followers (
    follower_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (follower_id, following_id)
);

CREATE INDEX idx_followers_following ON followers (following_id);
CREATE INDEX idx_followers_follower ON followers (follower_id);

-- Hype Moments — pinned messages on user profiles
CREATE TABLE hype_moments (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_id  UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    type        TEXT NOT NULL CHECK (type IN ('yap', 'clip', 'text')),
    pinned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
