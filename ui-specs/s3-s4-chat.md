# S3 & S4 — DM Chat + Server/Channel UI Spec

**Sprint S3:** Signal Protocol & E2EE Core (1:1 DMs)
**Sprint S4:** Servers, Channels & Group E2EE
**Primary reference:** Image 2 (Chat screen with Live Canvas)

---

## App Shell Layout (Authenticated)

**File:** `frontend/src/routes/(app)/+layout.svelte`

```
┌──────────────────────────────────────────────────────────────────────┐
│  TOPNAV (56px, sticky)                                               │
├────────────────────────┬─────────────────────────────────────────────┤
│                        │                                             │
│  LEFT SIDEBAR          │        MAIN CONTENT                        │
│  (240px fixed)         │                                             │
│                        │                                             │
│  ─ DMs                 │  (channel chat / DM / explore / profile)   │
│  ─ Servers             │                                             │
│  ─ Explore             │                                             │
│                        │                                             │
└────────────────────────┴─────────────────────────────────────────────┘
```

---

## Top Navigation

**File:** `frontend/src/lib/components/chat/TopNav.svelte`

```
┌──────────────────────────────────────────────────────────────────────┐
│ 🔵 Yapper  [🔍 Search channels...]  Channels  Direct  Explore  🔔 ⚙️ 👤│
└──────────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Height | 56px |
| Background | `#0f1117` |
| Border bottom | `1px solid rgba(255,255,255,0.06)` |
| Logo | Sphere icon 28px + "Yapper" wordmark, Inter 700 |
| Search bar | Rounded input, 200px wide, placeholder "Search channels...", Ctrl+K shortcut hint |
| Nav links | Inter 500, 14px, #9ca3af → active: #f9fafb |
| Notification bell | 20px icon, shows red badge dot if unread |
| Settings gear | 20px icon |
| Avatar | 32px circle, online status dot (8px) bottom-right |
| Active nav | Bottom border 2px #7c3aed, text #f9fafb |

---

## Server Sidebar

**File:** `frontend/src/lib/components/chat/ServerSidebar.svelte`

```
┌────────────────────────┐
│  # general-yapping     │  ← active channel
│  # announcements       │
│  # off-topic           │
│                        │
│  ▸ VOICE CHANNELS      │
│  🔊 Lounge             │
│  🔊 Gaming             │
│                        │
│  SERVER MEMBERS (1.2k) │
│  ● neo_kai             │
│  ● pixel_queen         │
│  ○ cyber_dave (away)   │
└────────────────────────┘
```

| Element | Spec |
|---------|------|
| Width | 240px |
| Background | `#0f1117` |
| Border right | `1px solid rgba(255,255,255,0.06)` |
| Server name header | 56px height, server name Inter 700, 15px, #f9fafb, padding 0 16px |
| Section label | "TEXT CHANNELS" / "VOICE CHANNELS" — Inter 600, 11px, #6b7280, letter-spacing 0.08em, uppercase, padding 16px 16px 6px |
| Channel row | 32px height, padding 0 12px, border-radius 6px, display flex, gap 8px |
| Channel row hover | background: rgba(255,255,255,0.05) |
| Channel row active | background: rgba(124,58,237,0.15), border-left 2px #7c3aed, text #f9fafb |
| Channel icon | `#` for text, `🔊` for voice, 14px, #6b7280 |
| Channel name | Inter 400, 14px, #9ca3af → active: #f9fafb |
| Unread indicator | Dot 6px right side, or bold channel name |
| Member row | 28px height, avatar 20px + username 13px, online dot |
| Online dot | 8px circle, position: relative to avatar |

---

## Channel Chat Page — Main Area

**File:** `frontend/src/routes/(app)/servers/[id]/channels/[channelId]/+page.svelte`

### Full Layout (references Image 2)

```
┌───────────────────────────────────────────────────────┬────────────┐
│  CHANNEL HEADER (48px)                                │            │
│  # general-yapping  [1.2k online]  [avatars +42] [🔍] │ LIVE       │
├───────────────────────────────────────────────────────┤ CANVAS     │
│                                                        │ (360px)    │
│  MESSAGE LIST                                          │            │
│  (scrollable, flex-col-reverse)                        │            │
│                                                        │            │
│  ┌──────────────────────────────────────────────────┐  │            │
│  │ [avatar] neo_kai  10:42 AM                       │  │            │
│  │ Did anyone see the new drop? 🔥                  │  │            │
│  └──────────────────────────────────────────────────┘  │            │
│                                                        │            │
│  ┌──────────────────────────────────────────────────┐  │            │
│  │ [avatar] pixel_queen  10:43 AM                   │  │            │
│  │ Checking it now. The color palette is a vibe.    │  │            │
│  │ ┌──────────────────────────────────┐              │  │            │
│  │ │  [image embed]                   │              │  │            │
│  │ └──────────────────────────────────┘              │  │            │
│  └──────────────────────────────────────────────────┘  │            │
│                                                        │            │
│  ┌──────────────────────────────────────────────────┐  │            │
│  │ [avatar] You  10:46 AM                           │  │            │
│  │ ┌──────────────────────────────┐                 │  │            │
│  │ │ ▶  ════════════  0:14 • YAP │                 │  │            │
│  │ └──────────────────────────────┘                 │  │            │
│  └──────────────────────────────────────────────────┘  │            │
│                                                        │            │
│  neo_kai is typing...                                  │            │
├───────────────────────────────────────────────────────┤            │
│  MESSAGE INPUT BAR (64px)                             │            │
│  [+] [Type a message...]       [🎤 Yap] [🎥 Clip] [→]│            │
└───────────────────────────────────────────────────────┴────────────┘
```

