# S5 & S6 — Media Messages, Real-Time Features, Live Canvas & Explore

**Sprint S5:** W11–W12 — Audio Yaps + Video Clips + Typing/Read Receipts/Presence
**Sprint S6:** W13–W14 — Live Canvas (Image 2 right panel) + Explore Page (Image 1)

---

## S5 — Media Messages

### YapRecorder

**File:** `frontend/src/lib/components/chat/YapRecorder.svelte`

Triggered by clicking the "Yap" mic button in `MessageInput`.

**States:**

1. **Pre-record (idle)** — shown when Yap button pressed:
```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │         Hold to record your Yap                  │   │
│  │                                                  │   │
│  │                 [🎤]                             │   │
│  │            ──────────────                        │   │
│  │           (tap to start)                         │   │
│  └──────────────────────────────────────────────────┘   │
│                                    [Cancel]              │
└─────────────────────────────────────────────────────────┘
```

2. **Recording:**
```
┌─────────────────────────────────────────────────────────┐
│  ● REC   0:07                                            │
│  ───────────────────────────────────────────────────────│
│  [Waveform animation — live bars, purple gradient]       │
│  ───────────────────────────────────────────────────────│
│  [Cancel]                              [Stop & Send →]  │
└─────────────────────────────────────────────────────────┘
```

3. **Preview (before send):**
```
┌─────────────────────────────────────────────────────────┐
│  ▶ Preview your Yap                                      │
│  ┌────────────────────────────────────────────────────┐  │
│  │  ▶   [waveform]   0:14                             │  │
│  └────────────────────────────────────────────────────┘  │
│  [Re-record]          [🔒 Encrypting...]   [Send Yap →] │
└─────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Panel | Slides up from bottom, glass card, border-radius top only (16px), backdrop-blur |
| REC indicator | Red dot (8px, pulsing) + "REC" Inter 700, 13px, #ef4444 |
| Timer | Inter 600, 18px, #f9fafb, monospace |
| Waveform | Canvas element, bars drawn via Web Audio API `analyser`, 40 bars, purple gradient `#7c3aed → #c4b5fd` |
| Encryption | Shows "🔒 Encrypting..." spinner before send (typically < 200ms) |
| Send button | Primary purple, "Send Yap →" |
| Cancel | Secondary/text button |
| Max duration | 5 minutes (shown as progress bar under waveform at 3+ min) |

---

### ClipRecorder

**File:** `frontend/src/lib/components/chat/ClipRecorder.svelte`

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │                                                  │   │
│  │          [camera preview / dark bg]              │   │
│  │                  16:9                            │   │
│  │                                                  │   │
│  └──────────────────────────────────────────────────┘   │
│  ● REC   0:12        🎤 (mic on)                         │
│  ────────────────────────────────────────                │
│  [Cancel]  [Flip cam]             [Stop & Preview →]     │
└─────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Preview | 16:9 video element, rounded 12px |
| REC indicator | Red animated pulse dot + timer |
| Flip camera | Icon button (Capacitor: use front/back camera) |
| Stop | Stops recording, goes to preview state |

**Preview state:**
```
┌─────────────────────────────────────────────────────────┐
│  ┌──────────────────────────────────────────────────┐   │
│  │              [video thumbnail]                   │   │
│  │              ▶  Play preview                     │   │
│  └──────────────────────────────────────────────────┘   │
│  Duration: 0:12                                          │
│  🔒 This clip will be encrypted before sending          │
│  [Re-record]                           [Send Clip →]    │
└─────────────────────────────────────────────────────────┘
```

---

### ClipMessage Bubble (Video)

**File:** `frontend/src/lib/components/chat/ClipMessage.svelte`

```
┌──────────────────────────────────────────┐
│  ┌──────────────────────────────────────┐ │
│  │  [video thumbnail / poster frame]    │ │
│  │                                      │ │
│  │                ▶                     │ │
│  │                                      │ │
│  └──────────────────────────────────────┘ │
│  0:12   •  CLIP  •  🔒                   │
└──────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Thumbnail | Max-width 300px, aspect-ratio 16:9, border-radius 12px |
| Play overlay | Centered 48px circle, white bg with opacity 0.9, black play icon 20px |
| On play | In-place video player; controls: play/pause, scrubber, volume |
| Labels | "CLIP" badge, duration, lock icon |
| Decrypt state | Spinner over thumbnail: "Decrypting..." |

---

### ReadReceipt

**File:** `frontend/src/lib/components/chat/ReadReceipt.svelte`

**DM view:**
```
You  10:46 AM
[message]
                                  ✓ Read 10:47
