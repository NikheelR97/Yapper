# Frontend Development

The frontend is a SvelteKit application that compiles to a static site and runs as a Web PWA, Tauri v2 desktop app, and Capacitor mobile app from the same codebase.

## Project structure

```
frontend/src/
├── routes/
│   ├── (app)/            # Authenticated app shell
│   │   ├── +layout.svelte      # Root layout: WS connection, auth guard
│   │   ├── dm/                 # Direct messages
│   │   ├── servers/[id]/       # Server + channel views
│   │   ├── explore/            # Server discovery
│   │   ├── profile/[username]/ # User profiles
│   │   ├── settings/           # All settings sections
│   │   └── parent/             # Parental controls dashboard
│   ├── (auth)/           # Unauthenticated pages
│   │   ├── login/
│   │   ├── register/
│   │   └── forgot-password/
│   └── oauth/            # OAuth callback handler
│
├── lib/
│   ├── api/              # HTTP client (client.ts) + typed endpoints
│   ├── signal/           # E2EE: X3DH · ratchet · sender keys · keystore
│   ├── stores/           # Svelte stores: auth · ws · conversations · servers
│   ├── components/       # Shared UI components
│   │   ├── settings/     # All settings section components
│   │   ├── TitleBar.svelte     # Tauri window chrome
│   │   └── …
│   ├── desktop/          # Tauri-specific: notifications · updater · vault · deep links
│   └── plugins/
│       └── tauri-compat.ts    # Platform detection + unified API (isTauri · isCapacitor)
│
└── app.html              # HTML shell

frontend/tests/           # Playwright E2E specs
frontend/src-tauri/       # Tauri v2 configuration + Rust shell
frontend/ios/             # Capacitor iOS platform (CocoaPods)
frontend/android/         # Capacitor Android platform (Gradle)
```

## Platform detection

Always import from `tauri-compat.ts` — never access `window.__TAURI_INTERNALS__` directly:

```typescript
import { isTauri, isCapacitor, isNative, platform } from '$lib/plugins/tauri-compat.js';

const showTitleBar = isTauri();   // desktop only
const isPush = isCapacitor();     // mobile push via FCM
```

**Tauri v2 notes:**
- Runtime detection: `window.__TAURI_INTERNALS__` (not `__TAURI__` from v1)
- WebView2 origin on Windows: `http://tauri.localhost`
- Cookies must be `SameSite=None; Secure` to reach the API from Tauri

## Authentication flow

```
1. POST /api/v2/auth/login → { access_token, csrf_token, user, device }
2. Stored in authStore (Svelte store) + localStorage
3. refresh_token in HttpOnly cookie
4. api/client.ts auto-refreshes on 401 via POST /api/v2/auth/refresh
```

The `authStore` and CSRF token are attached to every API call automatically by `client.ts`.

## WebSocket store

`stores/ws.ts` manages the single WebSocket connection:

```typescript
import { connectWS } from '$stores/ws.js';

// Called once in +layout.svelte after login
connectWS(accessToken);

// Register a handler for a specific message type
registerHandler('new_message', (msg) => { … });
```

The WS connection auto-reconnects with exponential backoff. A banner is shown when disconnected.

## E2EE in the frontend

See [E2EE Implementation](E2EE-Implementation) for the full protocol. Frontend entry points:

```typescript
// Send an encrypted DM
import { encryptMessage } from '$lib/signal/index.js';

// Send an encrypted channel message
import { encryptChannelMessage } from '$lib/signal/sender_keys.js';

// Decrypt incoming message
import { decryptMessage } from '$lib/signal/index.js';
```

The Signal keystore is persisted in IndexedDB via `idb` (see `signal/keystore.ts`).

## Adding a new settings section

1. Create `src/lib/components/settings/MySection.svelte`
2. In `routes/(app)/settings/+page.svelte`:
   - Add `"my_section"` to the `Section` type union
   - Add `{ id: "my_section", label: "My Section" }` to `navItems`
   - Add `{:else if activeSection === "my_section"}<MySection />{/if}`

## Type checking and linting

```bash
cd frontend
npm run check    # svelte-check (TypeScript + Svelte)
npm run lint     # ESLint + Prettier check
npm run format   # Prettier auto-format
```

CI runs `npm run check` on every push.

## Building

```bash
# Web (static)
npm run build

# Desktop (Tauri)
npm run tauri build

# Mobile (after npm run build)
npx cap sync ios      # sync to Xcode project
npx cap sync android  # sync to Android Studio project
```

Production environment variables are loaded from `.env.production` automatically by Vite.

## Playwright E2E tests

Tests live in `frontend/tests/`. Each spec file is self-contained:

| File | Coverage |
|------|----------|
| `auth.spec.ts` | Login · register · OAuth buttons |
| `auth-shell.spec.ts` | Authenticated shell · settings · logout |
| `navigation.spec.ts` | Redirects · public pages · nav links |
| `dm.spec.ts` | DM index · send message |
| `servers.spec.ts` | Create server · channel messages · typing indicator · invite link |
| `explore.spec.ts` | Search · unauthenticated redirect |
| `social.spec.ts` | Follow · unfollow · friend requests |
| `channel-e2ee.spec.ts` | Cross-user encrypted message decryption |
| `multi-device.spec.ts` | Secondary device pending approval gate |

Run against production:

```bash
BASE_URL=https://app.yapperhq.com \
E2E_EMAIL=your@email.com \
E2E_PASSWORD=yourpassword \
npx playwright test
```
