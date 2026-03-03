# S8 — Discord Integration & Screen Time UI Spec

**Sprint:** S8 (W17–W18)
**Phases:** 11 (Screen Time) + 12 (Discord Integration)

---

## Discord Profile Import

### Import Flow — Onboarding Prompt (Step 2b)

Shown during onboarding OR accessible from Settings → My Profile.

```
┌────────────────────────────────────────────────────────┐
│                                                        │
│           Import from Discord                          │
│  Skip setting up your profile manually.               │
│  Import your Discord name, avatar, and bio.           │
│                                                        │
│  ┌─────────────────────────────────────────────────┐  │
│  │  [Discord logo 32px]  Connect Discord           │  │
│  │  Blurple (#5865F2) bg, white text               │  │
│  └─────────────────────────────────────────────────┘  │
│                                                        │
│  [Skip for now]                                        │
│                                                        │
└────────────────────────────────────────────────────────┘
```

---

### Import Preview Screen

After OAuth redirect returns with Discord profile:

```
┌────────────────────────────────────────────────────────┐
│  Profile Preview                                  [✕]  │
│  Confirm the details we found on Discord              │
│                                                        │
│  ┌────────────────────────────────────────────────┐   │
│  │  [avatar 80px]  CosmicVibe_99                  │   │
│  │                 @cosmic_create (from Discord)  │   │
│  │                 [avatar source: Discord CDN →  │   │
│  │                  auto-uploading to Yapper R2]  │   │
│  └────────────────────────────────────────────────┘   │
│                                                        │
│  Display Name   ┌──────────────────────────────────┐  │
│                 │ CosmicVibe_99                    │  │
│                 └──────────────────────────────────┘  │
│                                                        │
│  Username       ┌──────────────────────────────────┐  │
│                 │ cosmic_create                    │  │
│                 └──────────────────────────────────┘  │
│                 ✓ Available                            │
│                                                        │
│  Email          ┌──────────────────────────────────┐  │
│                 │ user@example.com (from Discord)  │  │
│                 └──────────────────────────────────┘  │
│                                                        │
│  ✓ Uploading avatar to secure Yapper storage...       │
│                                                        │
│  [Edit Details]              [Use This Profile →]     │
└────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Modal max-width | 480px |
| Avatar | 80px circle, preview from Discord CDN (shown while uploading to R2) |
| Upload status | Spinner + "Uploading avatar..." green text when R2 upload in progress |
| Fields | Pre-filled, editable — standard input style |
| "Use This Profile" | Primary purple, saves profile + closes modal |
| Privacy note | Small text below: "Your avatar is saved to Yapper's servers, not Discord's CDN" — 12px, #6b7280 |

---

### Settings — Discord Section

**File:** `frontend/src/lib/components/settings/DiscordImport.svelte`

Shown in Settings → My Profile:

```
┌────────────────────────────────────────────────────────┐
│  Connected Accounts                                    │
│                                                        │
│  [Discord logo]  Discord        Connected ✓  [Unlink] │
│                  @cosmic_create                        │
│                                                        │
│  [Google logo]   Google         Connect →             │
│                                                        │
│  [Apple logo]    Apple          Connect →             │
└────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card | Glass card, padding 20px |
| Provider row | 48px height, icon 24px + name + status |
| Connected | "Connected ✓" in #22c55e + "Unlink" text link |
| Not connected | "Connect →" link in #7c3aed |
| Unlink confirmation | Inline: "Are you sure? This will remove your Discord link." + [Confirm] [Cancel] |

---

## Discord Bot Migration Tool

**File:** `frontend/src/lib/components/settings/BotMigrationTool.svelte`

Located in Settings → Yapper for Developers tab.

### Developer Portal Tab

```
┌────────────────────────────────────────────────────────────────┐
│  Settings                                                      │
│  [My Profile] [Privacy] [Appearance] [Voice] [Notifs]         │
│  [Yapper Premium] [Yapper for Developers]  ← new tab          │
└────────────────────────────────────────────────────────────────┘
```

### Bot Import Step 1 — Enter Token