```

**Channel view:**
```
[message]
                                  👁 3 reads
```

| Element | Spec |
|---------|------|
| DM read | "✓✓" checkmarks (1 = sent, 2 = delivered, 2 in blue = read) — OR — "Read HH:MM" text |
| Channel | Small eye icon + count, 11px, #6b7280, right-aligned under message |
| Animation | Fade in `opacity: 0 → 1` over 300ms when read receipt arrives via WS |

---

## S6 — Live Canvas Panel

**File:** `frontend/src/lib/components/canvas/LiveCanvas.svelte`
**Reference:** Image 2, right side (360px panel)

### Panel Layout

```
┌─────────────────────────────────────────────────────┐
│  Live Canvas                              TRENDING ● │
├─────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────┐ │
│  │  MUSIC WIDGET                                   │ │
│  │  [Album art 56px circle]  Neon Nights      [⏸] │ │
│  │                           Synthwave Collective  │ │
│  │  ────────────────────────────────────────────  │ │
│  │  1:45                                     3:20  │ │
│  │  [██████████████████░░░░░░░░░░░░░░] progress    │ │
│  │  [avatar][avatar] + 12 listening now            │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─────────────────────────────────────────────────┐ │
│  │  POLL WIDGET                                    │ │
│  │  Next Game Night?                   ACTIVE      │ │
│  │  Valorant      65% [████████████████░░░░░░]     │ │
│  │  Apex Legends  25% [███████░░░░░░░░░░░░░░░]     │ │
│  │  Minecraft     10% [███░░░░░░░░░░░░░░░░░░░]     │ │
│  │  142 votes · Ends in 2h                         │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─────────────────────────────────────────────────┐ │
│  │  Community Clips                    [View All]  │ │
│  │  ┌──────────┐  ┌──────────┐                    │ │
│  │  │ [thumb]  │  │ [thumb]  │                    │ │
│  │  │  ACE!    │  │  Setup   │                    │ │
│  │  └──────────┘  └──────────┘                    │ │
│  │  ┌──────────┐  ┌──────────┐                    │ │
│  │  │ [thumb]  │  │   [+]    │                    │ │
│  │  │          │  │ Add Clip │                    │ │
│  │  └──────────┘  └──────────┘                    │ │
│  └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Panel width | 360px, right-side, fixed |
| Panel background | `#0f1117` |
| Panel border-left | `1px solid rgba(255,255,255,0.06)` |
| Header height | 48px, padding 0 16px |
| Header text | "Live Canvas" Inter 700, 15px, #f9fafb |
| TRENDING indicator | "TRENDING" + animated green dot, Inter 600, 11px, letter-spacing 0.08em, #22c55e |
| Section cards | Background rgba(255,255,255,0.04), border-radius 12px, padding 16px, margin-bottom 16px |
| Toggle button | Tab in channel header to show/hide canvas (arrow icon) |

---

### MusicWidget

**File:** `frontend/src/lib/components/canvas/MusicWidget.svelte`

