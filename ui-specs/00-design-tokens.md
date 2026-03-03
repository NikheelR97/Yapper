# Yapper Design System — Tokens & Foundations

> This file is the single source of truth for all visual design values used across the Yapper app.
> Every component spec in this directory references these tokens.

---

## Color Palette

### Brand Purples
```css
--color-brand-50:  #f5f3ff   /* near-white tint */
--color-brand-100: #ede9fe
--color-brand-200: #ddd6fe
--color-brand-300: #c4b5fd   /* sphere highlight — specular */
--color-brand-400: #a78bfa
--color-brand-500: #8b5cf6
--color-brand-600: #7c3aed   /* PRIMARY BRAND — buttons, accents, links */
--color-brand-700: #6d28d9
--color-brand-800: #5b21b6
--color-brand-900: #4c1d95
--color-brand-950: #2e1065   /* sphere deep shadow */
```

### Backgrounds
```css
--color-bg-base:       #0a0a0f   /* page/app root background */
--color-bg-surface:    #0f1117   /* card, panel, sidebar background */
--color-bg-elevated:   #16171e   /* hover states, input backgrounds */
--color-bg-overlay:    #1c1d26   /* modal backdrop content */
--color-bg-glass:      rgba(255, 255, 255, 0.04)   /* glassmorphic card fill */
--color-bg-glass-hover: rgba(255, 255, 255, 0.07)
```

### Text
```css
--color-text-primary:   #f9fafb   /* headings, body text */
--color-text-secondary: #9ca3af   /* subtitles, labels, placeholders */
--color-text-muted:     #6b7280   /* timestamps, inactive nav */
--color-text-disabled:  #4b5563
--color-text-brand:     #a78bfa   /* links, interactive labels */
```

### Borders
```css
--color-border-default:  rgba(255, 255, 255, 0.08)
--color-border-subtle:   rgba(255, 255, 255, 0.05)
--color-border-strong:   rgba(255, 255, 255, 0.15)
--color-border-brand:    rgba(124, 58, 237, 0.5)
```

### Status Colors
```css
--color-success:   #22c55e   /* online indicator, approval */
--color-warning:   #f59e0b   /* pending, away status */
--color-error:     #ef4444   /* danger zone, decline, error */
--color-info:      #3b82f6   /* info alerts, links */
--color-live:      #ef4444   /* LIVE badge */
```

### Semantic Usage
```css
/* Parental / Safety theme (dark teal-green) */
--color-safety-bg:      #1a2e1a   /* safety toggle card background */
--color-safety-icon-bg: #2d4a2d   /* safety icon circle */

/* Premium / GoPro gradient */
--color-premium-start: #7c3aed
--color-premium-end:   #c4b5fd
```

---

## Typography

### Font Stack
```css
--font-family-base: 'Inter', 'Segoe UI', system-ui, -apple-system, sans-serif;
--font-family-mono: 'JetBrains Mono', 'Fira Code', monospace;
```

### Scale
| Token | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| `--text-xs`  | 11px | 400 | 1.4 | Timestamps, badges |
| `--text-sm`  | 13px | 400 | 1.5 | Secondary labels, captions |
| `--text-base`| 15px | 400 | 1.6 | Body text, messages |
| `--text-md`  | 16px | 500 | 1.5 | Nav items, form labels |
| `--text-lg`  | 18px | 600 | 1.4 | Card titles, section headers |
| `--text-xl`  | 22px | 700 | 1.3 | Page section titles |
| `--text-2xl` | 28px | 800 | 1.2 | Page headings |
| `--text-3xl` | 36px | 800 | 1.1 | Hero headings |
| `--text-4xl` | 48px | 900 | 1.0 | Marketing hero |

### Letter Spacing
```css
--tracking-tight:  -0.02em   /* large headings (brand style) */
--tracking-normal:  0em
--tracking-wide:    0.05em   /* caps labels, tags */
--tracking-wider:   0.1em    /* button text, all-caps labels */
```

---

## Spacing Scale (4px base unit)

```css
--space-1:   4px
--space-2:   8px
--space-3:  12px
--space-4:  16px
--space-5:  20px
--space-6:  24px
--space-8:  32px
--space-10: 40px
--space-12: 48px
--space-16: 64px
--space-20: 80px
--space-24: 96px
```

