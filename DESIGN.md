---
name: Yapper
description: End-to-end-encrypted, Discord-like real-time chat — playful, private, modern.
colors:
  brand-300: "#c4b5fd"
  brand-400: "#a78bfa"
  brand-500: "#8b5cf6"
  brand-600: "#7c3aed"
  brand-700: "#6d28d9"
  brand-800: "#5b21b6"
  brand-950: "#2e1065"
  bg-base: "#0f0f0f"
  bg-surface: "#1a1a1a"
  bg-elevated: "#222222"
  bg-nav: "#0d0d14"
  text-primary: "#fafafa"
  text-secondary: "#a1a1aa"
  text-muted: "#52525b"
  border-default: "#2a2a2a"
  border-subtle: "#1e1e1e"
  success: "#22c55e"
  warning: "#f59e0b"
  error: "#ef4444"
  info: "#3b82f6"
  safety-bg: "#1a2e1a"
typography:
  display:
    fontFamily: "Inter, 'Segoe UI', system-ui, sans-serif"
    fontSize: "36px"
    fontWeight: 800
    lineHeight: 1.1
    letterSpacing: "-0.02em"
  headline:
    fontFamily: "Inter, 'Segoe UI', system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 800
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  title:
    fontFamily: "Inter, 'Segoe UI', system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "normal"
  body:
    fontFamily: "Inter, 'Segoe UI', system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: "normal"
  label:
    fontFamily: "Inter, 'Segoe UI', system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 700
    lineHeight: 1.4
    letterSpacing: "0.08em"
  mono:
    fontFamily: "'JetBrains Mono', 'Cascadia Code', monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "normal"
rounded:
  sm: "6px"
  md: "10px"
  lg: "16px"
  full: "9999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "6": "24px"
  "8": "32px"
  "12": "48px"
components:
  button-primary:
    backgroundColor: "{colors.brand-600}"
    textColor: "#ffffff"
    rounded: "{rounded.full}"
    padding: "10px 24px"
  button-primary-hover:
    backgroundColor: "{colors.brand-700}"
    textColor: "#ffffff"
    rounded: "{rounded.full}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.full}"
    padding: "10px 24px"
  input:
    backgroundColor: "{colors.bg-elevated}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.md}"
    padding: "12px 16px"
  chip:
    backgroundColor: "{colors.bg-elevated}"
    textColor: "{colors.text-secondary}"
    rounded: "{rounded.full}"
    padding: "6px 14px"
  card:
    backgroundColor: "{colors.bg-surface}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.lg}"
    padding: "24px"
---

# Design System: Yapper

## 1. Overview

**Creative North Star: "The Lit Room After Dark"**

Yapper is a dark room that glows with life. The darkness is not a style choice
borrowed from developer tools; it is the felt expression of privacy — a calm,
unlit space where nothing is on display by default. Into that quiet dark comes
the glow: the violet Sphere brand mark, presence dots, typing indicators, live
badges, and the warm light of an active conversation. The dark is the privacy;
the people are the light. That single tension governs every surface.

The personality is playful, private, and modern. Energy lives in motion and the
brand violet, never in clutter or noise. Surfaces stack in near-black tonal
layers (`#0F0F0F` → `#1A1A1A` → `#222222`) so dense, long-running conversations
stay legible for hours, while a restrained violet glow marks the few things that
deserve attention. On desktop (Tauri), backgrounds turn semi-transparent so the
OS vibrancy (Mica/Acrylic) shows through, making the room feel physically real.

This system explicitly rejects three things. It is **not a generic Discord
clone**: it shares the server/channel/presence vocabulary but never the blurple
palette or Discord's exact chrome. It is **not a crypto/web3 aesthetic**: no
neon-on-black, no lock icons or circuit-board gradients performing "security" —
privacy is communicated through calm and clarity, not theatre. It is **not
childish**: despite COPPA parental controls, there are no mascots or primary-color
playground styling; safety reads as adult-legible trust.

