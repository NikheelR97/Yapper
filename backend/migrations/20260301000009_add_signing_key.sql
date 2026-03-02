-- Add Ed25519 signing key storage to identity_keys
-- The existing public_key column holds the X25519 DH key; signing_key holds Ed25519.
ALTER TABLE identity_keys ADD COLUMN IF NOT EXISTS signing_key BYTEA;
