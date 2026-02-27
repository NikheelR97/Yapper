-- Sessions & refresh token family tracking
-- JWT refresh token reuse detection: if a revoked token from a family is replayed,
-- the entire family is invalidated, forcing re-login on all devices.
CREATE TABLE sessions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id           UUID NOT NULL,  -- refresh token family (all rotations share this)
    token_hash          TEXT NOT NULL UNIQUE,  -- SHA-256 hash of the refresh token
    device_name         TEXT,
    ip_address          INET,
    user_agent          TEXT,
    expires_at          TIMESTAMPTZ NOT NULL,
    revoked_at          TIMESTAMPTZ,   -- set when rotated or explicitly revoked
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_family_id ON sessions (family_id);
CREATE INDEX idx_sessions_token_hash ON sessions (token_hash);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);
