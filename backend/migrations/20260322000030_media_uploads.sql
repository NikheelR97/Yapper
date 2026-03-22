-- Track media uploads for quota enforcement and garbage collection.
-- The server never stores plaintext media — only the R2 object key and metadata.

CREATE TABLE media_uploads (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    object_key  TEXT NOT NULL UNIQUE,
    media_type  TEXT NOT NULL CHECK (media_type IN ('yap', 'clip')),
    size_bytes  BIGINT NOT NULL CHECK (size_bytes > 0),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ -- NULL = no auto-expiry; set by lifecycle rules
);

-- Per-user upload listing (newest first).
CREATE INDEX idx_media_uploads_user ON media_uploads (user_id, created_at DESC);

-- Cleanup cursor: find expired uploads for R2 garbage collection.
CREATE INDEX idx_media_uploads_expiry ON media_uploads (expires_at)
    WHERE expires_at IS NOT NULL;
