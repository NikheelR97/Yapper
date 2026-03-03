# S10 & S11 — Desktop Polish, Premium & Launch UI Spec

**Sprint S10:** W21–W22 — Tauri Desktop Polish + Security Audit
**Sprint S11:** W23–W24 — Premium Placeholder + Launch Preparation

---

## S10 — Tauri Desktop Specifics

### Custom Title Bar

**File:** `frontend/src/lib/components/TitleBar.svelte`
(Shown only in Tauri, hidden in web/Capacitor)

```
┌──────────────────────────────────────────────────────────────────────┐
│  [sphere icon 16px] Yapper  [channel name]         [─] [□] [✕]      │
└──────────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Height | 32px |
| Background | `#0a0a0f` (matches app) |
| Drag area | Full width except window controls — `data-tauri-drag-region` |
| Window controls | Right-aligned: minimize, maximize, close |
| Minimize | `─` icon, 30px wide |
| Maximize | `□` icon, 30px wide |
| Close | `✕` icon, 30px wide, hover: red `#ef4444` bg |
| App icon | 16px sphere icon, left |
| App name | "Yapper" Inter 500, 13px |
| Current location | Dim text after name: "· #general-yapping" |

```css
/* Prevent text selection in title bar */
.title-bar { -webkit-user-select: none; user-select: none; }
```

---

### System Tray

**File:** `frontend/src-tauri/src/main.rs` (Tauri tray plugin)

Tray icon: the sphere icon (32×32 ICO/PNG)

**Tray right-click menu:**
```
Yapper
─────────────────
● neo_kai is online (preview)
─────────────────
Open Yapper
Settings
─────────────────
Quit Yapper
```

**Tray icon states:**
- Normal: sphere icon (no badge)
- Unread messages: sphere icon + red dot badge (OS notification badge)
- Do Not Disturb: sphere icon desaturated/dimmed

---

### Native Notifications (Tauri)

Triggered by WS events when app is backgrounded:

```
┌─────────────────────────────────────────────────┐
│  [sphere icon]  Yapper                          │
│  neo_kai sent you a message                     │
│  [REPLY]                                        │
└─────────────────────────────────────────────────┘
```

| Notification type | Template |
|-------------------|----------|
| DM | "neo_kai sent you a message" |
| Channel mention | "neo_kai mentioned you in #general" |
| Friend request | "neo_kai wants to be friends" |
| Parent alert | "New request requires your approval" |
| Server invite | "neo_kai invited you to Retro Gamers" |

---

### Deep Link Handler

`yapper://invite/abc123x` → opens Yapper + shows join server modal
`yapper://user/neo_kai` → opens Yapper + navigates to profile

No special UI — just routing via Tauri `open` events.

---

### Keyboard Shortcuts Reference

**File:** `frontend/src/lib/components/KeyboardShortcutsModal.svelte`

Triggered by `Ctrl+/`:

```
┌──────────────────────────────────────────────────────────┐
│  Keyboard Shortcuts                               [✕]    │
│                                                          │
│  NAVIGATION                                              │
│  Ctrl + K         Open quick search                     │
│  Ctrl + ,         Open settings                         │
│  Alt + ↑ / ↓     Navigate channels                     │
│                                                          │
│  MESSAGING                                               │
│  Enter            Send message                          │
│  Shift + Enter    New line                              │
│  ↑ (empty input) Edit last message                     │
│  Escape           Cancel / close modal                  │
│                                                          │
│  MEDIA                                                   │
│  Ctrl + Y         Start Yap recording                   │
│  Ctrl + Shift + V  Start Clip recording                 │
│                                                          │
│  OTHER                                                   │
│  Ctrl + /         Show this help                        │
│  Ctrl + M         Toggle mute                           │
└──────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Modal | max-width 480px, glass card |
| Category label | Inter 600, 11px, #6b7280, uppercase |
| Shortcut row | 40px height, description left + key combo right |
| Key combo | `<kbd>` style: background rgba(255,255,255,0.1), border 1px rgba(255,255,255,0.15), border-radius 4px, padding 2px 8px, Inter 500, 13px, monospace |

---

### Stronghold Key Storage (Desktop)

No special UI — uses Tauri Stronghold plugin transparently.

**Indicator in Settings → Privacy & Safety:**
```
KEY STORAGE
✓ Tauri Stronghold (encrypted native storage)
  Your encryption keys are protected by your OS keychain.
