-- Add index on server_emojis(server_id) for fast list queries
-- The server_emojis table already exists from migration 000008

CREATE INDEX IF NOT EXISTS idx_server_emojis_server
    ON server_emojis (server_id, created_at DESC);
