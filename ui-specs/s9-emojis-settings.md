# S9 — Custom Emojis & Settings UI Spec

**Sprint:** S9 (W19–W20)
**Phases:** 13 (Custom Emojis) + 14 (User Settings)
**Primary reference:** Image 7 (Settings — Profile Customization screen)

---

## Phase 13 — Custom Emoji System

### Emoji Picker

**File:** `frontend/src/lib/components/emoji/EmojiPicker.svelte`

Triggered by the emoji button in `MessageInput`.

```
┌──────────────────────────────────────────────────────────┐
│  😊  🔍 Search emojis...                                │
│  ────────────────────────────────────────────────────    │
│  [😀][😂][❤️][👍][🔥][✨][🎉][🙏][😭][🤔] ← recent  │
│  ────────────────────────────────────────────────────    │
│  SERVER EMOJIS — Retro Gamers United (12)               │
│  [img][img][img][img][img][img][img][img]                │
│  ────────────────────────────────────────────────────    │
│  SMILEYS & EMOTION                                       │
│  [😀][😃][😄][😁][😆][😅][😂][🤣][😊][😇]             │
│  [🙂][🙃][😉][😌][😍][🥰][😘][😗][😙][😚]             │
│                                                          │
│  PEOPLE & BODY                                           │
│  ...                                                     │
└──────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Popover | max-width 360px, max-height 400px, glass card, border-radius 16px |
| Position | Above message input, left-aligned |
| Search | Full-width input at top, 32px height |
| Category tabs | Horizontal scrollable tab bar: Recent · Server Emojis · 😀 People · 🌿 Nature · 🍕 Food · ⚽ Activity · ✈️ Travel · 💡 Objects · ♾️ Symbols · 🏁 Flags |
| Tab icons | 18px icon only, tooltip on hover |
| Emoji grid | 8-column grid, 36px × 36px per cell |
| Cell hover | Background rgba(124,58,237,0.15), border-radius 6px |
| Server emojis section | Shows current server's custom emojis at top |
| Custom emoji | `<img>` tag from R2 URL, 28px × 28px |
| Hover tooltip | Emoji name below cursor |
| Skin tone picker | Swatch row shown on hover of people emojis (5 tones) |
| Keyboard nav | Arrow keys + Enter to select |

---

### EmojiUploader (Admin Only)

**File:** `frontend/src/lib/components/emoji/EmojiUploader.svelte`

Accessible from Server Settings → Emoji tab.

```
┌────────────────────────────────────────────────────────┐
│  Upload Custom Emoji                              [✕]  │
│                                                        │
│  ┌────────────────────────────────────────────────┐   │
│  │                                                │   │
│  │          [drag-and-drop area]                  │   │
│  │          Cloud upload icon 32px                │   │
│  │          "Drag PNG/GIF here or click"          │   │
│  │          Max 256KB · PNG or GIF                │   │
│  │                                                │   │
│  └────────────────────────────────────────────────┘   │
│                                                        │
│  EMOJI NAME                                            │
│  ┌────────────────────────────────────────────────┐   │
│  │  :my_emoji_name:                               │   │
│  └────────────────────────────────────────────────┘   │
│  Preview: [emoji 32px]  :my_emoji_name:                │
│                                                        │
│  [Cancel]                        [Upload Emoji →]      │
└────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Drop zone | Dashed border rgba(124,58,237,0.3), border-radius 12px, background rgba(124,58,237,0.05) |
| Active drag | Solid border #7c3aed, background rgba(124,58,237,0.1) |
| Name input | Auto-populates from filename, editable, prefix/suffix ":" shown as placeholder |
| Validation | Red: name contains spaces, too long, already exists, file too large |
| Preview | Live update as name changes |
| Upload progress | Progress bar under button |

---

### CustomEmojiManager

**File:** `frontend/src/lib/components/emoji/CustomEmojiManager.svelte`

Accessible from Server Settings → Emoji.

