# S0 — Marketing Site UI Spec

**Sprint:** S0 (W1–W2)
**Framework:** Astro 4 + Svelte islands
**Route:** `yapperhq.com`
**File:** `marketing/src/pages/index.astro`

---

## Page Layout

```
┌─────────────────────────────────────────────────────┐
│  HEADER / NAV (sticky, 64px)                        │
├─────────────────────────────────────────────────────┤
│  1. HERO                         (~100vh)            │
│  2. FEATURE GRID                 (auto)              │
│  3. HOW IT WORKS                 (auto)              │
│  4. SAFETY SECTION               (auto)              │
│  5. PLATFORM BADGES              (auto)              │
│  6. PRICING PREVIEW              (auto)              │
│  7. FAQ ACCORDION                (auto)              │
│  8. FOOTER CTA                   (auto)              │
└─────────────────────────────────────────────────────┘
```

**Background:** `#0a0a0f` throughout
**Max content width:** 1200px, centered with `margin: 0 auto; padding: 0 24px`

---

## Header / Nav

**File:** `marketing/src/layouts/Base.astro`

```
┌────────────────────────────────────────────────────────────────┐
│  [sphere icon 28px] Yapper    Home  Features  Safety   [Join →]│
└────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Height | 64px |
| Background | `rgba(10, 10, 15, 0.9)` + `backdrop-filter: blur(12px)` |
| Border bottom | `1px solid rgba(255,255,255,0.06)` |
| Position | `sticky top: 0; z-index: 200` |
| Logo | Sphere icon (28px) + "Yapper" text — Inter 700, #f9fafb |
| Nav links | Inter 500, 15px, #9ca3af → hover: #f9fafb |
| CTA button | "Join the Wishlist" — purple pill button, 36px height |

---

## 1. Hero Section

**File:** `marketing/src/components/Hero.astro`

```
┌────────────────────────────────────────────────────────────────┐
│                                                                 │
│     ┌──────────────┐     A New Way to Yap.                    │
│     │  3D SPHERE   │                                           │
│     │   (240px)    │     End-to-end encrypted messaging with   │
│     │  animated    │     voice Yaps, video Clips, live server  │
│     └──────────────┘     canvases, and built-in parental       │
│                          safety. Coming soon.                   │
│                                                                 │
│               ┌──────────────────────────────┐                 │
│               │  your@email.com  [Join →]    │                 │
│               └──────────────────────────────┘                 │
│                   🔒  1,247 people already waiting             │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Layout | Two-column on desktop, stacked on mobile |
| Headline | "A New Way to Yap." — Inter 900, 64px, #f9fafb, tracking-tight |
| Brand word "Yap" | gradient text: `linear-gradient(135deg, #c4b5fd, #7c3aed)` |
| Subheadline | Inter 400, 18px, #9ca3af, max-width 520px |
| Sphere | 240px diameter, CSS `radial-gradient` sphere (see design tokens), `animation: pulse 3s ease-in-out infinite` — scale 1.0 → 1.03 → 1.0 |
| Sphere orbital ring | 300px diameter ellipse, rotated -25deg, stroke rgba(167,139,250,0.15), animated rotation |
| Email input | Full-width max-width 400px, glass input style, placeholder "your@email.com" |
| Submit button | "Join the Wishlist →", primary purple, attached to input or below on mobile |
| Counter | Inter 500, 14px, #6b7280 — "🔒 X people already waiting" |
| Padding top | 120px (below sticky nav) |

**Sphere animation:**
```css
@keyframes sphere-pulse {
  0%, 100% { transform: scale(1); box-shadow: 0 0 40px rgba(124, 58, 237, 0.4); }
  50% { transform: scale(1.03); box-shadow: 0 0 60px rgba(124, 58, 237, 0.6); }
}
@keyframes ring-rotate {
  from { transform: rotate(-25deg); }
  to { transform: rotate(335deg); }
}
```

---

## 2. Feature Grid

**File:** `marketing/src/components/FeatureGrid.astro`

8 cards in a responsive grid (4-col desktop, 2-col tablet, 1-col mobile):

```
┌──────┬──────┬──────┬──────┐
│  🔒  │  🎤  │  📹  │  ✨  │
│ E2EE │ Yap  │ Clip │ Canvas│
├──────┼──────┼──────┼──────┤
│  🛡️  │  🔗  │  😊  │  🧭  │
│Safety│Disc. │Emoji │Explore│
└──────┴──────┴──────┴──────┘
```

**Card spec:**
```css
.feature-card {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 16px;
  padding: 28px 24px;
  transition: border-color 0.2s, background 0.2s;
}
.feature-card:hover {
  border-color: rgba(124, 58, 237, 0.4);
  background: rgba(124, 58, 237, 0.05);
}
```

