-- PIN-encrypted Signal keystore backup
-- The encrypted_blob is AES-256-GCM ciphertext produced on the client.
-- The server stores it opaquely — never decrypts it.
CREATE TABLE key_backups (
    user_id       UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    encrypted_blob BYTEA NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