```
┌────────────────────────────────────────────────────────────┐
│  Custom Emojis    12 / 50                 [Upload Emoji +] │
│  ──────────────────────────────────────────────────────    │
│  [img] :retro_mario:    Uploaded by @admin · 2d ago [🗑️] │
│  [img] :pixel_heart:    Uploaded by @admin · 5d ago [🗑️] │
│  [img] :speedrun:       Uploaded by @admin · 1w ago [🗑️] │
│  ...                                                       │
│                                                            │
│  ──────────────────────────────────────────────────────    │
│  🔒 Upgrade to GoPro for 100 emoji slots                  │
└────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Count | "12 / 50" — Inter 500, 14px, #9ca3af |
| Row | 48px height, emoji 28px + name + uploader info + delete button |
| Delete | Trash icon, red on hover, shows confirmation popover |
| Premium upsell | Amber banner at limit: "Upgrade for 100 slots" |

---

## Phase 14 — Settings Page

**File:** `frontend/src/routes/(app)/settings/+page.svelte`
**Reference:** Image 7 (Profile Customization / Settings)

### Settings Layout

```
┌────────────────────────────────────────────────────────────────────────┐
│  [TOPNAV with Settings active]                                         │
├─────────────────────────┬──────────────────────────────┬───────────────┤
│  LEFT NAV (220px)       │  MAIN CONTENT               │ RIGHT SIDEBAR │
│                         │  (flex-grow, max 680px)     │ (280px)       │
│  Settings               │                             │               │
│  v2.4.0 (Stable)        │  [Section content]          │ [Contextual   │
│                         │                             │  actions]     │
│  ─ My Profile (active)  │                             │               │
│  ─ Privacy & Safety     │                             │               │
│  ─ Appearance           │                             │               │
│  ─ Voice & Video        │                             │               │
│  ─ Notifications        │                             │               │
│  ─ Yapper Premium  NEW  │                             │               │
│  ─ [Developer] (S8)     │                             │               │
└─────────────────────────┴──────────────────────────────┴───────────────┘
```

### Left Navigation

| Element | Spec |
|---------|------|
| Header | "Settings" — Inter 800, 20px, #f9fafb |
| Version | "v2.4.0 (Stable)" — Inter 400, 12px, #6b7280 |
| Nav items | 36px height, padding 0 16px, Inter 500, 14px |
| Active item | Background rgba(124,58,237,0.15), border-left 2px #7c3aed, text #f9fafb |
| Inactive | #9ca3af → hover: #f9fafb + rgba(255,255,255,0.04) bg |
| "NEW" badge | Pill, #7c3aed, 10px, "NEW" uppercase on Yapper Premium |
| Divider | 1px rgba(255,255,255,0.06), margin 8px 0 |

---

### Section: My Profile

**File:** `frontend/src/lib/components/settings/ProfileForm.svelte`

```
┌──────────────────────────────────────────────────────────────┐
│  Profile Customization                                       │
│  Customize your identity across the Yapperverse.            │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  [banner image — 200px height, purple waves]         │   │
│  │                                                      │   │
│  │  [avatar 80px]  CyberPunkUser99    [Edit Profile]    │   │
│  │                 Legendary Yapper Status · Joined 2023│   │
│  │                                                      │   │
│  │  ● [status quote — italic]                    [✏️]  │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌────────────────────────┐  ┌────────────────────────────┐ │
│  │  DISPLAY NAME          │  │  USERNAME                  │ │
│  │  ┌──────────────────┐  │  │  ┌──────────────────────┐  │ │
│  │  │ CyberPunkUser99  │  │  │  │ @  cyberpunk99  [🔒] │  │ │
│  │  └──────────────────┘  │  │  └──────────────────────┘  │ │
│  └────────────────────────┘  │  Username cannot be changed │ │
│                              └────────────────────────────┘ │
│                                                              │
│  ABOUT ME                                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Exploring the digital frontier. Always up for a    │   │
│  │  chat about tech, design, and the future.           │   │
│  └──────────────────────────────────────────────────────┘   │
│  Markdown supported                          86 / 190        │
│                                                              │
│  Profile Theme                                               │
│  [🟣][🔵][🟢][🔴][🟡][+]                                   │
│                                                              │
│  [Save Changes]                                              │
└──────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Section heading | "Profile Customization" — Inter 700, 24px |
| Subtitle | Inter 400, 14px, #9ca3af |
| Profile preview card | Glass card, border-radius 12px |
| Banner | 200px height, editable (click to upload), rounded top |
| Avatar overlay | Pencil icon on hover, click to upload |
| Edit Profile button | Secondary outlined, top-right of preview card |
| Status quote | Italic purple text, editable inline |
| Online dot | 10px #22c55e bottom-left of avatar |
| Field label | Inter 600, 12px, uppercase, #9ca3af |
| Display Name | Standard input |
| Username | Read-only state: lock icon suffix, "Username cannot be changed" note in #6b7280 |
| Username change | After 30 days: becomes editable (lock disappears) |
| About Me | Textarea, 4 rows min, markdown hint |
| Char counter | "86 / 190" right-aligned below textarea, red when near limit |
| Theme swatches | 5 pre-set 32px circles + "+" custom hex picker |
| Active swatch | White ring + checkmark overlay |
| Save button | Primary purple, full-width or right-aligned |