```
┌──────────────────────────────────────────────────────────┐
│  Import Your Discord Bot                                 │
│                                                          │
│  Migrate your Discord bot to Yapper with your existing  │
│  commands and configuration.                             │
│                                                          │
│  DISCORD BOT TOKEN                                       │
│  ┌──────────────────────────────────────────────────┐   │
│  │ Paste your bot token here...                    │   │
│  └──────────────────────────────────────────────────┘   │
│  ⚠️ This token is used once and never stored.           │
│                                                          │
│  [Import Bot →]                                          │
└──────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Warning | Amber pill: "Token used once, never stored" |
| Input type | `type="password"` to mask token |

### Bot Import Step 2 — Success

```
┌──────────────────────────────────────────────────────────┐
│  ✅  Bot Imported Successfully!                          │
│                                                          │
│  [bot avatar 56px]  DiscordBot#1234                     │
│                     Created as Yapper Bot Account        │
│                                                          │
│  YOUR YAPPER BOT TOKEN (shown once)                     │
│  ┌──────────────────────────────────────────────────┐   │
│  │ ym_bot_xxxxxxxxxxxxxxxxxxxxxxxx         [Copy]  │   │
│  └──────────────────────────────────────────────────┘   │
│  ⚠️ Save this token now. It won't be shown again.       │
│                                                          │
│  ─────────────────────────────────────────────────────  │
│  MIGRATION GUIDE                                         │
│                                                          │
│  Discord                    →  Yapper                   │
│  client.on('messageCreate') →  ws.on('message')        │
│  client.channels.send()     →  POST /api/v1/channels/.. │
│  Discord.js Client          →  yapper-bot-sdk (future) │
│                                                          │
│  [View Full API Docs]                                    │
└──────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Success icon | ✅ or large checkmark in #22c55e |
| Token display | Monospace font, glass card, "Copy" button |
| Warning | Amber pill: critical "save now" message |
| Migration table | Code-style table, monospace 13px |
| Background | Red-orange left border on token card |

---

## Screen Time Dashboard (Parental View)

**File:** `frontend/src/lib/components/parental/ScreenTimeDashboard.svelte`

Shown when parent clicks "Screen Time" in parental sidebar.

### Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  Alex's Screen Time                                              │
│  Today · Week · Month                    [Export Report]         │
│                                                                  │
│  TODAY'S SUMMARY                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  2h 34m total                                            │   │
│  │  ████████████████████░░░░░░░░   Daily limit: 3h          │   │
│  │                                    ⚠️  26m remaining     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  PER APP BREAKDOWN                                               │
│  ┌────────────────────────────────────────────────────────┐     │
│  │  [Y] Yapper          1h 12m  ████████████████░░░░░     │     │
│  │  [T] TikTok          45m     ████████░░░░░░░░░░░░░     │     │
│  │  [I] Instagram       22m     ████░░░░░░░░░░░░░░░░░     │     │
│  │  [Y] YouTube         15m     ██░░░░░░░░░░░░░░░░░░░     │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                  │
│  WEEKLY CHART                                                    │
│  ┌────────────────────────────────────────────────────────┐     │
│  │  [Bar chart: Mon–Sun, bars colored by Yapper vs other] │     │
│  │  Mon  Tue  Wed  Thu  Fri  Sat  Sun                      │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                  │
│  DAILY LIMIT SETTINGS                                            │
│  Set daily screen time limit for Alex                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Total daily limit:   [2h ─── ● ──── 4h]   3h           │   │
│  │  Bedtime block:       10 PM → 7 AM    [Edit]            │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Period tabs | "Today · Week · Month" — Inter 600, 14px, tab bar |
| Total bar | Thick progress bar (12px height), brand purple fill, max = daily limit |
| Warning threshold | At 80%: orange, "26m remaining" |
| Exceeded | Red, "Daily limit reached" |
| Per-app rows | App icon 24px + name + time + thin bar |
| Bar colors | Yapper = #7c3aed, other apps = #4b5563 |
| Weekly chart | Simple bar chart, Canvas or SVG, branded colors |
| Slider | Horizontal range slider, purple thumb, min 30m, max 8h |
| Bedtime block | Time range display, clickable to edit |

### Screen Time Permission Request (Child's App)

Shown on mobile when screen time tracking is not yet authorized:

```
┌────────────────────────────────────────────────────────┐
│              ⏱️  Screen Time Monitoring                │
│                                                        │
│  Your parent has enabled screen time tracking.        │
│  Allow Yapper to report your device usage.            │
│                                                        │
│  [Not Now]              [Allow Screen Time →]         │
└────────────────────────────────────────────────────────┘
```

Clicking "Allow" → opens iOS Screen Time Settings OR Android Usage Stats Settings (native OS prompt).

---

## Safety Number Verification

**File:** `frontend/src/lib/components/chat/SafetyNumber.svelte`

Accessible from DM chat: "..." menu → "View Safety Number"

```
┌────────────────────────────────────────────────────────┐
│  End-to-End Encryption                            [✕]  │
│                                                        │
│  Security code for you and neo_kai                     │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │   24831  52194  08743  19283  74921  30475        │  │
│  │   93841  27503  84720  38471  92847  01983        │  │
│  └──────────────────────────────────────────────────┘  │
│                                                        │
│  ┌────────────────────────────────────────────────┐   │
│  │                [QR Code]                       │   │
│  │                200px × 200px                   │   │
│  └────────────────────────────────────────────────┘   │
│                                                        │
│  If the numbers above match on both devices,           │
│  this conversation is fully secure.                    │
│                                                        │
│  [Mark as Verified]                                    │
└────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Number grid | Inter 700 monospace, 18px, #f9fafb, spaced in groups of 5 |
| QR code | Rendered from safety number, white on dark bg |
| Changed warning | Red banner: "⚠️ Security info changed. Verify this conversation." |
| Verified checkmark | "✓ Verified" badge in #22c55e, shown after verification |
