-- Tags on servers for explore/discovery filtering
ALTER TABLE servers ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}';
CREATE INDEX idx_servers_tags ON servers USING gin (tags);

-- Full-text search index on users (username + display_name)
CREATE INDEX idx_users_search ON users
    USING gin ((username || ' ' || COALESCE(display_name, '')) gin_trgm_ops)
    WHERE deleted_at IS NULL;