---

## Channel Header

**File:** `frontend/src/lib/components/chat/ChannelHeader.svelte`

| Element | Spec |
|---------|------|
| Height | 48px |
| Background | `#0f1117` |
| Border bottom | `1px solid rgba(255,255,255,0.06)` |
| Channel name | Inter 700, 16px, #f9fafb, prefix "#" in #6b7280 |
| Online badge | Glass pill: "1.2k online", 12px, #9ca3af, `●` dot in #22c55e |
| Member avatars | 3 overlapping 24px circles + "+42" text, stacked right-aligned, gap -8px |
| Search icon | 20px, right-side |

---

## MessageList

**File:** `frontend/src/lib/components/chat/MessageList.svelte`

### Message row spec:

```
┌──────────────────────────────────────────────────────────────┐
│ [avatar 36px]  username  10:42 AM                            │
│                message text content here...                   │
│                [image/media embed if applicable]              │
└──────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Avatar | 36px circle, margin-right 12px |
| Username | Inter 700, 14px, #f9fafb |
| Timestamp | Inter 400, 11px, #6b7280, margin-left 6px |
| Body text | Inter 400, 15px, #d1d5db, line-height 1.5 |
| Hover | Shows timestamp in right area, background rgba(255,255,255,0.02) |
| Grouped messages | Hide avatar/name for messages within 2 min of same sender; show only body, left-aligned with 48px indent |
| Date divider | Center: "Today", "Yesterday", "March 1" — Inter 400, 13px, #6b7280, hr lines on both sides |
| Scroll | Reverse flex, auto-scroll to bottom on new message |
| E2EE lock icon | Tiny 🔒 12px icon in message footer for confirmation, #4b5563 |

### Image Embed:
```css
.message-image {
  max-width: 400px;
  max-height: 300px;
  border-radius: 8px;
  margin-top: 8px;
  object-fit: cover;
  cursor: pointer; /* opens lightbox */
}
```

### System Messages:
```
─────── neo_kai joined the server ───────
─────── Alice created channel #general ───────
```
Centered, Inter 400, 13px, #6b7280, dashes on both sides.

---

## YapMessage Bubble (Audio)

**File:** `frontend/src/lib/components/chat/YapMessage.svelte`

```
┌──────────────────────────────────────┐
│  ▶   ═══════════════════   0:14     │
│      [waveform visualization]        │
│                          • YAP       │
└──────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Container | min-width 220px, max-width 320px, height 56px, background: `#1e3a5f` (dark blue), border-radius 28px, padding 0 16px |
| Play button | 32px circle, #7c3aed bg, white play icon 16px |
| Waveform | 80px wide SVG waveform visualization (static or animating during playback), color #60a5fa |
| Duration | Inter 500, 12px, #9ca3af, right-aligned |
| "YAP" badge | Dot + "YAP" text, 10px, #60a5fa, far right |
| Playing state | Play button → Pause icon; waveform animates |
| Decrypt state | Spinner shown while decrypting (< 200ms typical) |

---

## Message Input Bar

**File:** `frontend/src/lib/components/chat/MessageInput.svelte`

```
┌─────────────────────────────────────────────────────────────────────┐
│  [+]  [Type a message...]                     [🎤 Yap]  [📹 Clip]  [→]│
└─────────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Container height | 64px, padding 8px 16px |
| Background | `#0f1117`, border-top `1px solid rgba(255,255,255,0.06)` |
| Attach button (+) | 32px circle, rgba(255,255,255,0.1), plus icon #9ca3af |
| Text input | Flex-grow, background transparent, no border, Inter 400 15px, #f9fafb |
| Text input focus | No outline (parent div handles focus ring) |
| Input wrapper | `background: rgba(255,255,255,0.05); border-radius: 24px; border: 1px solid rgba(255,255,255,0.08)` |
| Emoji button | 😊 icon 20px, right-side of input |
| Yap button | Mic icon 16px + "Yap" label, pill button, background rgba(124,58,237,0.15), border 1px rgba(124,58,237,0.3), color #a78bfa |
| Clip button | Video icon 16px + "Clip" label, same style as Yap |
| Send button | 36px circle, #7c3aed, arrow-right icon 18px, disabled when empty |
| Typing emit | On keydown, send `typing_start` WS event (throttled 2s) |

