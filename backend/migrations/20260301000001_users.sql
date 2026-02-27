-- Users table — core identity store
CREATE TABLE users (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email                       TEXT UNIQUE NOT NULL,
    email_verified              BOOLEAN NOT NULL DEFAULT FALSE,
    email_verify_token          TEXT,
    email_verify_expires_at     TIMESTAMPTZ,
    username                    TEXT UNIQUE NOT NULL,
    display_name                TEXT NOT NULL,
    avatar_url                  TEXT,
    banner_url                  TEXT,
    password_hash               TEXT,  -- NULL for OAuth-only accounts
    account_type                TEXT NOT NULL DEFAULT 'standard'
                                    CHECK (account_type IN ('standard', 'parent', 'child', 'bot')),
    parental_controls_enabled   BOOLEAN NOT NULL DEFAULT FALSE,
    date_of_birth               DATE,
    coppa_consent_verified_at   TIMESTAMPTZ,
    gdpr_consent_at             TIMESTAMPTZ,
    profile_theme_color         TEXT DEFAULT '#7C3AED',  -- violet default
    about_me                    TEXT CHECK (char_length(about_me) <= 500),
    location                    TEXT,
    is_premium                  BOOLEAN NOT NULL DEFAULT FALSE,
    discord_id                  TEXT UNIQUE,
    last_seen_at                TIMESTAMPTZ,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at                  TIMESTAMPTZ  -- soft delete
);

CREATE INDEX idx_users_username ON users (username);
CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_users_discord_id ON users (discord_id) WHERE discord_id IS NOT NULL;
CREATE INDEX idx_users_deleted_at ON users (deleted_at) WHERE deleted_at IS NOT NULL;
