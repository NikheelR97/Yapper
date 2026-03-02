-- Yapper Wishlist -- D1 schema
-- Run with: wrangler d1 execute yapper-wishlist --remote --file=worker/schema.sql

CREATE TABLE IF NOT EXISTS wishlist (
    id         INTEGER  PRIMARY KEY AUTOINCREMENT,
    email      TEXT     NOT NULL UNIQUE COLLATE NOCASE,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_wishlist_created ON wishlist (created_at);