| Feature | Icon | Headline | Subtext |
|---------|------|----------|---------|
| E2EE | Lock (24px, #7c3aed) | End-to-End Encrypted | Not even we can read your messages |
| Yaps | Mic | Audio Yaps | Your voice, delivered in seconds |
| Clips | Video | Video Clips | Share the moment, not the file |
| Canvas | Sparkles | Live Canvas | Music, polls, and clips — live in every server |
| Safety | Shield | Parental Safety | COPPA-compliant. Metadata-only. E2EE preserved. |
| Discord | ArrowRightLeft | Discord Import | Bring your profile and bots. No starting over. |
| Emojis | Smile | Custom Emojis | Your server, your expressions |
| Explore | Compass | Explore & Discover | Find your community in seconds |

**Section header:**
- "Everything you need." — Inter 800, 40px, #f9fafb
- Subheadline — Inter 400, 18px, #9ca3af
- Section padding: 96px top/bottom

---

## 3. How It Works

**File:** `marketing/src/components/HowItWorks.astro`

3-step horizontal stepper:

```
  ① ──────────────── ② ──────────────── ③
Create your Yapper   Find your servers   Start Yapping
username + vibe     explore trending     E2EE messages
                    communities          Yaps, Clips
```

| Element | Spec |
|---------|------|
| Step number | Circle 40px, brand purple bg, white Inter 700 |
| Connector line | 2px dashed, rgba(124,58,237,0.3) |
| Step title | Inter 700, 20px, #f9fafb |
| Step description | Inter 400, 15px, #9ca3af |
| Layout | 3-column flexbox on desktop, vertical on mobile |
| Section bg | Subtle gradient: `radial-gradient(ellipse at 50% 0%, rgba(124,58,237,0.08) 0%, transparent 70%)` |

---

## 4. Safety Section

**File:** `marketing/src/components/SafetySection.astro`

Two-column layout:

```
┌─────────────────────────────────────────────────────┐
│  Left: Copy                Right: Dashboard Preview  │
│                                                      │
│  "Privacy That Can't      [Screenshot mockup of     │
│   Be Compromised."         parental dashboard]       │
│                                                      │
│  Signal Protocol —         (Image 4 as static        │
│  same tech as Signal       preview card)             │
│  and WhatsApp.                                       │
│                                                      │
│  Parents see metadata      ✓ Friend request control  │
│  only. Never messages.     ✓ Server join approval    │
│                            ✓ Real-time alerts        │
└─────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Heading | Inter 800, 40px, #f9fafb |
| Purple accent | "Can't Be Compromised" in brand gradient |
| Left column | 50% width, copy + checkmark list |
| Right column | 50% width, screenshot mockup in glass frame with glow shadow |
| Checkmarks | 18px `check-circle` icon in #22c55e |
| Background | Dark panel: rgba(255,255,255,0.02) with border |

---

## 5. Platform Badges

**File:** `marketing/src/components/PlatformBadges.astro`

Centered row of platform icons with labels:

```
  🌐 Web   📱 iOS   🤖 Android   🖥 macOS   🪟 Windows   🐧 Linux
```

| Element | Spec |
|---------|------|
| Icon size | 32px platform SVG icon |
| Label | Inter 500, 13px, #9ca3af |
| Container | flex row, gap 40px, centered |
| Background | None (blends with page) |

---

## 6. Pricing Preview

**File:** `marketing/src/components/PricingPreview.astro`

Two cards side-by-side:

```
┌──────────────────┐  ┌──────────────────────┐
│  FREE            │  │  GoPro 🚀            │
│                  │  │                      │
│  ✓ Core chat     │  │  ✓ Everything in Free│
│  ✓ 50 emojis     │  │  ✓ 100 emojis        │
│  ✓ Yaps & Clips  │  │  ✓ Custom badge       │
│  ✓ Audio Yaps    │  │  ✓ 50MB uploads       │
│  ✓ Community     │  │  ✓ Priority support   │
│                  │  │                      │
│  [Join Wishlist] │  │  [Join Wishlist]     │
└──────────────────┘  └──────────────────────┘
```

| Element | Spec |
|---------|------|
| Free card | Glass card, white border |
| GoPro card | Purple gradient border `linear-gradient(135deg, #7c3aed, #c4b5fd)`, slight glow |
| GoPro badge | "COMING SOON" chip in purple |
| Pricing text | "Free" and "Join wishlist to know first" — no prices shown |
| CTA | Both: "Join the Wishlist" → scroll to top form |
| Check marks | #7c3aed brand purple |

---

## 7. FAQ Accordion

**File:** `marketing/src/components/FAQAccordion.svelte` (Svelte island)

6 questions in single-column accordion:

| # | Question |
|---|---------|
| 1 | Is it really end-to-end encrypted? |
| 2 | How do parental controls work without reading messages? |
| 3 | Can I import my Discord account? |
| 4 | When does Yapper launch? |
| 5 | Is it free? |
| 6 | What platforms will it support? |

**Item spec:**
```css
.faq-item {
  border-bottom: 1px solid rgba(255,255,255,0.08);
  padding: 20px 0;
}
.faq-question {
  font: 600 17px/1.4 Inter;
  color: #f9fafb;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.faq-answer {
  font: 400 15px/1.6 Inter;
  color: #9ca3af;
  margin-top: 12px;
  /* Animated: max-height 0 → auto, opacity 0 → 1 */
}
.faq-chevron {
  transition: transform 0.2s;
}
.faq-item.open .faq-chevron {
  transform: rotate(180deg);
}
```

---

## 8. Footer / CTA

**File:** `marketing/src/components/Footer.astro`

```
┌──────────────────────────────────────────────────────┐
│                  Join the Waitlist                    │
│           Be first to know when we launch            │
│                                                      │
│    ┌──────────────────────────────────────────┐      │
│    │  your@email.com            [Join →]      │      │
│    └──────────────────────────────────────────┘      │
│                                                      │
│  [sphere icon] Yapper    Privacy · Terms · Discord   │
│  © 2026 Yapper. All rights reserved.                 │
└──────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| Footer CTA heading | Inter 800, 36px |
| Email input | Same as hero |
| Divider | 1px solid rgba(255,255,255,0.06) |
| Links | Inter 400, 14px, #6b7280 → hover #9ca3af |
| Copyright | Inter 400, 13px, #4b5563 |

---

## WishlistForm.svelte (Interactive Island)

**File:** `marketing/src/components/WishlistForm.svelte`

States:
1. **Idle** — email input + submit button
2. **Loading** — button shows spinner, input disabled
3. **Success** — "🎉 You're on the list! Check your inbox." (green)
4. **Already registered** — "You're already on the list." (neutral)
5. **Error** — "Something went wrong. Try again." (red)
6. **Rate limited** — "Too many attempts. Try again later." (orange)

```svelte
<form on:submit|preventDefault={handleSubmit}>
  <div class="input-group">
    <input
      type="email"
      bind:value={email}
      placeholder="your@email.com"
      disabled={loading || success}
      required
    />
    <button type="submit" disabled={loading || success}>
      {#if loading}
        <Spinner size={16} />
      {:else}
        Join →
      {/if}
    </button>
  </div>
  {#if message}
    <p class="form-message {messageType}">{message}</p>
  {/if}
</form>
```

---

## Global CSS

**File:** `marketing/src/styles/global.css`

```css
:root {
  --bg-base: #0a0a0f;
  --color-brand: #7c3aed;
  --color-text: #f9fafb;
  --color-text-secondary: #9ca3af;
}

* { box-sizing: border-box; margin: 0; padding: 0; }

html { scroll-behavior: smooth; }

body {
  background: var(--bg-base);
  color: var(--color-text);
  font-family: 'Inter', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
}

/* Scrollbar */
::-webkit-scrollbar { width: 8px; }
::-webkit-scrollbar-track { background: #0a0a0f; }
::-webkit-scrollbar-thumb { background: rgba(124, 58, 237, 0.4); border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: rgba(124, 58, 237, 0.7); }

/* Selection */
::selection { background: rgba(124, 58, 237, 0.3); }
```

---

## Responsive Breakpoints

```css
/* Mobile first */
--breakpoint-sm:  640px   /* large phones */
--breakpoint-md:  768px   /* tablets */
--breakpoint-lg: 1024px   /* small desktop */
--breakpoint-xl: 1280px   /* standard desktop */
```

### Key responsive changes:
- Hero: 2-col → 1-col stack at `md`
- Feature grid: 4-col → 2-col at `lg`, 1-col at `sm`
- Pricing: 2-col → 1-col at `md`
- How It Works: horizontal → vertical at `md`
- Nav: full links → hamburger menu at `md`

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Lighthouse Performance | ≥ 91 |
| Lighthouse Accessibility | 100 |
| Lighthouse Best Practices | ≥ 90 |
| Lighthouse SEO | 100 |
| LCP | < 2.5s |
| CLS | < 0.1 |
| FID | < 100ms |

Key tactics:
- Self-host Inter font (subset to Latin, weights 400/500/600/700/800/900)
- All images: WebP format, lazy loaded, explicit width/height
- Sphere: pure CSS, no image assets needed
- No third-party scripts (zero analytics JS on page)
- Svelte islands only hydrate the WishlistForm and FAQ components
