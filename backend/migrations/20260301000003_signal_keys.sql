-- Signal Protocol key storage (public keys only — private keys never leave clients)

-- Identity keys: one per user per device
CREATE TABLE identity_keys (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id       INTEGER NOT NULL DEFAULT 1,
    public_key      BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id)
);

-- Signed prekeys: rotated weekly
CREATE TABLE signed_prekeys (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id       INTEGER NOT NULL DEFAULT 1,
    key_id          INTEGER NOT NULL,
    public_key      BYTEA NOT NULL,
    signature       BYTEA NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id, key_id)
);

CREATE INDEX idx_signed_prekeys_user ON signed_prekeys (user_id, device_id);

-- One-time prekeys: batch of 100 uploaded at registration, consumed on each session init
CREATE TABLE one_time_prekeys (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id       INTEGER NOT NULL DEFAULT 1,
    key_id          INTEGER NOT NULL,
    public_key      BYTEA NOT NULL,
    consumed        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, device_id, key_id)
);

-- Partial index: only non-consumed OPKs — the most queried subset
CREATE INDEX idx_otpk_available ON one_time_prekeys (user_id, device_id, consumed)
    WHERE consumed = FALSE;