#### Profile Theme Colors (presets):
1. Purple `#7c3aed` (default)
2. Blue `#3b82f6`
3. Green `#22c55e`
4. Red `#ef4444`
5. Yellow `#f59e0b`
6. Custom `+` → hex color picker

---

### Right Sidebar — Account Actions

```
ACCOUNT ACTIONS
┌──────────────────────────────────┐
│  ⬇ Export Data                  │
│    Download your chat history    │
└──────────────────────────────────┘
┌──────────────────────────────────┐
│  ↗ Log Out                      │
│    Sign out of this device       │
└──────────────────────────────────┘

⚠️ DANGER ZONE
┌──────────────────────────────────┐
│  Disable Account                 │
│  Temporarily hide your profile   │
└──────────────────────────────────┘
┌──────────────────────────────────┐
│  Delete Account                  │
│  Permanently remove all data     │
└──────────────────────────────────┘

GoPro 🚀
┌──────────────────────────────────┐
│  Unlock animated avatars and     │
│  exclusive themes.               │
│  [Upgrade Now]                   │
└──────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| "ACCOUNT ACTIONS" label | Inter 600, 11px, #6b7280, uppercase |
| Action cards | Glass card, 56px height, flex row: icon + title/subtitle |
| Export Data icon | Download icon, #9ca3af |
| Log Out icon | LogOut icon, #9ca3af |
| "DANGER ZONE" label | "⚠️ DANGER ZONE" — Inter 600, 11px, #ef4444, uppercase |
| Danger cards | border-left 2px #ef4444, background rgba(239,68,68,0.05) |
| Danger text | Red |
| GoPro card | Brand gradient border, rocket icon, "Upgrade Now" purple button |

---

### Section: Privacy & Safety

```
┌──────────────────────────────────────────────────────────┐
│  Privacy & Safety                                        │
│                                                          │
│  WHO CAN MESSAGE YOU                                     │
│  ○ Everyone                                              │
│  ● Friends only                                          │
│  ○ Nobody                                                │
│                                                          │
│  WHO CAN FIND YOU IN SEARCH                              │
│  ○ Everyone                                              │
│  ● Friends only                                          │
│                                                          │
│  BLOCKED USERS                                           │
│  [View and manage blocked users →]                       │
│                                                          │
│  SAFETY NUMBERS                                          │
│  [View safety numbers for your conversations →]         │
└──────────────────────────────────────────────────────────┘
```

Radio buttons: custom styled — 16px circle, brand purple fill when selected.

---

### Section: Appearance

```
THEME
○ Dark (default)     ● Light    ○ System

FONT SIZE
[A-] ──●─────────── [A+]
      Normal

MESSAGE DENSITY
○ Comfortable (default)
● Compact

LANGUAGE
[English (US) ▾]
```

| Font size slider | Range slider, 3 positions: Small / Normal / Large |
| Compact mode | Reduces message padding, tighter list view |

---

### Section: Voice & Video

```
MICROPHONE
[Default — Built-in Mic ▾]
[Test Mic]  🎤 ████░░░░░░ (level meter)

