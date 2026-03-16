# Architecture

## High-level overview

```
┌─────────────────────────────────────────────────────────────────┐
│  Clients                                                        │
│  ┌──────────┐  ┌─────────────────┐  ┌──────────────────────┐  │
│  │  Web PWA │  │ Desktop (Tauri) │  │  Mobile (Capacitor)  │  │
│  │ SvelteKit│  │    + Tauri v2   │  │  iOS · Android       │  │
│  └────┬─────┘  └────────┬────────┘  └──────────┬───────────┘  │
└───────┼─────────────────┼────────────────────────┼─────────────┘
        │  HTTPS / WSS    │                        │
        ▼                 ▼                        ▼
┌──────────────────────────────────────────────────┐
│  Fly.io — yapper-api (Rust + Axum)               │
│                                                  │
│  HTTP API (v1 · v2)   WebSocket hub              │
│  ┌─────────────────┐  ┌────────────────────────┐│
│  │ auth · users    │  │ per-user channels       ││
│  │ servers · DMs   │  │ tokio mpsc broadcast    ││
│  │ keys · media    │  │ presence · typing       ││
│  │ canvas · explore│  │ read receipts           ││
│  │ parental · etc. │  └────────────────────────┘│
│  └────────┬────────┘                             │
└───────────┼──────────────────────────────────────┘
            │
     ┌──────┴──────────────────────┐
     │                             │
     ▼                             ▼
┌─────────────┐           ┌──────────────────┐
│  Neon       │           │  Cloudflare R2   │
│  PostgreSQL │           │  (media uploads) │
│  (primary   │           │  10 GB free      │
│   store)    │           └──────────────────┘
└─────────────┘
```

## Tech stack

### Backend
| Layer | Choice | Reason |
|-------|--------|--------|
| Language | Rust 1.85 | Memory safety, ~10 MB idle RAM (critical on 256 MB Fly VM), native libsignal support |
| Web framework | Axum 0.7 | Tokio-native, ergonomic extractors, WebSocket support |
| Database | sqlx 0.7 (async, compile-time checked) | No ORM overhead; offline query cache for CI |
| Auth | JWT RS256 (jsonwebtoken) + Argon2 | Industry standard; RS256 allows public-key verification |
| Rate limiting | governor + dashmap | In-memory, no Redis dependency |
| Image processing | image crate | Pure Rust; converts uploads to WebP |
| Error monitoring | Sentry 0.34 | Panic capture + structured traces |
| Hosting | Fly.io (jnb region) | Always-on free tier; fast deploy via pre-built image |

### Frontend
| Layer | Choice | Reason |
|-------|--------|--------|
| Framework | SvelteKit (static adapter) | Minimal JS bundle, excellent TypeScript support |
| Desktop | Tauri v2 | Rust-based shell; smaller binary than Electron |
| Mobile | Capacitor 8 | Shared SvelteKit codebase for iOS + Android |
| Crypto | @noble/curves + @noble/hashes | Pure JS, audited, no WASM; runs in WebView |
| State | Svelte stores | Fine-grained reactivity without boilerplate |
| E2E testing | Playwright | Sharded CI runs; reuses saved auth state |

### Infrastructure (all free tier)
| Service | Role |
|---------|------|
| Fly.io | Backend API (2 always-on machines, jnb) |
| Neon | PostgreSQL 0.5 GB |
| Cloudflare Pages | Frontend + marketing hosting |
| Cloudflare R2 | Media storage (10 GB) |
| Cloudflare Workers + D1 | Wishlist API |
| Resend | Transactional email (3 K/month) |
| Firebase / FCM | Push notifications |
| Sentry | Error monitoring (5 K errors/month) |

## Real-time architecture

The backend uses a single in-memory WebSocket hub (`src/hub.rs`):

```
Client A ──WS──► hub ──broadcast──► Client B (same user, other device)
                  │
                  └──fan-out──► Client C (different user, same channel)
```

- Each connected client gets a `tokio::sync::mpsc` sender stored in `Arc<RwLock<HashMap<UserId, Vec<Sender>>>>`
- Messages are serialised to JSON and sent to all matching receivers
- `typing_timers: DashMap<(channel_id, user_id), JoinHandle>` — auto-cancels typing indicators after 5 s
- `away_timers: DashMap<UserId, JoinHandle>` — marks users away after 5 min inactivity
- `PostgreSQL LISTEN/NOTIFY` is available for cross-process fan-out (post-MVP multi-instance)

## E2EE summary

See the [E2EE Implementation](E2EE-Implementation) page for full details.

- **DMs**: X3DH key agreement → shared secret → double ratchet (AES-256-GCM). Server never sees plaintext.
- **Channels**: Sender Keys (HMAC-SHA256 chain + Ed25519 signing + ECIES distribution). Single key per sender per channel.
- **Media**: Client encrypts with AES-256-GCM before upload to R2. Decryption key is embedded in the encrypted message payload.
- **Key backup**: PIN-derived encryption of the Signal keystore stored server-side.

## Multi-platform client

```
SvelteKit (shared source)
    │
    ├── vite build ──► Cloudflare Pages (Web PWA)
    │
    ├── tauri build ──► .exe / .dmg / .AppImage (Tauri v2 shell)
    │                   Auto-updater · System tray · Deep links
    │
    └── npx cap sync ──► iOS (Xcode) / Android (Gradle)
                         Capacitor push notifications
```

Tauri-specific code is guarded by `isTauri()` from `src/lib/plugins/tauri-compat.ts`.
Capacitor-specific code is guarded by `isCapacitor()` from the same module.
