# S7 — User Profiles & Parental Controls UI Spec

**Sprint:** S7 (W15–W16)
**Screens:** Profile (Image 3), Parental Dashboard (Image 4), Child Setup Steps 1-3 (Images 5 & 6)

---

## Screen 3 — User Profile Page

**File:** `frontend/src/routes/(app)/profile/[username]/+page.svelte`
**Reference:** Image 3 (CosmicVibe_99 profile)

### Full Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  [TOPNAV — Feed | Communities | Yaps | Create Yap | ⚙ | 👤]     │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                BANNER IMAGE (full-width, 220px tall)        │ │
│  │                [dark gradient / user-set image]             │ │
│  │                                         [✏️] [share]       │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  [avatar 112px]  CosmicVibe_99           [Follow] [Message] [⋯] │
│  ⚡ 9.2k         @cosmic_create  · 📍 Neo-Tokyo                  │
│                                                                  │
├─────────────────┬────────────────────────────────────────────────┤
│  LEFT SIDEBAR   │                                                │
│  (300px)        │  HYPE MOMENTS                      [⊞] [☰]   │
│                 │  ┌────────────────────┐  ┌──────────────────┐ │
│  BIO            │  │ [image moment]     │  │ [text quote      │ │
│  "Digital soul  │  │                    │  │  moment]         │ │
│  wandering      │  │ Neon Nights in     │  │                  │ │
│  through the    │  │ District 9         │  │ "Design is not   │ │
│  ethernet..."   │  │ 💬42  ↺12  2h ago │  │  just what it    │ │
│                 │  └────────────────────┘  │  looks like..."  │ │
│  #Design        │  ┌──────────────────────┐│ └──────────────────┘│
│  #Cyberpunk     │  │ [video clip moment]  ││ ┌──────────────────┐ │
│  #Photography   │  │                      ││ │ [image moment]   │ │
│                 │  │  ▶  New Setup Tour   ││ │                  │ │
│  12.5k Followers│  │     💬108  👁5.2k   ││ │ Sunday Morning   │ │
│  482 Following  │  └──────────────────────┘│ │ Lo-Fi            │ │
│  89 Yaps        │                          │ └──────────────────┘ │
│                 │                          │                      │
│  TOP COMMUNITIES│                          │                      │
│  Cyber Sec Hub  │                          │                      │
│  UI/UX Designers│                          │                      │
│  Analog Photo   │                          │                      │
│                 │                          │                      │
│  MUTUAL CONNECT │                          │                      │
│  [avatar][avtr] │                          │                      │
│  +12            │                          │                      │
│  You both follow│                          │                      │
│  TechTrends     │                          │                      │
└─────────────────┴────────────────────────────────────────────────┘
```

---

### Banner + Avatar Header

| Element | Spec |
|---------|------|
| Banner height | 220px on desktop, 160px on mobile |
| Banner | Background: user-uploaded image OR dark gradient fallback `linear-gradient(135deg, #1a1a2e, #0a0a0f)` |
| Edit icons | Top-right: pencil + share — only shown on own profile |
| Avatar size | 112px circle, 4px white/purple border, position: absolute bottom -56px left 24px |
| Avatar border | `border: 3px solid #7c3aed` |
| Follower count badge | Below avatar: ⚡ icon + "9.2k" — glass pill, Inter 700, 14px |
| Name | Inter 800, 28px, #f9fafb, margin-top: 72px (to clear avatar overlap) |
| Handle + location | "@handle · 📍 Location" — Inter 400, 14px, #9ca3af |
| Follow button | Primary purple pill, 40px height, "Follow" / "Following ✓" |
| Message button | Secondary outlined pill, "Message" |
| More button | "⋯" icon button, shows dropdown: Block, Report, Copy Profile Link |
| Own profile | Replace Follow/Message with "Edit Profile" button |

---

### BIO Card (Left Sidebar)

```
┌──────────────────────────────┐
│  BIO                         │
│                              │
│  Digital soul wandering      │
│  through the ethernet.       │
│  Obsessed with cyber-        │
│  aesthetics, retro-gaming,   │
│  and lofi beats.             │
│                              │
│  [#Design] [#Cyberpunk]      │
│  [#Photography]              │
│                              │
│  ────────────────────────    │
│  12.5k     482      89       │
│  Followers Following Yaps    │
└──────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card | Glass card, border-radius 12px, padding 20px |
| "BIO" label | Inter 600, 11px, #6b7280, uppercase, letter-spacing 0.08em |
| Bio text | Inter 400, 15px, #d1d5db, line-height 1.6 |
| Interest tags | Small chips, Inter 500, 13px, rgba(255,255,255,0.08) bg |
| Divider | 1px rgba(255,255,255,0.06) |
| Stats row | 3 columns: value Inter 700, 18px, #f9fafb; label Inter 400, 12px, #9ca3af |

---

### Top Communities Card

```
┌──────────────────────────────┐
│  Top Communities  [View All] │
│                              │
│  [icon] Cyber Security Hub > │
│         12k members · 45 online│
│                              │
│  [icon] UI/UX Designers    > │
│         8.5k members · 120 online│
│                              │
│  [icon] Analog Photography > │
│         5k members · 12 online│
└──────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Server icon | 36px circle, colored per server |
| Name | Inter 600, 14px, #f9fafb |
| Metadata | Inter 400, 12px, #9ca3af |
| Arrow | `›` right-aligned, #6b7280 |
| Row hover | Background rgba(255,255,255,0.04) |

---

### Mutual Connections Card

```
┌──────────────────────────────┐
│  Mutual Connections          │
│                              │
│  [av][av][av] +12            │
│                              │
│  You both follow TechTrends  │
│  and FutureBass.             │
└──────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Avatars | 28px circles, stacked with -10px gap, max 3 shown + count |
| Text | Inter 400, 13px, #9ca3af — mutual servers/followers highlighted in #a78bfa |

---

### Hype Moments Grid (Main Column)

**File:** `frontend/src/lib/components/profile/HypeMoments.svelte`

Masonry grid layout — 2 columns, variable card heights.

**Card types:**

**1. Media/Image Moment:**
```
┌────────────────────────────────────────┐
│  [full-bleed image]                    │
│  ⚡ 2.4k                                │
│                                        │
│  Neon Nights in District 9             │
│  "Just explored the new market         │
│   sector. The vibes are immaculate."   │
│  💬 42   ↺ 12   2h ago                 │
└────────────────────────────────────────┘
```

**2. Text/Quote Moment:**
```
┌────────────────────────────────────────┐
│  ❝  "Design is not just what it looks │
│      like and feels like. Design is   │
│      how it works."                   │
│                                        │
│  [avatar] Shared via DesignDaily       │
│                          ♥            │
└────────────────────────────────────────┘
```
Background: bright gradient (randomized per quote card — blue, teal, purple).

**3. Video/Clip Moment:**
```
┌────────────────────────────────────────┐
│  [video thumbnail with play button]    │
│                                        │
│  New Setup Tour 2024                   │
│  💬 108   👁 5.2k   1d ago             │
└────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card border-radius | 16px |
| Grid gap | 12px |
| Image aspect ratio | Auto (fills card), max-height 300px |
| Overlay gradient | Bottom fade on media cards |
| Caption | Inter 600, 14px, #f9fafb |
| Description | Inter 400, 13px, #9ca3af, 2 lines |
| Engagement | 💬 / ↺ / 👁 + count, Inter 400, 12px, #9ca3af |
| New badge | "New" pill on recently pinned moments |
| Load more | "Show all X Hype Moments" link at bottom |

---

## Screen 4 — Parental Dashboard

**File:** `frontend/src/routes/parent/dashboard/+page.svelte`
**Reference:** Image 4 (Alex's Safety Dashboard)

### Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  [sphere] Yapper  [🔍 Search activity...]  Safety Center  Family   │
│  Setup  Settings                          Sarah Jenkins  Admin [👤] │
├───────────────────┬──────────────────────────────────────────────────┤
│                   │                                                  │
│  MANAGED ACCOUNTS │  Alex's Safety Dashboard              [Export]  │
│                   │  Monitoring active · Daily report updated 5m ago │
│  ●Alex   (Online) │  [Adjust Filters]                               │
│  ○Mia (2h ago)    │                                                  │
│                   │  ⚠️ Pending Alerts           ○ 2 New            │
│  [+ Add Child]    │  ┌──────────────────────┐ ┌──────────────────┐  │
│                   │  │ 👤 New DM Request    │ │ 👥 Community     │  │
│                   │  │ From @gamer_dude99   │ │ Join Request     │  │
│  CONTROLS         │  │ Stranger · Not in    │ │ To join "Late    │  │
│  ●Safety Overview │  │ mutuals             │ │ Night Valorant"  │  │
│  ○Screen Time     │  │ [Review] [Dismiss]   │ │ [Review][Dismiss]│  │
│  ○Content Filters │  └──────────────────────┘ └──────────────────┘  │
│  ○Alert Settings  │                                                  │
│                   │  ┌──────────────────────┐  ┌──────────────────┐ │
│                   │  │  Safety Feed         │  │ Activity Snapshot│ │
│                   │  │                      │  │                  │ │
│                   │  │  ○ Content Warning   │  │ TOP COMMUNITIES  │ │
│                   │  │  10m ago             │  │ School Friends   │ │
│                   │  │  Message flagged...  │  │ 2h 15m today     │ │
│                   │  │                      │  │ Minecraft Server │ │
│                   │  │  ○ New Friend        │  │ 1h 40m today     │ │
│                   │  │  45m ago             │  │                  │ │
│                   │  │  Alex added Sarah M. │  │ MOST INTERACTED  │ │
│                   │  │                      │  │ [avtr][avtr] +4  │ │
│                   │  │  ○ Screen Time       │  │ [View Friends]   │ │
│                   │  │  1h ago              │  │                  │ │
│                   │  │  2h limit reached    │  └──────────────────┘ │
│                   │  │                      │                       │
│                   │  │  View all →          │                       │
│                   │  └──────────────────────┘                       │
└───────────────────┴──────────────────────────────────────────────────┘
```

---

### Parental Top Navigation

| Element | Spec |
|---------|------|
| Background | `#0f1117` |
| Logo | Same as main app |
| Center links | "Safety Center" · "Family Setup" · "Settings" — Inter 500, 14px |
| Right | Parent name + "Admin" badge (blue) + avatar |
| Different from main nav | No "Channels/Direct/Explore" — parental context only |

---

### Left Sidebar — Managed Accounts

| Element | Spec |
|---------|------|
| Section label | "MANAGED ACCOUNTS" — Inter 600, 11px, #6b7280, uppercase |
| Child row | 48px height, avatar 36px + name + status |
| Online status | Green dot + "Online now" in #22c55e |
| Inactive | Gray dot + "Last active 2h ago" in #9ca3af |
| Active child | Background rgba(124,58,237,0.12), border-left 2px #7c3aed |
| Add Child | Dashed border button: `+` icon + "Add Child" — 40px height |
| CONTROLS section | Separate section below with nav items |
| Control item | 36px row, icon + label, #9ca3af → active: #f9fafb with purple bg |
| Active control | "Safety Overview" — background rgba(124,58,237,0.15), border-left 2px #7c3aed |

---

### Main Content — Safety Dashboard

**File:** `frontend/src/lib/components/parental/SafetyDashboard.svelte`

| Element | Spec |
|---------|------|
| Page title | "{Child}'s Safety Dashboard" — Inter 800, 28px |
| Subtitle | "Monitoring active · Daily report updated 5m ago" — #9ca3af, 14px |
| Export Report button | Secondary outlined, download icon |
| Adjust Filters | Secondary outlined, settings icon |

---

### PendingAlerts

**File:** `frontend/src/lib/components/parental/PendingAlerts.svelte`

```
⚠️ Pending Alerts   [2 New badge]
```

Alert card spec:
```
┌─────────────────────────────────────────────┐
│  [👤 icon 40px]  New DM Request             │
│                  From @gamer_dude99          │
│                  Stranger · Not in mutuals  │
│                                             │
│                  [Review]  [Dismiss]        │
└─────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card | Glass card, border-left 3px #f59e0b (pending), padding 16px 20px |
| Icon | 40px circle, rgba(245,158,11,0.15) bg, icon in #f59e0b |
| Title | Inter 700, 14px, #f9fafb |
| Metadata | Inter 400, 13px, #9ca3af |
| Review button | Primary blue `#3b82f6`, 32px height |
| Dismiss | Text button, #6b7280 |
| "2 New" badge | #ef4444 pill |
| Alert types: DM Request | Person icon, amber |
| Alert types: Server join | Users icon, amber |
| Alert types: Friend request | User-plus icon, amber |
| Approved state | Green border, checkmark, "Approved" |
| Declined state | Red border, x-mark, "Declined" |

---

### SafetyFeed (Timeline)

**File:** `frontend/src/lib/components/parental/SafetyFeed.svelte`

```
○ [Content Warning]  10 mins ago
  Message flagged in "School Friends" group
  for potential bullying keywords.

○ [New Friend]  45 mins ago
  [avatar] Alex became friends with Sarah M.
  (Mutual friends: 12)

○ [Screen Time]  1 hour ago
  Daily limit of 2 hours reached on
  TikTok Integration.
```

| Element | Spec |
|---------|------|
| Timeline line | 2px vertical line, rgba(255,255,255,0.1) |
| Timeline dot | 10px circle: Content Warning = amber, New Friend = blue, Screen Time = orange |
| Event card | Background rgba(255,255,255,0.03), border-radius 8px, padding 12px 16px |
| Event type badge | Colored pill chip (Content Warning = amber, New Friend = blue) |
| Timestamp | Right-aligned, 11px, #6b7280 |
| Body text | 14px, #d1d5db |
| View all | "View all activity →" link, #7c3aed |

---

### ActivitySnapshot

**File:** `frontend/src/lib/components/parental/ActivitySnapshot.svelte`

```
┌──────────────────────────────────┐
│  Activity Snapshot               │
│                                  │
│  TOP COMMUNITIES                 │
│  [SF] School Friends             │
│       2h 15m today  ████────     │
│  [MC] Minecraft Server           │
│       1h 40m today  ███─────     │
│                                  │
│  MOST INTERACTED                 │
│  [av][av][av] +4                 │
│  [View Friends List]             │
└──────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card | Glass card, padding 16px |
| Section labels | "TOP COMMUNITIES" / "MOST INTERACTED" — 11px caps, #6b7280 |
| Server icon | 32px colored square with initials |
| Time bar | Thin progress bar, 60px wide, brand purple fill |
| Friends avatars | Stacked 28px circles |
| View Friends button | Outlined secondary, full-width |

---

## Screen 5 — Child Account Setup (Step 1 of 3)

**File:** `frontend/src/routes/parent/children/setup/+page.svelte`
**Reference:** Image 5 (Secure Their Space)

### Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  [sphere] Yapper                              🔔  ⚙️  👤         │
│                                                                  │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░         │
│  ░░░░  PURPLE GRADIENT BACKGROUND  ░░░░░░░░░░░░░░░░░░░░         │
│  ░░░░                                                  ░░░       │
│  ░░░░  ┌────────────────────────────────────────────┐  ░░░       │
│  ░░░░  │           🛡️                               │  ░░░       │
│  ░░░░  │     Secure Their Space                     │  ░░░       │
│  ░░░░  │  Create a safe identity for your child     │  ░░░       │
│  ░░░░  │  to start exploring Yapper.                │  ░░░       │
│  ░░░░  │                                            │  ░░░       │
│  ░░░░  │  CHILD'S DISPLAY NAME                      │  ░░░       │
│  ░░░░  │  ┌──────────────────────────────────────┐  │  ░░░       │
│  ░░░░  │  │ [👤] e.g. SpaceExplorer24            │  │  ░░░       │
│  ░░░░  │  └──────────────────────────────────────┘  │  ░░░       │
│  ░░░░  │                                            │  ░░░       │
│  ░░░░  │  SELECT A VIBE          [See more]         │  ░░░       │
│  ░░░░  │  ┌────┐ ┌────┐ ┌────┐ ┌────┐              │  ░░░       │
│  ░░░░  │  │[av1]│[av2]│[av3]│[av4]│              │  ░░░       │
│  ░░░░  │  └────┘ └────┘ └────┘ └────┘              │  ░░░       │
│  ░░░░  │                                            │  ░░░       │
│  ░░░░  │  ─────────────────────────────────────     │  ░░░       │
│  ░░░░  │  Step 1 of 3        [─── ─── ───]         │  ░░░       │
│  ░░░░  │                                            │  ░░░       │
│  ░░░░  │  [Cancel]           [Continue →]          │  ░░░       │
│  ░░░░  └────────────────────────────────────────────┘  ░░░       │
└──────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Page background | Full-screen purple gradient: `radial-gradient(ellipse at 50% 0%, #3b0a6e 0%, #1a0a2e 40%, #0d0d1a 100%)` |
| Modal card | Glass card, max-width 500px, centered, padding 40px, border-radius 20px |
| Shield icon | 48px circle purple bg, shield icon 24px white, centered at top |
| Heading | Inter 800, 26px, #f9fafb, text-center |
| Subtitle | Inter 400, 15px, #9ca3af, text-center |
| Field label | "CHILD'S DISPLAY NAME" — Inter 600, 12px, #9ca3af, uppercase, letter-spacing 0.08em |
| Name input | Person icon prefix, full-width, placeholder "e.g. SpaceExplorer24" |
| "SELECT A VIBE" label | Same style as above |
| "See more" | Text link, right-aligned, #7c3aed, 13px |
| Avatar grid | 4 square cards, 80px × 80px each, border-radius 16px |
| Avatar card | Image fill, border-radius 16px |
| Selected avatar | Purple ring: `border: 3px solid #7c3aed; box-shadow: 0 0 0 2px rgba(124,58,237,0.3)` |
| Divider | 1px rgba(255,255,255,0.08) |
| Step label | "Step 1 of 3" — Inter 400, 13px, #9ca3af, left |
| Progress pills | 3 pills right-aligned: active = #7c3aed 40px, pending = rgba(255,255,255,0.2) 24px |
| Cancel | Secondary outlined, 44px height |
| Continue | Primary purple, "Continue →", 44px height |

---

## Screen 5b — Child Setup Step 2 (DOB + COPPA)

**File:** Inline step 2

```
┌──────────────────────────────────────────┐
│  📅                                      │
│  How old is your child?                  │
│  We need this for safety compliance.     │
│                                          │
│  DATE OF BIRTH                           │
│  [DD] / [MM] / [YYYY]                   │
│                                          │
│  ─────────────────────────────────────   │
│  ⚠️  Under 13? COPPA notice required    │
│  Children under 13 require additional   │
│  parental consent per COPPA guidelines. │
│                                          │
│  Step 2 of 3      [─── ─── ───]         │
│  [Back]                  [Continue →]   │
└──────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| DOB input | Three separate fields: day (2 digits), month (2 digits), year (4 digits) OR date picker |
| COPPA banner | Amber warning: amber icon + text, background rgba(245,158,11,0.1), border-left 3px #f59e0b |
| COPPA consent (if < 13) | Full COPPA disclosure text + checkbox "I am the parent/guardian and consent" |

---

## Screen 6 — Safety Gates (Step 3 of 3)

**File:** Inline step 3 OR `frontend/src/routes/parent/children/setup/safety-gates/+page.svelte`
**Reference:** Image 6 (Safety Gates screen)

### Layout (Full page, not modal)

```
┌───────────────────────────────────────────────────────────────────────┐
│  [🛡️ PARENTAL DASHBOARD]                                              │
│                                                                       │
│  Safety Gates                                                         │
│  Configure privacy and safety settings for your child's              │
│  Yapper account. These settings act as a digital shield.             │
│                                                                       │
│  ┌──────────────────────────────────────────────┐  ┌───────────────┐ │
│  │                                              │  │ 💡 QUICK TIP  │ │
│  │  ┌──────────────────────────────────────┐   │  │               │ │
│  │  │ [🛡️]  Auto-Hold DM Requests     [●─]│   │  │ Setting       │ │
│  │  │       Incoming messages from unknown │   │  │ "Hidden       │ │
│  │  │       users held in quarantine...    │   │  │ Search        │ │
│  │  └──────────────────────────────────────┘   │  │ Profile" is   │ │
│  │                                              │  │ recommended   │ │
│  │  ┌──────────────────────────────────────┐   │  │ for children  │ │
│  │  │ [👥]  Community Join Approval   [●─]│   │  │ under 13.     │ │
│  │  │       Require parental approval      │   │  │               │ │
│  │  │       before joining communities...  │   │  │ Read our      │ │
│  │  └──────────────────────────────────────┘   │  │ Safety Guide  │ │
│  │                                              │  │               │ │
│  │  ┌──────────────────────────────────────┐   │  ├───────────────┤ │
│  │  │ [🔍]  Hidden Search Profile     [─○]│   │  │               │ │
│  │  │       Profile invisible in global    │   │  │ 👨‍👩‍👧 Parental  │ │
│  │  │       search results.                │   │  │ Override      │ │
│  │  └──────────────────────────────────────┘   │  │               │ │
│  │                                              │  │ You can       │ │
│  │  ┌──────────────────────────────────────┐   │  │ always change │ │
│  │  │ [😊]  Content Filter: Strict    [●─]│   │  │ these settings│ │
│  │  │       Automatically filter offensive  │   │  │ later.        │ │
│  │  │       language and mature content... │   │  │               │ │
│  │  └──────────────────────────────────────┘   │  │ Step 3 of 3  │ │
│  │                                              │  │ 100% Complete │ │
│  │  ─────────────────────────────────────────  │  │ [══════════] │ │
│  │  [Back to Profile Setup]   [Create Account→]│  │               │ │
│  └──────────────────────────────────────────────┘  └───────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

### Safety Toggle Row

| Element | Spec |
|---------|------|
| Row card | Background: `#1a2e1a` (dark green-tinted), border-radius 12px, padding 20px |
| Border | `1px solid rgba(255,255,255,0.08)` |
| Icon | 40px circle, `#2d4a2d` background, icon 20px white |
| Title | Inter 700, 15px, #f9fafb |
| Description | Inter 400, 13px, #9ca3af |
| Toggle | Right-aligned custom toggle (see design tokens) |
| ON state | Toggle: #3b82f6 (blue) — matches reference image |
| OFF state | Toggle: rgba(255,255,255,0.15) |
| Row gap | 12px between cards |

### Safety Gate items:
| Gate | Icon | Default | Description |
|------|------|---------|-------------|
| Auto-Hold DM Requests | shield | ON | Messages from unknown users held in quarantine queue |
| Community Join Approval | users | ON | Require parental approval before joining |
| Hidden Search Profile | eye-off | OFF | Profile invisible in search results |
| Content Filter: Strict | smile | ON | Filter offensive language and mature content |

### Right Panel:

**Quick Tip card:**
- Glass card, border-radius 12px, padding 20px
- 💡 icon + "QUICK TIP" label in blue
- Tip text: 14px, #d1d5db
- "Read our Safety Guide" link: #3b82f6

**Parental Override card:**
- 👨‍👩‍👧 icon (blue) + "Parental Override" title
- Description text
- Progress: "Step 3 of 3" + "100% Complete"
- Progress bar: full-width blue `#3b82f6`

### Bottom CTA row:
- "Back to Profile Setup" — text link, #6b7280, left
- "Create Yapper Account →" — large primary button (dark bg with glow), center-right
  - `background: linear-gradient(135deg, #1e3a5f, #162d4a)` — dark blue (matches reference)
  - White text, border-radius 12px, padding 16px 32px

---

## Empty Parental Dashboard (No Children)

```
┌────────────────────────────────────────────────────┐
│                                                    │
│              🛡️  Set Up Family Safety              │
│                                                    │
│   Add your first child account to get started      │
│   with parental controls and safety monitoring.    │
│                                                    │
│   [Create Child Account →]                         │
│                                                    │
└────────────────────────────────────────────────────┘
```

---

## Review Modal (Pending Alert)

Opened from "Review" button on pending alert card:

```
┌────────────────────────────────────────────────────┐
│  Review DM Request                            [✕]  │
│                                                    │
│  [avatar 56px]  @gamer_dude99                      │
│                 Not in mutual followers            │
│                                                    │
│  This user wants to send direct messages           │
│  to Alex.                                          │
│                                                    │
│  Profile:  0 mutual friends · Joined 3 months ago │
│                                                    │
│  [View Profile]                                    │
│                                                    │
│  [Decline]                 [Approve]              │
└────────────────────────────────────────────────────┘
```

| Decline button | Secondary/outlined, red border |
| Approve button | Primary blue |