SPEAKER
[Default — Built-in Speakers ▾]
[Test Audio]

NOISE SUPPRESSION
[Toggle ON/OFF]  AI-powered background noise filter

ECHO CANCELLATION
[Toggle ON/OFF]  Reduces feedback in open environments
```

---

### Section: Notifications

```
PUSH NOTIFICATIONS

[Toggle] Direct Messages        All messages
[Toggle] Server Messages        Mentions only
[Toggle] Friend Requests        On
[Toggle] Yapper News & Updates  Off

DO NOT DISTURB
[Toggle] Mute all notifications
         From: [10 PM ▾]  To: [7 AM ▾]
```

---

### Section: Yapper Premium (GoPro)

```
┌────────────────────────────────────────────────────────┐
│  🚀  GoPro                                             │
│  Unlock the full Yapper experience.                    │
│                                                        │
│  FREE                    GOPRO                         │
│  ─────────────────────   ──────────────────────────    │
│  ✓ Core messaging        ✓ Everything in Free          │
│  ✓ Voice Yaps            ✓ Animated avatar             │
│  ✓ Video Clips           ✓ Custom profile badge        │
│  ✓ 50 emojis/server      ✓ 100 emojis/server          │
│  ✓ Standard uploads      ✓ 50MB uploads               │
│  ✓ Community access      ✓ Priority support            │
│                                                        │
│  ┌────────────────────────────────────────────────┐   │
│  │  🚀 Upgrade to GoPro                          │   │
│  │     Coming Soon — Join wishlist for early access│   │
│  │     [Join Wishlist]                            │   │
│  └────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Free column | Default glass card |
| GoPro column | Purple gradient border, `background: rgba(124,58,237,0.05)` |
| Check marks | ✓ in #22c55e (Free), ✓ in #7c3aed (GoPro) |
| "Coming Soon" | Gray pill, no payment UI (placeholder) |
| Premium features | Grayed out with lock icon in rest of app when not premium |

---

### Banner/Avatar Upload

Used in Settings → My Profile and server settings.

```
┌──────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────┐  │
│  │                                                │  │
│  │  [current banner image]                        │  │
│  │                                   [Change] [✕]│  │
│  │                                                │  │
│  └────────────────────────────────────────────────┘  │
│  JPG, PNG, GIF · Max 10MB (50MB with GoPro)          │
└──────────────────────────────────────────────────────┘
```

Upload states:
1. Idle — click to open file picker
2. Uploading — progress overlay on image: `opacity: 0.5` + spinner + "Uploading... 45%"
3. Done — brief "✓" flash, then shows new image
4. Error — red toast notification

---

### Data Export Flow

Triggered from "Export Data" in account actions:

```
┌──────────────────────────────────────────────────────┐
│  Export Your Data                               [✕]  │
│                                                      │
│  Download a copy of your Yapper data including:      │
│                                                      │
│  ✓ Profile information                               │
│  ✓ Friend and follower list                         │
│  ✓ Server memberships                               │
│  ✓ Message metadata (no message content — E2EE)    │
│                                                      │
│  ⚠️ Message content cannot be exported because      │
│  it's end-to-end encrypted and only exists on       │
│  your device.                                       │
│                                                      │
│  [Cancel]                   [Request Export →]      │
└──────────────────────────────────────────────────────┘
```

After request: "Your export will be ready within 24 hours. We'll notify you when it's ready to download."

---

### Account Deletion Flow

```
┌──────────────────────────────────────────────────────┐
│  Delete Account                                 [✕]  │
│                                                      │
│  ⛔ This action is permanent.                        │
│                                                      │
│  Your account and all data will be permanently       │
│  deleted after 30 days. During this period you       │
│  can log in to cancel the deletion.                  │
│                                                      │
│  TYPE YOUR USERNAME TO CONFIRM                       │
│  ┌────────────────────────────────────────────────┐  │
│  │  cyberpunk99                                   │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│  [Cancel]               [Permanently Delete →]       │
└──────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Warning icon | Red circle-slash 32px |
| Confirmation input | Must match exact username, case-sensitive |
| Delete button | Red, disabled until username matches |
| Cancel | Secondary |
