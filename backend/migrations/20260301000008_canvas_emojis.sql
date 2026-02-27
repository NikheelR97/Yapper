-- Live Canvas: music state + polls
-- Custom server emojis

CREATE TABLE canvas_music_state (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL UNIQUE REFERENCES servers(id) ON DELETE CASCADE,
    artist          TEXT NOT NULL,
    title           TEXT NOT NULL,
    source_url      TEXT,
    album_art_url   TEXT,
    set_by_user_id  UUID NOT NULL REFERENCES users(id),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE polls (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    creator_id  UUID NOT NULL REFERENCES users(id),
    question    TEXT NOT NULL,
    options     JSONB NOT NULL,  -- [{ "text": "...", "index": 0 }, ...]
    ends_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE poll_votes (
    poll_id         UUID NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_index    INTEGER NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (poll_id, user_id)
);

-- Custom emojis per server (limit: 50 free / 100 premium)
CREATE TABLE server_emojis (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,  -- shortcode name (alphanumeric + underscores)
    image_url       TEXT NOT NULL,  -- public R2 URL
    image_r2_key    TEXT NOT NULL,  -- R2 object key for deletion
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (server_id, name)
);

-- Bot applications
CREATE TABLE bot_applications (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    description         TEXT,
    avatar_url          TEXT,
    discord_bot_id      TEXT,  -- original Discord bot ID if imported
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE bot_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_app_id      UUID NOT NULL REFERENCES bot_applications(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,  -- SHA-256 hash; raw token shown once at creation
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