```

On Web/Mobile: "IndexedDB (browser storage)" — same row, different text.

---

## S11 — Premium Placeholder

### GoPro Lock Overlay

Applied to premium-only features throughout the app:

```
┌──────────────────────────────────────────────────────┐
│  [blurred/disabled feature area]                     │
│                                                      │
│            🚀                                       │
│         GoPro Feature                               │
│    Animated avatars are a GoPro perk.               │
│    [Learn More]                                     │
└──────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Overlay | `position: absolute; inset: 0; backdrop-filter: blur(4px) brightness(0.5)` |
| Card | Centered, glass card, padding 24px |
| Rocket icon | 32px, brand purple |
| Heading | "GoPro Feature" Inter 700, 18px |
| Description | Feature-specific, 14px, #9ca3af |
| Button | Secondary "Learn More" → Settings → Premium tab |

---

### Premium Badge on Profile

Users with GoPro show a badge next to their name:

```
CyberPunkUser99  🚀
```

- Rocket emoji or custom SVG badge, 16px, brand purple, tooltip "Yapper GoPro"

---

### Server Emoji Limit UI

At 50 emojis (free limit) — shown in emoji manager:

```
┌────────────────────────────────────────────────────────┐
│  🚀  You've reached the 50 emoji limit.               │
│     Upgrade to GoPro to add up to 100 custom emojis. │
│     [Upgrade to GoPro]                               │
└────────────────────────────────────────────────────────┘
```

Amber/purple gradient banner.

---

## Launch / Onboarding Improvements (S11)

### App Loading Screen

Shown while the app boots (SvelteKit hydrates, Signal WASM loads):

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│                [sphere animation]                    │
│                  320px, pulsing                      │
│                                                      │
│                    Yapper                            │
│                                                      │
│     [████████████████████░░░░]  Loading...          │
│                                                      │
└──────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Full-screen | #0a0a0f background |
| Sphere | 120px, CSS animation |
| App name | Inter 800, 32px, below sphere |
| Progress bar | Linear indeterminate bar, brand purple |
| Steps shown | "Initializing encryption..." → "Connecting..." → "Loading your messages..." |
| Skip condition | Hides as soon as app is ready, no fixed minimum duration |

---

### New in This Version Toast

Shown once per version update:

```
┌────────────────────────────────────────────────────────┐
│  ✨  What's New in Yapper 2.4.0                   [✕] │
│     • Custom server emojis are here!                   │
│     • Profile themes now support custom hex colors     │
│     • Safety improvements for parental controls        │
│     [See full changelog]                               │
└────────────────────────────────────────────────────────┘
```

- Appears bottom-right, auto-dismisses after 8s
- Background: glass card with brand purple accent border-left

---

### Sentry Error Boundary

**File:** `frontend/src/lib/components/ErrorBoundary.svelte`

Shown when a component throws an unrecoverable error:

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│              ⚠️  Something went wrong.              │
│                                                      │
│   An unexpected error occurred. We've been          │
│   notified and are working on a fix.                │
│                                                      │
│   [Reload App]              [Report Issue]          │
└──────────────────────────────────────────────────────┘
```

---

## Global UI Components

### Toast Notifications

**File:** `frontend/src/lib/components/Toast.svelte`

Position: bottom-right, stacked up to 3.

```
┌──────────────────────────────────────────────────┐
│  ✓  Message sent                                 │
└──────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────┐
│  ⚠️  Connection lost. Reconnecting...            │
└──────────────────────────────────────────────────┘
```

| Type | Icon | Border-left color |
|------|------|-------------------|
| Success | ✓ check-circle | #22c55e |
| Error | ✕ x-circle | #ef4444 |
| Warning | ⚠ alert-triangle | #f59e0b |
| Info | ℹ info | #3b82f6 |

```css
.toast {
  display: flex; align-items: center; gap: 12px;
  padding: 14px 18px;
  background: #16171e;
  border-left: 3px solid <color>;
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  animation: slide-in 200ms ease-out;
}
@keyframes slide-in {
  from { opacity: 0; transform: translateX(100%); }
  to { opacity: 1; transform: translateX(0); }
}
```

Auto-dismiss: 4s for success/info, 6s for warning, stays until dismissed for error.

---

### Confirmation Modal (Generic)

Used for destructive actions (delete message, leave server, etc.):

```
┌──────────────────────────────────────────────────┐
│  Leave "Retro Gamers United"?                [✕] │
│                                                  │
│  You'll need a new invite link to rejoin         │
│  this server.                                    │
│                                                  │
│  [Cancel]                    [Leave Server]      │
└──────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Max-width | 400px |
| Padding | 24px |
| Title | Inter 700, 18px |
| Body | Inter 400, 14px, #9ca3af |
| Cancel | Secondary, left |
| Confirm | Destructive (red) or primary depending on action |
| Overlay | rgba(0,0,0,0.8), click-outside to dismiss |