---

## Border Radius

```css
--radius-sm:   6px    /* small badges, chips */
--radius-md:   8px    /* inputs, small cards */
--radius-lg:  12px    /* cards, panels */
--radius-xl:  16px    /* modal dialogs, large cards */
--radius-2xl: 24px    /* modals, bottom sheets */
--radius-full: 9999px /* pills, avatars, toggles */
```

---

## Shadows & Glows

```css
/* Card drop shadows */
--shadow-sm:  0 1px 3px rgba(0, 0, 0, 0.4);
--shadow-md:  0 4px 12px rgba(0, 0, 0, 0.5);
--shadow-lg:  0 8px 24px rgba(0, 0, 0, 0.6);

/* Brand glow (purple) */
--glow-brand-sm:  0 0 12px rgba(124, 58, 237, 0.3);
--glow-brand-md:  0 0 24px rgba(124, 58, 237, 0.4);
--glow-brand-lg:  0 0 48px rgba(124, 58, 237, 0.5);

/* Sphere-specific (see logo-icon.svg) */
--sphere-gradient: radial-gradient(circle at 35% 35%, #c4b5fd 0%, #7c3aed 45%, #2e1065 100%);
```

---

## Glassmorphism

Used on cards, modals, and floating panels:
```css
.glass-card {
  background: var(--color-bg-glass);
  backdrop-filter: blur(12px) saturate(180%);
  -webkit-backdrop-filter: blur(12px) saturate(180%);
  border: 1px solid var(--color-border-default);
  border-radius: var(--radius-lg);
}

.glass-card:hover {
  background: var(--color-bg-glass-hover);
  border-color: var(--color-border-strong);
}
```

---

## Component Patterns

### Buttons

**Primary (brand purple):**
```css
background: #7c3aed;
color: #ffffff;
border-radius: var(--radius-full);
padding: 10px 24px;
font-weight: 700;
font-size: 15px;
transition: background 0.15s, transform 0.1s;

&:hover { background: #6d28d9; transform: translateY(-1px); }
&:active { transform: translateY(0); }
```

**Secondary (outlined):**
```css
background: transparent;
color: var(--color-text-primary);
border: 1px solid var(--color-border-strong);
border-radius: var(--radius-full);
padding: 10px 24px;

&:hover { background: var(--color-bg-elevated); }
```

**Destructive:**
```css
background: rgba(239, 68, 68, 0.1);
color: #ef4444;
border: 1px solid rgba(239, 68, 68, 0.3);
```

### Inputs

```css
.input {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-default);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  font-size: 15px;
  padding: 12px 16px;
  width: 100%;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.input:focus {
  outline: none;
  border-color: var(--color-brand-600);
  box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.15);
}

.input::placeholder {
  color: var(--color-text-muted);
}
```

### Tags / Chips

```css
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-default);
  border-radius: var(--radius-full);
  font-size: 13px;
  color: var(--color-text-secondary);
  cursor: pointer;
  white-space: nowrap;
}

.chip:hover {
  background: var(--color-bg-glass-hover);
  border-color: var(--color-border-brand);
  color: var(--color-text-brand);
}
```

### Avatars

```css
/* Sizes */
--avatar-xs:  24px;
--avatar-sm:  32px;
--avatar-md:  40px;
--avatar-lg:  56px;
--avatar-xl:  80px;
--avatar-2xl: 112px;

/* Style */
.avatar {
  border-radius: 50%;
  object-fit: cover;
  flex-shrink: 0;
}

/* Status dot */
.avatar-status-online  { background: var(--color-success); }
.avatar-status-away    { background: var(--color-warning); }
.avatar-status-offline { background: var(--color-text-muted); }
```

### Toggle Switch

```css
/* Used in Safety Gates and Settings */
.toggle {
  width: 44px; height: 24px;
  background: var(--color-bg-elevated);
  border-radius: 12px;
  position: relative;
  border: 1px solid var(--color-border-default);
  cursor: pointer;
  transition: background 0.2s;
}

.toggle.on {
  background: var(--color-brand-600);
  border-color: transparent;
}

.toggle-thumb {
  width: 18px; height: 18px;
  background: #fff;
  border-radius: 50%;
  position: absolute;
  top: 2px; left: 2px;
  transition: transform 0.2s;
}

.toggle.on .toggle-thumb {
  transform: translateX(20px);
}
```