**Key Characteristics:**
- Near-black tonal layering carries depth; violet glow is rationed, never ambient.
- One brand hue (violet `#7C3AED`) does all the identity work.
- Fully-rounded, springy, tactile controls — lively, never stiff.
- Inter for everything, weight contrast for hierarchy; JetBrains Mono for code/keys.
- Real-time signals (presence, typing, live) are first-class visual citizens.

## 2. Colors

A single saturated violet against a near-black, zinc-tinted neutral field, with a
small functional status set. The violet is the only chromatic voice in the room.

### Primary
- **Electric Violet** (`#7C3AED`, `brand-600`): The one brand color. Primary
  buttons, links, active nav, toggles, focus rings, the `NEW` badge, the Sphere
  core. This is the glow.
- **Violet Light** (`#A78BFA`, `brand-400`): Interactive text, link labels,
  hover-state accents, the Sphere's mid-tone, decorative orbital rings.
- **Violet Specular** (`#C4B5FD`, `brand-300`): The Sphere's highlight and the
  light end of the premium gradient. Almost never used as a flat fill.
- **Violet Deep** (`#2E1065`, `brand-950`): The Sphere's shadow core and the dark
  end of glows. A shadow color, not a surface color.

### Secondary (functional status, not brand)
- **Online Green** (`#22C55E`): Presence "online", approvals, success.
- **Away Amber** (`#F59E0B`): Pending state, "away" presence, warnings.
- **Danger Red** (`#EF4444`): Errors, destructive actions, and the `LIVE` badge.
- **Info Blue** (`#3B82F6`): Informational alerts only. Never competes with violet
  for "primary action" duty.

### Tertiary (semantic theme)
- **Safety Green-Black** (`#1A2E1A`): The dark teal-green ground for parental and
  safety surfaces. Signals a distinct, calmer "guardian" context without leaving
  the dark room.

### Neutral
- **Ink White** (`#FAFAFA`, `text-primary`): Headings and body. Never pure `#FFF`.
- **Zinc Secondary** (`#A1A1AA`, `text-secondary`): Subtitles, labels,
  placeholders. Verified ≥4.5:1 on every surface step.
- **Zinc Muted** (`#52525B`, `text-muted`): Timestamps, inactive nav, disabled.
  Decorative/non-essential text only — never body copy.
- **Void** (`#0F0F0F`, `bg-base`): The room. Page/app root.
- **Surface** (`#1A1A1A`, `bg-surface`): Cards, panels, sidebars — one step up.
- **Elevated** (`#222222`, `bg-elevated`): Inputs, hover states — two steps up.
- **Nav Deep** (`#0D0D14`, `bg-nav`): The slightly cooler, deeper nav/rail black.
- **Hairline** (`#2A2A2A`, `border-default`): Default 1px borders and dividers.

### Named Rules
**The One Glow Rule.** Violet is the only brand color and earns its presence by
scarcity — primary CTAs, the active element, and live/real-time signals. If a
screen has more than a few violet elements competing, the glow has become wallpaper.

**The Light-Theme Parity Rule.** A `[data-theme='light']` mode exists: white
cards (`bg-surface #FFFFFF`) and nav sit raised above a soft-zinc page canvas
(`bg-base #F1F1F4`), with inputs a faint step inset (`bg-elevated #E8E8EC`), so
depth still comes from tonal layers rather than borders alone. Any new surface
MUST define both themes; a component that only works in the dark room is
unfinished.

Brand and status hues read as *text* differently per theme: the saturated palette
values (violet `#7C3AED`, danger `#EF4444`, amber `#F59E0B`) are bright enough for
white text on near-black but fail AA as dark-on-tint in light. Light theme
therefore uses darker text tokens — `--color-brand-text #5B21B6`,
`--color-error-text #B42318`, `--color-warning-text #854D0E` — for colored labels
and destructive actions, while the *tinted backgrounds* keep the canonical hue.
This hue shift in light is intentional AA adaptation, not drift. (Dark theme keeps
the bright `--color-*-text` values: brand `#A78BFA`, error `#FCA5A5`, etc.)