---

## TypingIndicator

**File:** `frontend/src/lib/components/chat/TypingIndicator.svelte`

```
neo_kai is typing...   [● ● ●]
```

| Element | Spec |
|---------|------|
| Container | 24px height, padding 0 16px, absolute bottom of message list |
| Text | Inter 400, 13px, #9ca3af, italic |
| Dots | 3 dots, 6px each, #9ca3af, animate: stagger scale 1.0 → 1.4 → 1.0, 400ms, 133ms offset each |
| Multi-user | "neo_kai and pixel_queen are typing..." |
| Many | "Several people are typing..." |
| Fade in/out | `opacity: 0 → 1` over 150ms; auto-hides after 5s with no new event |

---

## DM Conversation Page

**File:** `frontend/src/routes/(app)/dm/[conversationId]/+page.svelte`

Same layout as channel chat but:
- No sidebar (replaced by DM conversation list)
- Header shows recipient name + online status
- "End-to-end encrypted · Safety number" footer link in empty state
- Message density identical

### DM Conversation List (sidebar replacement for DMs):
```
┌────────────────────────┐
│  Direct Messages       │
│  [+ New DM]            │
│                        │
│  ● neo_kai             │  ← active
│    "checking it now..."│
│    10:43 AM            │
│                        │
│  ○ pixel_queen (away)  │
│    "what time is..."   │
│    Yesterday           │
│                        │
│  ● cyber_dave          │
│    "hop on voice?"     │
│    Monday              │
└────────────────────────┘
```

| Element | Spec |
|---------|------|
| Header | "Direct Messages" Inter 700, 15px + "+" button right |
| Conversation row | 64px height, avatar 36px + name + last message preview + timestamp |
| Preview text | Inter 400, 13px, #9ca3af, truncated 1 line |
| Timestamp | Inter 400, 11px, #6b7280, right-aligned |
| Unread | Bold name, dot indicator, unread count badge |
| Active | Background rgba(124,58,237,0.12), border-left 2px #7c3aed |

---

## Server Creation Modal

**File:** `frontend/src/lib/components/chat/CreateServerModal.svelte`

```
┌──────────────────────────────────────────┐
│  Create a Server                      [✕] │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │  SERVER ICON                       │  │
│  │  [circle upload area, 80px]        │  │
│  │  Click to upload (optional)        │  │
│  └────────────────────────────────────┘  │
│                                          │
│  Server Name                             │
│  ┌────────────────────────────────────┐  │
│  │ My Awesome Server                  │  │
│  └────────────────────────────────────┘  │
│                                          │
│  Description (optional)                  │
│  ┌────────────────────────────────────┐  │
│  │                                    │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ☐ Make this server public               │
│     Anyone can find and join             │
│                                          │
│  [Cancel]          [Create Server →]     │
└──────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Modal | Max-width 440px, glass card, padding 32px |
| Icon upload | 80px circle, dashed border rgba(124,58,237,0.3), camera icon center, drag-and-drop |
| Toggle | Checkbox → custom styled toggle or checkbox |
| Cancel | Secondary button |
| Create | Primary button |
| Overlay | Semi-transparent `rgba(0,0,0,0.8)` backdrop |

---

## Invite Link Modal

**File:** `frontend/src/lib/components/chat/InviteModal.svelte`

```
┌──────────────────────────────────────────┐
│  Invite People to #general-yapping    [✕] │
│                                          │
│  Share this link                         │
│  ┌────────────────────────────────────┐  │
│  │ yapperhq.com/join/abc123x  [Copy] │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ○ Never expire   ● Expire in: [24h ▾]  │
│  ○ Unlimited uses ● Max uses:  [10 ___]  │
│                                          │
│  [Generate New Link]                     │
└──────────────────────────────────────────┘
```

---

## Empty States

### No servers joined:
```
┌──────────────────────────────────────────────────────┐
│                                                      │
│          🌐  Welcome to Yapper!                       │
│                                                      │
│    You haven't joined any servers yet.               │
│                                                      │
│    [Explore Communities →]   [Create Server]         │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### Empty DM thread:
```
🔒 This conversation is end-to-end encrypted.
   Messages can only be read by you and neo_kai.
   [View Safety Number]
```
Centered, #6b7280, 14px, lock icon 20px #7c3aed above.

### Loading state:
Message list skeleton: 3-4 rows of shimmer placeholders:
```css
.skeleton {
  background: linear-gradient(90deg,
    rgba(255,255,255,0.05) 0%,
    rgba(255,255,255,0.1) 50%,
    rgba(255,255,255,0.05) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
  border-radius: 4px;
}
```

---

## Keyboard Shortcuts (Channel Chat)

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line |
| `↑` (empty input) | Edit last message |
| `Escape` | Cancel edit / close modal |
| `Ctrl+K` | Open search |
| `Ctrl+/` | Show keyboard shortcuts |
