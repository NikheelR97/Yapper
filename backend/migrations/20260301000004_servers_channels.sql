-- Servers, channels, memberships, invite links

CREATE TABLE servers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    slug            TEXT UNIQUE NOT NULL,
    owner_id        UUID NOT NULL REFERENCES users(id),
    icon_url        TEXT,
    banner_url      TEXT,
    description     TEXT,
    is_public       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_servers_public ON servers (is_public, created_at DESC)
    WHERE is_public = TRUE;

-- Full-text search on server name + description
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX idx_servers_search ON servers
    USING gin ((name || ' ' || COALESCE(description, '')) gin_trgm_ops);

CREATE TABLE server_memberships (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member'
                        CHECK (role IN ('owner', 'admin', 'member')),
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX idx_server_memberships_user ON server_memberships (user_id);
CREATE INDEX idx_server_memberships_server ON server_memberships (server_id);

CREATE TABLE channels (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    type            TEXT NOT NULL DEFAULT 'text'
                        CHECK (type IN ('text', 'voice', 'announcement')),
    position        INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (server_id, name)
);

CREATE INDEX idx_channels_server ON channels (server_id, position);

CREATE TABLE server_invite_links (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    code            TEXT UNIQUE NOT NULL,
    creator_id      UUID NOT NULL REFERENCES users(id),
    uses            INTEGER NOT NULL DEFAULT 0,
    max_uses        INTEGER,  -- NULL = unlimited
    expires_at      TIMESTAMPTZ,  -- NULL = never
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invite_links_code ON server_invite_links (code);
