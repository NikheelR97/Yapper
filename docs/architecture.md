# Architecture

## Overview

Yapper is a monorepo containing four independently deployable units:

```
┌──────────────────────────────────────────────────────────────────┐
│                            CLIENTS                               │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────┐   │
│  │  Web (PWA)  │  │  Desktop    │  │  Mobile                  │   │
│  │  CF Pages   │  │  Tauri v2   │  │  Capacitor (iOS/Android)│   │
│  └──────┬───────┘  └──────┬──────┘  └──────────────┬───────────┘   │
│         └──────────────────┴──────────────────────┴──────────────┘
│                               │ HTTPS + WSS                        │
│                               ▼                                    │
│                     BACKEND (Fly.io)                               │
│                                                                  │
│   Rust + Axum + Tokio                              ┌──────────┐    │
│   ┌──────────────┐   ┌──────────────────────┐     │ Hub      │    │
│   │ REST API     │   │ WebSocket Handler    │     │ (RAM)    │    │
│   │ /api/v2/*    │   │ /ws                  │     │ DashMap  │    │
│   └──────┬───────┘   └──────────┬───────────┘     └──────────┘    │
│          │                      │                                   │
│          └──────────────┬───────┘                                   │
│                         ▼                                           │
│                  sqlx pool / Neon PostgreSQL                       │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Backend Modules

All modules live under `backend/src/`.

| Module | Route prefix | Responsibility |
|--------|-------------|----------------|
| `auth` | `/api/v2/auth/*`, `/auth/oauth/*` | Register, login, refresh, logout, email verify, password reset, OAuth |
| `users` | `/api/v2/users/*`, `/api/v2/account/*` | Profiles, follow graph, friend requests, hype moments, settings, account actions |
| `servers` | `/api/v2/servers/*` | Server CRUD, membership, invite links, join/leave |
| `channels` | `/api/v2/channels/*` | Channel CRUD, sender key distribution, message history |
| `messages` | `/api/v2/conversations/*` | 1:1 DM conversations, message delivery |
| `keys` | `/api/v2/keys/*` | Signal Protocol key server (identity, signed prekey, OPKs, backup) |
| `canvas` | `/api/v2/canvas/*` | Live Canvas: music, polls, clips per server |
| `explore` | `/api/v2/explore/*`, `/api/v2/search` | Discovery: communities, live servers, tags, search, top yappers |
| `parental` | `/api/v2/parental/*` | COPPA child account management, approval workflows, audit trail |
| `media` | `/api/v2/media/*` | R2 presigned URLs for encrypted upload/download |
| `emojis` | `/api/v2/emojis/*` | Custom server emoji CRUD |
| `notifications` | `/api/v2/notifications/*` | FCM push notification dispatch |
| `screentime` | `/api/v2/screentime/*` | Screen time report ingestion + aggregation |
| `bots` | `/api/v2/bots/*` | Bot application management |
| `discord` | `/api/v2/discord/*` | Discord profile import + bot migration |
| `support` | `/api/v2/support/*` | Support tickets with HubSpot CRM integration |
| `devices` | `/api/v2/devices/*` | Multi-device management, trust states, sync events |
| `constants` | `-` | Centralized limits |
| `hub` | `-` | In-memory WebSocket hub, rate limiting, typing timers, away detection |
| `csrf` | `-` | Double-submit CSRF middleware |
| `error` | `-` | Unified `AppError` to HTTP status mapping |
| `db` | `-` | sqlx connection-pool wrapper |

## WebSocket Hub

The hub (`src/hub.rs`) is the real-time core. It holds:

```rust
pub struct Hub {
    connections: DashMap<Uuid, DashMap<ConnectionId, ConnTx>>,
    device_connections: DashMap<Uuid, DashMap<Uuid, ConnectionId>>,
    connection_meta: DashMap<ConnectionId, ConnectionMeta>,
    msg_limiters: DashMap<Uuid, MsgRateLimiter>,
    typing_timers: DashMap<(Uuid, Uuid), JoinHandle<()>>,
    away_timers: DashMap<Uuid, JoinHandle<()>>,
    away_users: DashMap<Uuid, ()>,
}
```

Inbound message types:

| Type | Action |
|------|--------|
| `Auth { token }` | Authenticate the WS connection with a JWT |
| `Reauth { token }` | Refresh auth without reconnecting |
| `SendDm` | Send an E2EE direct message |
| `SendChannel` | Send an E2EE channel message |
| `TypingStart` | Start/reset typing indicator for a channel |
| `Read { message_id, channel_id }` | Mark a message read |
| `Ping` | Keepalive |

Outbound message types:

| Type | Description |
|------|-------------|
| `Message` | Delivered DM or channel message |
| `Typing` / `TypingStop` | Typing indicator events |
| `ReadReceipt` | Read-receipt fan-out |
| `Presence` | Online / away / offline status |
| `CanvasUpdate` | Live Canvas widget update |
| `ParentNotification` | Real-time parental alert |
| `Error` | WS-level error response |
| `Pong` | Keepalive reply |