## 3. Typography

**Display / Body Font:** Inter (with `'Segoe UI', system-ui, -apple-system, sans-serif`)
**Label / Mono Font:** JetBrains Mono (with `'Cascadia Code', monospace`)

**Character:** One humanist-geometric sans does all the talking; hierarchy comes
from a wide weight range (400 → 900), not from a second typeface. Mono appears
only where the content is literally code, keys, or fingerprints — the visible
edge of the encryption. The pairing is modern and quiet, letting motion and color
carry the personality.

### Hierarchy
- **Marketing Hero** (900, 48px, line-height 1.0, tracking -0.02em): Landing-page
  hero only. App UI never goes this large.
- **Display** (800, 36px, 1.1): Top app/page headings.
- **Headline** (800, 28px, 1.2): Page-section titles.
- **Title** (600–700, 18–22px, 1.3–1.4): Card titles, section headers.
- **Body** (400, 15px, 1.6): Messages and body text. The workhorse. Cap prose at
  65–75ch; chat bubbles set their own max width.
- **Nav / Label** (500, 16px, 1.5): Nav items and form labels.
- **Caption** (400, 13px, 1.5): Secondary labels, captions.
- **Micro Label** (700, 11px, 1.4, tracking 0.08em, UPPERCASE): Badges,
  timestamps, `LIVE` / `NEW` pills. Uppercase is reserved for ≤4-word labels only.

### Named Rules
**The Mono-Means-Crypto Rule.** JetBrains Mono signals "this is cryptographic
material" — safety numbers, device fingerprints, invite codes. Don't use it
decoratively; its rarity is what makes it legible as a signal.

**The No-Shout Rule.** No all-caps body copy, ever. Uppercase stops at the 11px
micro-label tier. Headline tracking never goes below -0.02em.

## 4. Elevation

Depth is built from **tonal layering first, shadow second, glow third.** Surfaces
step through near-blacks (`#0F0F0F` → `#1A1A1A` → `#222222`); a dark drop shadow
lifts cards and modals off the room; violet glow is reserved almost entirely for
the Sphere and primary CTAs. On desktop, Tauri vibrancy replaces flat backgrounds
with OS blur, so the layering becomes literal translucency.

### Shadow Vocabulary
- **Card** (`box-shadow: 0 1px 3px rgba(0,0,0,0.4)`): Resting cards, dropdowns.
- **Floating** (`box-shadow: 0 4px 12px rgba(0,0,0,0.5)`): Popovers, menus.
- **Modal** (`box-shadow: 0 8px 24px rgba(0,0,0,0.6)`): Dialogs, bottom sheets.
- **Brand Glow** (`box-shadow: 0 0 24px rgba(124,58,237,0.4)`): The Sphere, hero
  CTAs, focused premium surfaces. Rationed — see the One Glow Rule.

### Named Rules
**The Glow-Is-Earned Rule.** Violet glow is not an ambient effect applied to every
card. It marks the Sphere, the single primary action, or an actively "live"
element. A screen where everything glows reads as the crypto aesthetic we reject.

**The Glass-Is-Surface-Glow, Not-Decoration Rule.** Backdrop-blur glass
(`blur(12px) saturate(180%)`) belongs to floating panels and Tauri vibrancy where
real depth exists behind it. Do not scatter glass cards over flat backgrounds for
decoration.

## 5. Components

### Buttons
- **Shape:** Fully rounded pills (`9999px`, `rounded.full`). This is the signature
  control shape across the whole product.
- **Primary:** Violet `#7C3AED` fill, white text, 700 weight, `10px 24px` padding.
- **Hover / Focus:** Background → `#6D28D9` (`brand-700`), `translateY(-1px)` lift
  with a springy ease; `:active` returns to `translateY(0)`. Focus-visible shows a
  violet ring (`0 0 0 3px rgba(124,58,237,0.15)`).
- **Secondary:** Transparent fill, 1px strong border, primary text; hover fills to
  `bg-elevated`.
