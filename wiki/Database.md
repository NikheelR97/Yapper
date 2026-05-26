# Database

Yapper uses PostgreSQL 16 via [Neon](https://neon.tech) (serverless, 0.5 GB free tier). The ORM-free access layer is [sqlx](https://github.com/launchbadge/sqlx) with compile-time query checking via an offline cache (`.sqlx/`).

## Migrations

All migrations live in `backend/migrations/` and are applied automatically on server startup via `sqlx::migrate!()`.

| # | File | Description |
|---|------|-------------|
| 001 | `20260301000001_users.sql` | Users and core account fields |
| 002 | `20260301000002_sessions.sql` | Sessions and refresh-token storage |
| 003 | `20260301000003_signal_keys.sql` | Signal identity keys, signed prekeys, and one-time prekeys |
| 004 | `20260301000004_servers_channels.sql` | Servers, channels, memberships, and invite links |
| 005 | `20260301000005_messages.sql` | DM conversations, messages, and read receipts |
| 006 | `20260301000006_social.sql` | Friendships, followers, and hype moments |
| 007 | `20260301000007_parental.sql` | Parent/child relationships and approval queues |
| 008 | `20260301000008_canvas_emojis.sql` | Initial Canvas and custom emoji tables |
| 009 | `20260301000009_add_signing_key.sql` | Channel signing key support |
| 010 | `20260301000010_key_backups.sql` | PIN-encrypted E2EE key backups |
| 011 | `20260301000011_sender_keys_group.sql` | Sender Key group distribution |
| 012 | `20260301000012_explore_tags.sql` | Explore tags and discovery metadata |
| 013 | `20260303000013_screentime_settings.sql` | Screen-time settings |
| 014 | `20260303000014_user_settings.sql` | User settings |
| 015 | `20260303000015_emoji_indexes.sql` | Emoji indexes |
| 016 | `20260303000016_user_appearance_settings.sql` | Appearance preferences |
| 017 | `20260303000017_user_notification_settings.sql` | Notification preferences |
| 018 | `20260304000018_screen_time_records.sql` | Screen-time records |
| 019 | `20260304000019_bots.sql` | Bot application tables |
| 020 | `20260304000020_premium.sql` | Premium status and promo codes |
| 021 | `20260306000019_multidevice_e2ee.sql` | Multi-device E2EE metadata and trust state |
| 022 | `20260307000020_dm_envelope_msg_num.sql` | DM envelope message numbers |
| 023 | `20260307000021_fix_device_installation_unique_index.sql` | Device installation uniqueness fix |
| 024 | `20260309000022_auth_security_hardening.sql` | Login-attempt tracking and auth hardening |
| 025 | `20260309000023_dm_double_ratchet.sql` | Double Ratchet chain-state columns |
| 026 | `20260312000024_relax_msg_has_content_for_v2_dms.sql` | V2 DM envelope content constraint relaxation |
| 027 | `20260312000025_friendships_requested_by.sql` | Friend request initiator tracking |
| 028 | `20260315000026_support_tickets.sql` | Support tickets and HubSpot sync ID |
| 029 | `20260320000027_performance_indexes.sql` | Performance indexes and counters |
| 030 | `20260321000028_push_tokens.sql` | FCM push-token registration |
| 031 | `20260321000029_canvas_expansion.sql` | Canvas music queue, enhanced polls, clips, reactions, and events |
| 032 | `20260322000030_media_uploads.sql` | Media upload tracking and per-user quota |
| 033 | `20260324000031_deleted_account_retention.sql` | Deleted-account retention metadata |
| 034 | `20260326000032_normalize_email_and_linked_identities.sql` | Normalized email and linked identities |
| 035 | `20260326000033_message_and_join_invariants.sql` | Message and join invariants |
| 036 | `20260327000034_media_upload_expiry_backfill.sql` | Media upload expiry backfill |
| 037 | `20260328000031_fix_message_ciphertext_xor_plaintext.sql` | Enforce ciphertext/plaintext XOR constraint |

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
