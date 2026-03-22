# Architecture

## Overview

Yapper is a monorepo containing four independently deployable units:

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENTS                                  │
│                                                                  │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │  Web (PWA)  │  │ Desktop      │  │ Mobile               │   │
│  │ CF Pages    │  │ Tauri v2     │  │ Capacitor (iOS/Andr) │   │
│  └──────┬──────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         └────────────────┴───────────────────────┘              │
└──────────────────────────────┬──────────────────────────────────┘
                               │ HTTPS + WSS
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      BACKEND  (Fly.io)                           │
│                                                                  │
│   Rust + Axum + Tokio                              ┌──────────┐  │
│   ┌──────────────────┐   ┌──────────────────────┐ │  Hub     │  │
│   │   REST API       │   │   WebSocket Handler  │ │  (RAM)   │  │
│   │   /api/v1/*      │   │   /ws                │ │          │  │
│   └────────┬─────────┘   └──────────┬───────────┘ │ DashMap  │  │
│            │                        │             │ mpsc     │  │
│            └────────────┬───────────┘             └────┬─────┘  │
│                         │                              │         │
│                  ┌──────▼──────┐               fan-out│         │
│                  │  sqlx pool  │◄──────────────────────┘         │
│                  └──────┬──────┘                                 │
└─────────────────────────┼───────────────────────────────────────┘
                          │
                          ▼
          ┌───────────────────────────────┐
          │  Neon (PostgreSQL serverless) │
          └───────────────────────────────┘

          ┌──────────────┐   ┌─────────────────────┐
          │ Cloudflare   │   │ Firebase (FCM)       │
          │ R2 (media)   │   │ Push notifications   │
          └──────────────┘   └─────────────────────┘
```

---

## Backend Modules

All modules live under `backend/src/`. Each is a Rust module with its own `mod.rs` (or subdirectory).

| Module | Route prefix | Responsibility |
|--------|-------------|----------------|
| `auth` | `/auth/*`, `/auth/oauth/*` | Register, login, refresh, logout, email verify, password reset, Discord/Google/Apple OAuth, CSRF |
| `users` | `/api/v1/users/*` | Profiles, follow graph, friend requests (parental-intercepted), hype moments, activity feed |
| `servers` | `/api/v1/servers/*` | Server CRUD, membership, invite links, join/leave |
| `channels` | `/api/v1/channels/*` | Channel CRUD, sender key distribution, message history |
| `messages` | `/api/v1/conversations/*` | 1:1 DM conversations, message delivery |
| `keys` | `/api/v1/keys/*` | Signal Protocol key server (identity, signed prekey, OPKs, backup) |
| `canvas` | `/api/v1/canvas/*` | Live Canvas — music, polls, clips per server |
| `explore` | `/api/v1/explore/*` | Discovery — communities, live servers, trending tags, search, top yappers |
| `parental` | `/api/v1/parental/*` | COPPA child account management, approval workflows, audit trail |
| `media` | `/api/v1/media/*` | R2 presigned URLs for encrypted upload/download |
| `emojis` | `/api/v1/emojis/*` | Custom server emoji CRUD |
| `notifications` | `/api/v1/notifications/*` | FCM push notification dispatch |
| `screentime` | `/api/v1/screentime/*` | Screen time report ingestion + aggregation |
| `bots` | `/api/v1/bots/*` | Bot application management |
| `discord` | `/api/v1/discord/*` | Discord profile import + bot migration |
| `support` | `/api/v1/support/*` | Support tickets with HubSpot CRM integration |
| `devices` | `/api/v1/devices/*` | Multi-device management, trust states, sync events |
| `constants` | — | Centralized limits (`MAX_UPLOAD_SIZE`, `MAX_UPLOADS_PER_MINUTE`, etc.) |
| `hub` | — | In-memory WebSocket hub, rate limiting, typing timers, away detection |
| `csrf` | — | Double-submit CSRF middleware |
| `error` | — | Unified `AppError` → HTTP status mapping |
| `db` | — | sqlx connection pool wrapper |

---

## WebSocket Hub

The hub (`src/hub.rs`) is the real-time core. It holds:

```rust
pub struct Hub {
    connections:      DashMap<Uuid, DashMap<ConnectionId, ConnTx>>,
    device_connections: DashMap<Uuid, DashMap<Uuid, ConnectionId>>, // user → device → conn
    connection_meta:  DashMap<ConnectionId, ConnectionMeta>,        // conn → metadata
    msg_limiters:     DashMap<Uuid, MsgRateLimiter>,   // 5 msg/sec, burst 20
    typing_timers:    DashMap<(Uuid, Uuid), JoinHandle<()>>, // auto-stop after 5s
    away_timers:      DashMap<Uuid, JoinHandle<()>>,   // 5 min inactivity → away
    away_users:       DashMap<Uuid, ()>,               // currently-away set
}
```

**Inbound message types** (`WsInbound`):

| Type | Action |
|------|--------|
| `Auth { token }` | Authenticate the WS connection with a JWT |
| `Reauth { token }` | Refresh auth without reconnecting |
| `SendDm` | Send an E2EE direct message |
| `SendChannel` | Send an E2EE channel message (fan-out to all members) |
| `TypingStart` | Start/reset typing indicator for a channel |
| `Read { message_id, channel_id }` | Mark a message read (upsert + fan-out) |
| `Ping` | Keepalive |

**Outbound message types** (`WsOutbound`):

| Type | Description |
|------|-------------|
| `Message` | Delivered DM or channel message |
| `Typing` / `TypingStop` | Typing indicator events |
| `ReadReceipt` | Read receipt fan-out |
| `Presence` | Online / away / offline status |
| `CanvasUpdate` | Live Canvas widget update (music, poll, clips) |
| `ParentNotification` | Real-time parental alert (friend request, server join) |
| `Error` | WS-level error response |
| `Pong` | Keepalive reply |

---

## Database Schema

Managed via `sqlx` migrations in `backend/migrations/`.

| Migration | Tables |
|-----------|--------|
| 000001 users | `users` |
| 000002 sessions | `sessions`, `email_verifications`, `password_resets` |
| 000003 signal_keys | `identity_keys`, `signed_prekeys`, `one_time_prekeys`, `prekey_bundles` |
| 000004 servers_channels | `servers`, `server_memberships`, `server_invite_links`, `channels` |
| 000005 messages | `messages`, `channel_messages`, `read_receipts` |
| 000006 social | `friendships`, `followers`, `hype_moments`, `pending_friend_requests`, `pending_server_joins` |
| 000007 parental | `parent_child_relationships`, `parent_notifications`, `parental_action_audit` |
| 000008 canvas_emojis | `canvas_state`, `canvas_polls`, `canvas_poll_votes`, `canvas_clips`, `server_emoji` |
| 000009 add_signing_key | `signing_key BYTEA` on `identity_keys` |
| 000010 key_backups | `key_backups` |
| 000011 sender_keys_group | `sender_keys`, `sender_key_distributions` |
| 000012 explore_tags | `tags TEXT[]` on servers, GIN trigram indexes |

---

## Data Flow: Sending an E2EE Channel Message

```
Client A (sender)
  │
  │  1. Encrypt with sender key (AES-256-GCM)
  │     wire: base64(Ed25519_sig_64 || AES_ciphertext)
  │
  ▼
WebSocket hub (SendChannel)
  │
  │  2. Authenticate + rate-limit
  │  3. Persist to channel_messages (ciphertext only)
  │  4. Query all channel members (≤ 500)
  │  5. Fan-out WsOutbound::Message to each online member
  │  6. Store offline for disconnected members
  │
  ▼
Client B, C, D… (receivers)
  │
  │  7. Verify Ed25519 signature
  │  8. Decrypt with cached sender key
  │
  ▼
  Plaintext rendered in UI
```

---

## Parental Controls Architecture

Parental controls intercept at two integration points without touching E2EE:

```
Friend request to child
  │
  ├─ parental_controls_enabled = FALSE → create friendship (pending status)
  │
  └─ parental_controls_enabled = TRUE
       │
       ├── INSERT pending_friend_requests
       ├── INSERT parent_notifications
       └── WsOutbound::ParentNotification → parent's WS session

Server join by child
  │
  ├─ parental_controls_enabled = FALSE → INSERT server_memberships
  │
  └─ parental_controls_enabled = TRUE
       │
       ├── INSERT pending_server_joins
       ├── INSERT parent_notifications
       └── WsOutbound::ParentNotification → parent's WS session

Parent approves
  │
  ├─ Friend: INSERT friendships (accepted)
  └─ Server: INSERT server_memberships (direct, bypasses re-check)
       │
       └─ INSERT parental_action_audit (every action logged)
```

**Privacy guarantee:** Parents see only *metadata* — who, when, which server. Message content is E2EE and inaccessible to the server.

---

## Frontend Architecture

```
frontend/src/
├── lib/
│   ├── signal/          # E2EE — all crypto runs here
│   │   ├── x3dh.ts      # X3DH key agreement
│   │   ├── ratchet.ts   # Symmetric ratchet (AES-256-GCM)
│   │   ├── keystore.ts  # IndexedDB key persistence
│   │   ├── senderkey.ts # Group sender keys (HMAC-SHA256 chain)
│   │   └── backup.ts    # PIN-encrypted key backup
│   │
│   ├── stores/          # Svelte writable stores
│   │   ├── ws.ts        # WS connection, message routing, handler registry
│   │   ├── auth.ts      # Auth state, token refresh
│   │   ├── servers.ts   # Server list, channel messages
│   │   ├── canvas.ts    # Live Canvas per-server state
│   │   └── explore.ts   # Discovery page state
│   │
│   ├── api/             # Typed REST client
│   └── components/      # Reusable Svelte components
│       ├── canvas/      # LiveCanvas, MusicWidget, PollWidget, ClipsCarousel
│       └── explore/     # TrendingTags, LiveServerCard, CommunityCard
│
└── routes/
    ├── (auth)/          # /login, /register, /oauth/callback
    └── (app)/           # Authenticated shell
        ├── +layout.svelte   # WS connect, handler registration, nav
        ├── servers/[id]/channels/[channelId]/  # Main chat + LiveCanvas panel
        ├── conversations/[id]/  # 1:1 DM
        ├── explore/     # Discovery page
        └── profile/[username]/  # Public profile (WIP)
```

### WS Message Routing

`ws.ts` maintains a handler registry (`Map<string, Set<Function>>`). Components register handlers via `onWsMessage(type, handler)` and clean up in `onDestroy`. The layout registers global handlers (canvas updates, parent notifications) on mount.
