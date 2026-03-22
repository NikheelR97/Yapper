# Database

Yapper uses PostgreSQL 16 via [Neon](https://neon.tech) (serverless, 0.5 GB free tier). The ORM-free access layer is [sqlx](https://github.com/launchbadge/sqlx) with compile-time query checking via an offline cache (`.sqlx/`).

## Migrations

All migrations live in `backend/migrations/` and are applied automatically on server startup via `sqlx::migrate!()`.

| # | File | Description |
|---|------|-------------|
| 001 | `20260225000001_initial_schema.sql` | Users, servers, channels, messages, relationships |
| 002 | `20260225000002_add_refresh_tokens.sql` | JWT refresh tokens |
| 003 | `20260225000003_add_invites.sql` | Server invite links |
| 004 | `20260226000004_add_email_verification.sql` | Email verification tokens |
| 005 | `20260226000005_add_password_reset.sql` | Password reset tokens |
| 006 | `20260227000006_add_oauth_providers.sql` | OAuth provider columns |
| 007 | `20260228000007_add_signal_keys.sql` | Signal identity keys + prekeys |
| 008 | `20260228000008_add_media.sql` | Media attachments |
| 009 | `20260301000009_add_signing_key.sql` | `signing_key` on identity_keys |
| 010 | `20260301000010_add_pin_backup.sql` | E2EE key backup (PIN-encrypted) |
| 011 | `20260302000011_parental_controls.sql` | Child accounts, parent relationships, approval queues |
| 012 | `20260303000012_canvas_and_explore.sql` | Canvas (music/polls/clips), server tags, search indexes |
| 013 | `20260303000013_screen_time.sql` | Screen time reports |
| 014 | `20260303000014_profile_and_social.sql` | Follow graph, friend requests, hype moments |
| 015 | `20260303000015_user_settings.sql` | Privacy, appearance, notification preferences |
| 016 | `20260303000016_sender_keys.sql` | Channel Sender Key distributions |
| 017 | `20260303000017_audit_log.sql` | Parental audit trail |
| 018 | `20260303000018_emojis.sql` | Custom server emojis |
| 019 | `20260304000019_bots.sql` | Discord bot import (token hash) |
| 020 | `20260304000020_premium.sql` | Subscription status, promo codes |
| 021 | `20260306000019_multidevice_e2ee.sql` | Devices, Signal device IDs, trust states |
| 022 | `20260307000020_dm_envelope_msg_num.sql` | DM envelope message numbers |
| 023 | `20260307000021_fix_device_installation_unique_index.sql` | Index fix |
| 024 | `20260309000022_auth_security_hardening.sql` | Login attempt tracking |
| 025 | `20260309000023_dm_double_ratchet.sql` | Double ratchet chain state columns |
| 026 | `20260315000026_support_tickets.sql` | User support tickets + HubSpot ID |
| 027 | `20260316000027_device_sync_events.sql` | Device sync events for multi-device trust |
| 028 | `20260321000028_push_tokens.sql` | FCM push token registration |
| 029 | `20260321000029_canvas_expansion.sql` | Canvas music queue, enhanced polls, clip reactions, events |
| 030 | `20260322000030_media_uploads.sql` | Media upload tracking + per-user quota |

## Key tables

### users
```sql
id              UUID PRIMARY KEY
username        TEXT UNIQUE NOT NULL
display_name    TEXT NOT NULL
email           TEXT UNIQUE NOT NULL
password_hash   TEXT
account_type    TEXT  -- 'standard' | 'parent' | 'child' | 'bot'
parental_controls_enabled BOOLEAN
date_of_birth   DATE
deleted_at      TIMESTAMPTZ  -- soft delete
```

### messages
```sql
id              UUID PRIMARY KEY
channel_id      UUID REFERENCES channels
conversation_id UUID REFERENCES conversations
sender_id       UUID REFERENCES users
ciphertext      BYTEA       -- AES-256-GCM encrypted
message_number  INTEGER     -- double ratchet sequence
created_at      TIMESTAMPTZ
```

### identity_keys
```sql
user_id         UUID REFERENCES users
device_id       INTEGER
ik_public       BYTEA   -- identity key (Ed25519)
spk_public      BYTEA   -- signed prekey (X25519)
spk_signature   BYTEA   -- SPK signed by IK
signing_key     BYTEA   -- channel signing key (Ed25519)
```

### devices
```sql
id                  UUID PRIMARY KEY
user_id             UUID REFERENCES users
signal_device_id    INTEGER UNIQUE per user
installation_id     UUID UNIQUE
platform            TEXT  -- 'web' | 'tauri' | 'capacitor'
trust_state         TEXT  -- 'trusted' | 'pending_trust' | 'revoked'
```

## Running migrations locally

```bash
cd backend
sqlx migrate run
```

## Creating a new migration

```bash
sqlx migrate add my_feature_name
# Writes: backend/migrations/{timestamp}_my_feature_name.sql
```

Edit the generated file, then run `sqlx migrate run`.

## Updating the sqlx offline cache

The `.sqlx/` directory contains query metadata for compile-time checks in CI (where no live DB is available). After changing any query or schema:

```bash
# Must use the direct (non-pooler) Neon endpoint — pooler = PgBouncer = no prepared stmts
DATABASE_URL=postgres://…direct-endpoint… cargo sqlx prepare
git add .sqlx/
git commit -m "chore: update sqlx query cache"
```

## Useful psql queries

```sql
-- Check migration status
SELECT * FROM _sqlx_migrations ORDER BY installed_on DESC;

-- Active WebSocket sessions (via hub presence)
SELECT id, username, last_seen_at FROM users
WHERE last_seen_at > NOW() - INTERVAL '5 minutes';

-- Support tickets awaiting HubSpot sync
SELECT id, ticket_type, subject, created_at
FROM support_tickets
WHERE hubspot_ticket_id IS NULL
ORDER BY created_at DESC;
```