- **Destructive:** Translucent red fill (`rgba(239,68,68,0.1)`), red text, red
  border. Never a solid red block.

### Chips / Tags
- **Style:** `bg-elevated` fill, 1px hairline border, secondary text, pill radius,
  6px gap for leading icon.
- **State:** Hover shifts border to violet (`border-brand`) and text to violet —
  the chip lights up rather than changing shape.

### Cards / Containers
- **Corner Style:** `16px` (`rounded.lg`); large modals/sheets go `24px`.
- **Background:** `bg-surface` (`#1A1A1A`), one step above the room.
- **Shadow Strategy:** Resting Card shadow; see Elevation. Glass variant uses
  backdrop-blur only on floating/vibrancy contexts.
- **Border:** 1px hairline (`#2A2A2A`); strengthens on hover.
- **Internal Padding:** `24px` default (`space-6`).

### Inputs / Fields
- **Style:** `bg-elevated` fill, 1px hairline border, `10px` radius, 15px text,
  `12px 16px` padding.
- **Focus:** Border → `brand-600`, plus a 3px violet ring
  (`0 0 0 3px rgba(124,58,237,0.15)`). No outline removal without this ring.
- **Placeholder:** `text-muted` — but verify ≥4.5:1; bump toward secondary if close.

### Navigation
- **App shell:** 56px sticky topnav; 240px server sidebar; right-side Live Canvas
  panel at 360px (toggleable). Marketing header is 64px.
- **States:** Inactive nav uses `text-muted`; active uses primary text with a
  violet indicator. Mobile collapses the sidebar to a slide-over
  (`transform 250ms ease-out`).
- **Z-index scale:** base 0 → raised 10 → dropdown 100 → sticky 200 → modal 300 →
  toast 400 → tooltip 500. Never use arbitrary `9999`.

### The Sphere (signature component)
The violet 3D Sphere is Yapper's brand mark — app icon, onboarding hero, and the
animated marketing hero. Radial gradient `circle at 35% 35%` through
`#C4B5FD → #7C3AED → #2E1065`, with a soft outer brand glow, an inset deep-violet
shadow for volume, a white specular highlight (top-left), and optional decorative
orbital rings at low opacity. It is the single most important visual asset; treat
it as the literal source of the room's glow.

## 6. Do's and Don'ts

### Do:
- **Do** build depth from tonal near-black layers (`#0F0F0F` → `#1A1A1A` →
  `#222222`) first; reach for shadow and glow only after.
- **Do** keep violet rare and intentional — primary action, active state, and
  live/real-time signals (the One Glow Rule).
- **Do** use fully-rounded pill buttons and springy hover lifts; the controls
  should feel tactile and alive.
- **Do** define both `dark` (default) and `[data-theme='light']` for every new
  surface, and respect Tauri vibrancy translucency.
- **Do** reserve JetBrains Mono for actual cryptographic material (safety numbers,
  fingerprints, invite codes).
- **Do** verify body text hits ≥4.5:1 on its surface step, and honor
  `prefers-reduced-motion` with a crossfade/instant alternative for every animation.

### Don't:
- **Don't** make it a **generic Discord clone** — no Discord blurple, no copied
  chrome. Earn the server/channel conventions; don't inherit the skin.
- **Don't** drift into a **crypto / web3 aesthetic** — no neon-on-black, no lock
  icons or circuit-board gradients performing security. Privacy is calm, not loud.
- **Don't** make it **childish** — no mascots, no primary-color playground styling
  on parental/safety surfaces; safety reads as adult trust (`safety-bg #1A2E1A`).
- **Don't** let violet glow become ambient wallpaper on every card; if everything
  glows, nothing does.
- **Don't** use all-caps for anything longer than a 4-word label, and never tighten
  heading tracking past -0.02em.
- **Don't** scatter glassmorphism over flat backgrounds for decoration; glass is
  for floating panels and real OS vibrancy only.
- **Don't** use `text-muted` (`#52525B`) for body copy — it's for timestamps and
  inactive states; it fails contrast as body text.