| Element | Spec |
|---------|------|
| Album art | 56px circle, border 2px rgba(124,58,237,0.4), `animation: spin 8s linear infinite` when playing |
| Track name | Inter 700, 14px, #f9fafb |
| Artist name | Inter 400, 13px, #9ca3af |
| Play/Pause | 32px circle button, background rgba(255,255,255,0.1), icon 16px |
| Progress bar | Full-width, 4px height, background rgba(255,255,255,0.1), fill: linear-gradient(90deg, #7c3aed, #c4b5fd) |
| Time labels | "1:45" left, "3:20" right — Inter 400, 12px, #6b7280 |
| Listeners | Row of 2-3 small avatars + "+ 12 listening now" — Inter 400, 12px, #9ca3af |
| Admin badge | Admin-only: floating "Edit" icon top-right |
| No music state | "No music playing" — dim text, music-off icon |

**Spin animation:**
```css
@keyframes album-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
```

---

### PollWidget

**File:** `frontend/src/lib/components/canvas/PollWidget.svelte`

| Element | Spec |
|---------|------|
| Question | Inter 700, 15px, #f9fafb |
| ACTIVE badge | "ACTIVE" — #22c55e, 10px, 0.08em letter-spacing, uppercase |
| Option row | Option label + percentage + fill bar |
| Fill bar | `transition: width 0.5s ease-out` on vote update, background: linear-gradient(90deg, #7c3aed, #a78bfa) |
| Leading option bar | Slightly brighter / bolder |
| Vote count | "142 votes · Ends in 2h" — Inter 400, 13px, #9ca3af |
| Click to vote | Click option row → highlight, count increments optimistically |
| Already voted | Show checkmark on voted option, bars frozen |
| Expired | ACTIVE → "ENDED", bars static |

---

### ClipsCarousel

**File:** `frontend/src/lib/components/canvas/ClipsCarousel.svelte`

| Element | Spec |
|---------|------|
| Grid | 2×2 thumbnail grid, each 110px × 80px |
| Thumbnail | Video poster frame, border-radius 8px, object-fit: cover |
| Label | Caption below thumbnail, 11px, #9ca3af, 1-line truncated |
| Add Clip tile | Dashed border rgba(255,255,255,0.1), "+" icon 20px centered, "Add Clip" text 12px |
| Hover | Slight brightness + play icon overlay |
| Click | Expands to full-screen decrypted clip player |
| View All | Link to full clips list modal |

---

## S6 — Explore Page

**File:** `frontend/src/routes/(app)/explore/+page.svelte`
**Reference:** Image 1 (Explore screen)

### Full Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  [TOPNAV — with Explore active]                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Find Your Hype.  Join the Yap.                                  │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  🔍  Search for communities, live yaps, or vibes...  CMD+K│  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  TRENDING:  #gaming  #tech-talk  🔥 hot-takes  #sneakerheads    │
│                                                                  │
│  Live Now & Trending                              [⊞] [☰]       │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐     │
│  │ ● LIVE   12k   │  │     2m ago     │  │  [image card]  │     │
│  │ [live server   │  │  [media card]  │  │  Digital Art   │     │
│  │  card]         │  │  Late Night    │  │  Expo 2024     │     │
│  │                │  │  Lofi Beats    │  │  #3dart        │     │
│  │ Is AI Art Real?│  │  @techbro      │  └────────────────┘     │
│  │ @techbro + 3   │  │  Study vibes.. │  ┌────────────────┐     │
│  └────────────────┘  └────────────────┘  │ "Pineapple     │     │
│  ┌────────────────┐  ┌────────────────┐  │  belongs on    │     │
│  │ [community     │  │ [community     │  │  pizza."       │     │
│  │  card]         │  │  card]         │  │  @pizza_king   │     │
│  │ Retro Gamers   │  │ Frontend Wiz.  │  │  ♥ 452 💬 89  │     │
│  │ United         │  │ 452 Online ●   │  └────────────────┘     │
│  │ #nintendo #sega│  │                │                         │
│  └────────────────┘  └────────────────┘                         │
└──────────────────────────────────────────────────────────────────┘
```

### Page Header

| Element | Spec |
|---------|------|
| Heading | "Find Your Hype. **Join the Yap.**" — "Find Your Hype." in #f9fafb, "Join the Yap." in brand gradient |
| Font | Inter 900, 48px, tracking-tight |
| Subtitle | Hidden (heading is sufficient) |
| Padding top | 40px below topnav |

---

### Search Bar

**File:** `frontend/src/lib/components/explore/SearchBar.svelte`

| Element | Spec |
|---------|------|
| Width | 100%, max-width 900px |
| Height | 56px |
| Background | rgba(255,255,255,0.05) |
| Border | 1px solid rgba(255,255,255,0.1) |
| Border radius | 16px |
| Focus border | 1px solid rgba(124,58,237,0.5) + glow |
| Icon | 🔍 20px, left-padded, #6b7280 |
| Placeholder | "Search for communities, live yaps, or vibes..." — #4b5563 |
| Shortcut hint | "CMD+K" — glass pill right-side, Inter 500 12px |
| Debounce | 350ms before API call |
| Results dropdown | Glass card, max-height 400px, scrollable — servers + users sections |

---

### TrendingTags

**File:** `frontend/src/lib/components/explore/TrendingTags.svelte`

```
TRENDING:  [# gaming]  [# tech-talk]  [🔥 hot-takes]  [# sneakerheads]  [🎤 live-music]
```

| Element | Spec |
|---------|------|
| "TRENDING:" label | Inter 600, 12px, #6b7280, uppercase, letter-spacing 0.08em |
| Tag chips | Pills: background rgba(255,255,255,0.05), border 1px rgba(255,255,255,0.1), Inter 500, 13px, padding 6px 14px |
| Icon tags | Emoji or icon prefix + tag name |
| Active/selected | Background rgba(124,58,237,0.15), border rgba(124,58,237,0.4), text #a78bfa |
| Horizontal scroll | On mobile: horizontal scroll with no scrollbar |

---

### Grid/List Toggle

```
[⊞ Grid]  [☰ List]
```
Two icon buttons, top-right of "Live Now & Trending" section.
Active state: background rgba(124,58,237,0.2).

---

### LiveServerCard

**File:** `frontend/src/lib/components/explore/LiveServerCard.svelte`

```
┌──────────────────────────────────────────┐
│  ● LIVE   🧑‍🤝‍🧑 12k                       │
│                                          │
│  [background image or gradient]          │
│                                          │
│  Is AI Art Real Art? 🤖🎨                │
│  @techbro + 3 others speaking            │
└──────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card size | 300px × 200px (aspect-ratio 3:2) |
| Background | Random vibrant gradient (server-defined or procedural from name hash) |
| LIVE badge | Top-left, red pill "● LIVE", pulse animation on the dot |
| Member count | Top-right, dark glass pill, person-icon + count |
| Title | Bottom overlay, Inter 800, 18px, white |
| Speakers | "@ + X others speaking" small text, #f9fafb with opacity 0.8 |
| Gradient overlay | Bottom fade: `linear-gradient(transparent 30%, rgba(0,0,0,0.8) 100%)` |
| Hover | Scale 1.02, border rgba(124,58,237,0.4) |

---

### CommunityCard

**File:** `frontend/src/lib/components/explore/CommunityCard.svelte`

```
┌──────────────────────────────────────────┐
│  [server icon 40px]              [Join]  │
│  Retro Gamers United                     │
│  The ultimate spot for 8-bit lovers...   │
│  [#nintendo] [#sega]                     │
└──────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Card | Glass card, padding 16px 20px, border-radius 12px |
| Icon | 40px circle, fallback: gradient bg + first letter of name |
| Name | Inter 700, 15px, #f9fafb |
| Description | Inter 400, 13px, #9ca3af, 2 lines max |
| Tags | Small chips, #gaming style, max 3 shown |
| Join button | "Join" text-link on right in brand purple |
| Joined state | "Joined ✓" in #22c55e |
| Pending | "Pending..." for child accounts awaiting approval |

---

### TopYapperCard (Quote Style)

**File:** `frontend/src/lib/components/explore/TopYapperCard.svelte`

```
┌──────────────────────────────────────────┐
│  ❝  "Pineapple belongs on pizza.        │
│      Fight me."                          │
│                                          │
│  [avatar] @pizza_king                   │
│                                          │
│  ♥ 452   💬 89                          │
└──────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Background | Bright accent color (varies per card — yellow, orange, blue, etc.) |
| Quote marks | Large "❝" decorative, top-left, 40px, semi-transparent |
| Quote text | Inter 700, 18px, #1a1a2e (dark on bright bg) or white |
| Avatar | 28px + @handle, Inter 500, 13px |
| Engagement | Heart + count, comment + count |
| Card sizes | Varies in masonry grid, min-height 160px |

---

### Live Audio Server Card

```
┌──────────────────────────────────────────────────────┐
│  🔊 LIVE AUDIO                           Started 5m ago│
│  NBA Finals Predictions 🏀                           │
│  [avatar] HoopsTalk                        [🎧]     │
│           Host                                       │
└──────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| "LIVE AUDIO" badge | Blue pill, mic-wave icon, #3b82f6 |
| Speaker icon | Headphones 24px, right-aligned, primary purple |
| Host info | Avatar 32px + username + "Host" label |
| Card height | 80px, list-style |

---

### Empty / No Results State

```
┌──────────────────────────────────────────────────────┐
│               🔍                                     │
│    No communities found for "obscure query"          │
│                                                      │
│    Try a different search term or                    │
│    [Create a community]                              │
└──────────────────────────────────────────────────────┘
```

---

## Search Results Dropdown

```
┌─────────────────────────────────────────────────────┐
│  SERVERS                                            │
│  [icon] Retro Gamers United  · 1.2k members         │
│  [icon] Frontend Wizards     · 452 members          │
│                                                     │
│  USERS                                              │
│  [avatar] @neo_kai — CyberPunk vibes               │
│  [avatar] @pixel_queen — Digital artist             │
│                                                     │
│  Press ↵ to search all results                      │
└─────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Container | Glass card, max-width 600px, below search bar, z-index: 100 |
| Section label | "SERVERS" / "USERS" — uppercase 11px #6b7280 |
| Row | 44px height, avatar/icon + name + metadata |
| Hover | rgba(124,58,237,0.1) background |
| Keyboard nav | ↑/↓ to navigate, ↵ to select |
| Close | Click outside or Escape |