---

### Context Menu

**File:** `frontend/src/lib/components/ContextMenu.svelte`

Right-click on messages, users, channels:

**Message context menu:**
```
┌──────────────────────────────┐
│  ⚡ Add Hype Moment          │
│  ✏️  Edit Message            │
│  📋 Copy Text                │
│  🔗 Copy Message Link        │
│  ─────────────────────────   │
│  🗑️  Delete Message          │
└──────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Width | 200px |
| Background | `#1c1d26` |
| Border | `1px solid rgba(255,255,255,0.1)` |
| Border radius | 10px |
| Item height | 36px |
| Item padding | 0 12px |
| Item font | Inter 400, 14px, #f9fafb |
| Item hover | rgba(124,58,237,0.1), color #a78bfa |
| Destructive item | #ef4444 |
| Divider | 1px rgba(255,255,255,0.06) |

---

### Online Presence Indicator

Used everywhere an avatar appears:

```
[avatar]
   ●      ← status dot, bottom-right of avatar
```

| Status | Color | Size |
|--------|-------|------|
| Online | #22c55e | 10px (for 36px avatar), 8px (for 28px), 12px (for 56px) |
| Away | #f59e0b | same |
| Offline | #4b5563 | same |

White 2px border around dot to separate from avatar image.

---

### Skeleton Loading States

Used during data fetch for all major views:

**Message list skeleton:**
3-4 rows, each:
- Avatar placeholder (36px circle, shimmer)
- Name bar (80px × 12px, shimmer)
- Content bars (2-3 lines, varying widths, shimmer)

**Profile skeleton:**
- Banner (full-width × 220px, shimmer)
- Avatar (112px circle, shimmer)
- Name bar (200px × 24px, shimmer)
- Bio text (3 lines, shimmer)

**Card skeleton:**
- Card background (shimmer fill)
- Inner element placeholders

```css
/* Shimmer animation */
@keyframes shimmer {
  0%   { background-position: -200% 0; }
  100% { background-position:  200% 0; }
}
.skeleton {
  background: linear-gradient(90deg,
    rgba(255,255,255,0.04) 0%,
    rgba(255,255,255,0.08) 50%,
    rgba(255,255,255,0.04) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s linear infinite;
  border-radius: 4px;
}
```

---

## Mobile (Capacitor) Specific

### Bottom Navigation Bar

Replaces the desktop sidebar on mobile (iOS/Android):

```
┌──────────────────────────────────────────────────────┐
│  [Messages]  [Explore]  [+Create]  [Notifications] [Profile] │
└──────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Height | 60px + safe area inset |
| Background | `#0f1117` |
| Border top | `1px solid rgba(255,255,255,0.06)` |
| Icon size | 24px |
| Label | Inter 500, 11px, below icon |
| Active | Icon + label: #7c3aed |
| Inactive | #6b7280 |
| Create button | Center, 48px circle, #7c3aed bg, white + icon |
| Safe area | `padding-bottom: env(safe-area-inset-bottom)` |

### Mobile-Specific Patterns

1. **Pull to refresh** — on message list, conversation list
2. **Swipe to dismiss** — toast notifications
3. **Long press** — triggers context menu (replaces right-click)
4. **Swipe right** — navigate back (iOS)
5. **Haptic feedback** — on send message, on reaction, on destructive action

---

## Accessibility Requirements

| Requirement | Implementation |
|-------------|----------------|
| Focus visible | 2px purple outline on all interactive elements (`outline: 2px solid #7c3aed; outline-offset: 2px`) |
| Color contrast | All text ≥ 4.5:1 on backgrounds |
| ARIA labels | All icon-only buttons need `aria-label` |
| Skip nav | "Skip to main content" link at page top |
| Screen reader | Announce new messages via `aria-live="polite"` |
| Reduced motion | `@media (prefers-reduced-motion)` — disable animations, replace with instant transitions |
| Keyboard nav | All actions reachable via keyboard |

```css
@media (prefers-reduced-motion: reduce) {
  *, ::before, ::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```
