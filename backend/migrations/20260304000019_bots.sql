-- Bot applications and API tokens (S8)

CREATE TABLE IF NOT EXISTS bot_applications (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    description       TEXT,
    avatar_url        TEXT,
    discord_bot_id    TEXT UNIQUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS bot_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_app_id  UUID NOT NULL REFERENCES bot_applications(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,  -- SHA-256 of raw token; raw token shown once and never stored
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bot_applications_developer ON bot_applications (developer_user_id);
CREATE INDEX IF NOT EXISTS idx_bot_tokens_app ON bot_tokens (bot_app_id);