### Badge / Pill

```css
.badge-live {
  background: #ef4444;
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  text-transform: uppercase;
}

.badge-new {
  background: var(--color-brand-600);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  text-transform: uppercase;
}
```

---

## Navigation / Layout

### App Shell (authenticated)

```
┌─────────────────────────────────────────────────────┐
│  TOPNAV  (56px height, sticky)                      │
├──────┬──────────────────────────────────────────────┤
│      │                                              │
│SIDE  │          MAIN CONTENT AREA                  │
│BAR   │                                              │
│(240px│                                              │
│fixed)│                                              │
└──────┴──────────────────────────────────────────────┘
```

### Top Navigation Heights
- Main app topnav: **56px**
- Marketing site header: **64px**

### Sidebar Widths
- App server sidebar: **240px**
- Settings sidebar: **220px**
- Parental sidebar: **260px**
- Live Canvas panel: **360px** (right-side, toggleable)

### Z-Index Scale
```css
--z-base:    0
--z-raised:  10
--z-dropdown: 100
--z-sticky:   200
--z-modal:    300
--z-toast:    400
--z-tooltip:  500
```

---

## Animation

```css
/* Duration tokens */
--duration-fast:   100ms
--duration-base:   200ms
--duration-slow:   300ms
--duration-slower: 500ms

/* Easing */
--ease-out:     cubic-bezier(0.0, 0, 0.2, 1)
--ease-in:      cubic-bezier(0.4, 0, 1, 1)
--ease-in-out:  cubic-bezier(0.4, 0, 0.2, 1)
--ease-spring:  cubic-bezier(0.34, 1.56, 0.64, 1)   /* for buttons, bouncy */
```

### Standard Transitions
- Button hover: `background 150ms ease-out, transform 100ms ease-spring`
- Card hover: `background 150ms ease-out, border-color 150ms ease-out`
- Input focus: `border-color 150ms ease-out, box-shadow 150ms ease-out`
- Sidebar slide: `transform 250ms ease-out`
- Modal open: `opacity 200ms ease-out, transform 250ms ease-spring`

---

## The Sphere (Brand Mark)

The Yapper brand identity is built around the 3D purple sphere — used in:
- App icon (`logo-icon.svg`)
- Onboarding Screen 1 (animated, large)
- Marketing site hero (animated CSS version)

```css
/* Sphere gradient — mirrors logo-icon.svg */
.sphere {
  background: radial-gradient(circle at 35% 35%,
    #c4b5fd 0%,
    #7c3aed 45%,
    #2e1065 100%
  );
  border-radius: 50%;
  box-shadow:
    0 0 40px rgba(124, 58, 237, 0.4),
    inset 0 -20px 40px rgba(46, 16, 101, 0.5);
}

/* Specular highlight */
.sphere::before {
  content: '';
  position: absolute;
  top: 15%; left: 20%;
  width: 35%; height: 35%;
  background: radial-gradient(circle, rgba(255,255,255,0.25) 0%, transparent 100%);
  border-radius: 50%;
}

/* Orbital ring (decorative) */
.sphere-ring {
  border: 1.5px solid rgba(167, 139, 250, 0.12);
  border-radius: 50%;
  position: absolute;
  transform: rotate(-25deg);
}
```

---

## Iconography

Icon set: **Lucide Icons** (MIT license, Svelte-friendly)
Stroke width: **1.5px** (default), **2px** for small sizes (<16px)
Size scale: 14, 16, 18, 20, 24px

Common icons used in Yapper:
| Context | Icon name |
|---------|-----------|
| Yap (audio) | `mic`, `waveform` (custom SVG) |
| Clip (video) | `video` |
| Send | `send` |
| Explore | `compass` |
| Settings | `settings` |
| Profile | `user` |
| Safety / Shield | `shield` |
| Lock (E2EE) | `lock` |
| Server | `hash` (channel), `server` |
| Follow | `user-plus` |
| Hype / Pin | `zap`, `pin` |
| Live Canvas | `layout` |
| Online status | filled circle |
| Premium (GoPro) | `rocket` |
