# S2 — Authentication & Onboarding UI Spec

**Sprint:** S2 (W5–W6)
**Screens:** Sign In (7), Sign Up (8), Onboarding 1 (9), Onboarding 2 (10)
**Routes:** `(auth)/login`, `(auth)/register`, `(auth)/onboarding`

---

## Screen 7 — Sign In

**File:** `frontend/src/routes/(auth)/login/+page.svelte`

### Layout

```
┌─────────────────────────────────────────────────────────┐
│                 [sphere icon 40px] Yapper                │
│                                                          │
│         ┌────────────────────────────────────┐          │
│         │                                    │          │
│         │     Welcome back.                  │          │
│         │     Sign in to your account        │          │
│         │                                    │          │
│         │  [Discord]  [Google]  [Apple]       │          │
│         │                                    │          │
│         │  ─────── or continue with ───────  │          │
│         │                                    │          │
│         │  Email address                     │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │ alice@example.com            │  │          │
│         │  └──────────────────────────────┘  │          │
│         │                                    │          │
│         │  Password                          │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │ ••••••••••          [👁]     │  │          │
│         │  └──────────────────────────────┘  │          │
│         │                                    │          │
│         │           Forgot password?          │          │
│         │                                    │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │     Enter the Void →         │  │          │
│         │  └──────────────────────────────┘  │          │
│         │                                    │          │
│         │  Don't have an account? Sign up    │          │
│         └────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────┘
```

### Specs

| Element | Spec |
|---------|------|
| Page background | `#0a0a0f` with subtle radial glow: `radial-gradient(ellipse at 50% 0%, rgba(124,58,237,0.12) 0%, transparent 60%)` |
| Logo | Sphere icon 40px + "Yapper" Inter 700, centered at top |
| Card | Glass card, max-width 420px, padding 40px, centered vertically and horizontally |
| Heading | "Welcome back." — Inter 800, 28px, #f9fafb |
| Subheading | Inter 400, 15px, #9ca3af |
| Social buttons | 3-column row, each: 44px height, glass style, border 1px solid rgba(255,255,255,0.1), Inter 600, 14px, icon 18px |
| Discord button | Icon: discord SVG (blurple `#5865F2` icon on dark bg) |
| Google button | Icon: Google multicolor logo |
| Apple button | Icon: Apple logo (#f9fafb on dark) |
| Divider | "or continue with" — 1px solid rgba(255,255,255,0.08), text Inter 400 12px #6b7280 |
| Field label | Inter 500, 13px, #9ca3af, letter-spacing 0.05em, uppercase |
| Input | Standard input style (see design tokens) |
| Password visibility toggle | Eye icon 18px, button in input suffix, #6b7280 |
| Forgot password | Inter 400, 13px, #7c3aed, text-right, underline on hover |
| CTA button | "Enter the Void →" — full-width, primary purple, 48px height, Inter 700, 16px |
| Sign up link | Inter 400, 14px, #6b7280 — "Don't have an account? **Sign up**" (bold link #a78bfa) |
| Error state | Red input border + error message below field, #ef4444, 13px |
| Loading state | Button shows spinner, inputs disabled |

### Rate Limit UI
After 5 failed attempts: show lockout banner above form:
```
⚠️ Too many attempts. Try again in 14:32.
```
- Background: rgba(239,68,68,0.1), border-left 3px #ef4444
- Countdown timer in red (live)
- Inputs remain disabled during lockout

---

## Screen 8 — Sign Up

**File:** `frontend/src/routes/(auth)/register/+page.svelte`

### Layout

```
┌─────────────────────────────────────────────────────────┐
│                 [sphere icon 40px] Yapper                │
│                                                          │
│         ┌────────────────────────────────────┐          │
│         │                                    │          │
│         │     Create your account.           │          │
│         │     Join the Yapper community      │          │
│         │                                    │          │
│         │  [Discord]  [Google]  [Apple]       │          │
│         │  ─────── or continue with ───────  │          │
│         │                                    │          │
│         │  Display Name                      │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │ e.g. CosmicVibes             │  │          │
│         │  └──────────────────────────────┘  │          │
│         │                                    │          │
│         │  Username  (@ handle)              │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │ @                            │  │          │
│         │  └──────────────────────────────┘  │          │
│         │  ✓ Available                        │          │
│         │                                    │          │
│         │  Email address                     │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │                              │  │          │
│         │  └──────────────────────────────┘  │          │
│         │                                    │          │
│         │  Password                          │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │ ••••••••••          [👁]     │  │          │
│         │  └──────────────────────────────┘  │          │
│         │  [████████████░░░░]  Strong         │          │
│         │                                    │          │
│         │  ┌──────────────────────────────┐  │          │
│         │  │       Join the Hype →        │  │          │
│         │  └──────────────────────────────┘  │          │
│         │                                    │          │
│         │  Already have an account? Sign in  │          │
│         └────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────┘
```

### Specs

| Element | Spec |
|---------|------|
| Card max-width | 440px |
| Heading | "Create your account." — Inter 800, 28px |
| Username field | Prefix "@" icon or text, debounced availability check (300ms), shows ✓ available / ✗ taken |
| Username validation | Live: lowercase letters, numbers, underscore only; 3-20 chars |
| Password strength meter | Bar below password field, 4-segment: Weak / Fair / Good / Strong |
| Strength colors | Weak: #ef4444, Fair: #f59e0b, Good: #22c55e, Strong: #7c3aed |
| Strength bar segments | 4 rounded pills in a row, gray → filled as strength increases |
| Strength label | Inter 500, 12px, matching color |
| CTA | "Join the Hype →" — full-width primary purple |
| T&C | Small text below CTA: "By signing up you agree to our Terms and Privacy Policy" — 12px, #6b7280 |

### Username availability states:
- Checking: spinner 12px, "Checking..." gray
- Available: `check-circle` 14px #22c55e, "@alice is available"
- Taken: `x-circle` 14px #ef4444, "Username is taken"
- Too short: gray, "Must be at least 3 characters"

### Post-Register Flow:
After successful register → show inline confirmation:
```
┌──────────────────────────────────────────────────────┐
│  📧  Check your inbox!                               │
│  We sent a verification link to alice@example.com    │
│  [Resend email]                    [Continue →]      │
└──────────────────────────────────────────────────────┘
```

---

## Screen 9 — Onboarding Step 1: "A New Way to Yap"

**File:** `frontend/src/routes/(auth)/onboarding/+page.svelte` (step 1)

### Layout

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│                                                          │
│                    ╭──────────────╮                      │
│                    │    SPHERE    │                      │
│                    │  280px dia.  │                      │
│                    │  animated    │                      │
│                    ╰──────────────╯                      │
│                    [glow ring]                           │
│                                                          │
│                  A New Way to Yap.                       │
│                                                          │
│           End-to-end encrypted. Private.                 │
│               For everyone.                              │
│                                                          │
│                   ● ○ ○ ○                                │
│                                                          │
│              [Get Started →]                             │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Specs

| Element | Spec |
|---------|------|
| Background | Full-screen `#0a0a0f` with centered radial glow `rgba(124,58,237,0.15)` |
| Sphere | 280px, CSS sphere (radial-gradient, see design tokens), centered |
| Sphere animation | Pulse glow + slow Y-axis rotation via `perspective(600px) rotateY()` CSS animation, 8s loop |
| Sphere shadow | `filter: drop-shadow(0 0 40px rgba(124,58,237,0.5))` |
| Headline | "A New Way to Yap." — Inter 900, 40px, #f9fafb, text-center, tracking-tight |
| Subheadline | Inter 400, 18px, #9ca3af, text-center, max-width 320px |
| Dot pagination | 4 dots — active: 32px wide pill (brand purple), inactive: 8px circle (rgba(255,255,255,0.2)) |
| CTA | "Get Started →" primary purple pill, 52px height, 200px min-width |
| Spacing | Sphere: margin-bottom 48px; headline: margin-bottom 16px; dots: margin-top 40px; CTA: margin-top 24px |

---

## Screen 10 — Onboarding Step 2: Community Discovery

**File:** `frontend/src/routes/(auth)/onboarding/+page.svelte` (step 2)

### Layout

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│              Find Your Hype.                             │
│          Discover communities that match you.            │
│                                                          │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│   │  🎮 Gaming   │  │  🎵 Music    │  │  💻 Tech     │  │
│   │  1.2k online │  │  892 online  │  │  430 online  │  │
│   │  [Join]      │  │  [Join]      │  │  [Join]      │  │
│   └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│   │  🎨 Art      │  │  📚 Study    │  │  🏆 Sports   │  │
│   │  228 online  │  │  156 online  │  │  344 online  │  │
│   └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│                   ● ● ○ ○                                │
│                                                          │
│     [Skip]                   [Ready to Yap →]           │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Specs

| Element | Spec |
|---------|------|
| Heading | "Find Your Hype." — Inter 800, 36px, #f9fafb, text-center |
| Subheading | Inter 400, 16px, #9ca3af |
| Community cards | Responsive 3-column grid (2-col mobile), 160px height each |
| Card style | Glass card, border: 1px rgba(255,255,255,0.08), rounded-xl |
| Card icon | 32px emoji or icon, margin-bottom 8px |
| Card name | Inter 700, 16px, #f9fafb |
| Card online count | Inter 400, 13px, #9ca3af, "X online" |
| Join button | Appears on hover or as small chip: "Join", 28px height, brand purple, pill |
| Selected state | Border changes to `rgba(124,58,237,0.5)`, background `rgba(124,58,237,0.1)` |
| Dot pagination | Same as Screen 9 but step 2 lit |
| Skip | Text button, #6b7280, font-size 14px, left-aligned |
| CTA | "Ready to Yap →" full-width purple, 52px |

---

## Onboarding Step 3 — PIN Setup (Key Backup)

**File:** Inline in `onboarding/+page.svelte` step 3

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│              🔐  Secure Your Messages.                   │
│                                                          │
│   Your E2EE keys live only on your device.              │
│   Set a 6-digit PIN to back them up securely.           │
│                                                          │
│        ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐             │
│        │ 1 │ │ 2 │ │ • │ │ • │ │ • │ │ • │             │
│        └───┘ └───┘ └───┘ └───┘ └───┘ └───┘             │
│                                                          │
│    ⚠️  If you lose your device and forget this PIN,     │
│       your message history cannot be recovered.         │
│                                                          │
│                   ● ● ● ○                                │
│                                                          │
│     [Skip for now]          [Set PIN & Continue →]      │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---------|------|
| PIN boxes | 6 input boxes, 52px × 64px each, centered, gap 12px |
| Box style | Background rgba(255,255,255,0.06), border 1px rgba(255,255,255,0.12), border-radius 8px |
| Active box | Border: 2px #7c3aed, box-shadow: 0 0 0 3px rgba(124,58,237,0.2) |
| Filled box | Shows dot (●) not digit |
| Warning | Amber icon + italic text, #f59e0b |
| Skip | Small text link, #6b7280 |
| Number pad | Virtual numpad on mobile (auto) |

---

## Layout Route: (auth)

**File:** `frontend/src/routes/(auth)/+layout.svelte`

- Applies to all unauthenticated screens
- Background: `#0a0a0f`
- Centered content with sphere glow background effect
- Redirect to `(app)/explore` if already authenticated

```svelte
<script>
  import { auth } from '$lib/stores/auth';
  import { goto } from '$app/navigation';

  if ($auth.user) goto('/explore');
</script>

<div class="auth-root">
  <div class="auth-bg-glow" />
  <slot />
</div>
```

```css
.auth-root {
  min-height: 100vh;
  background: #0a0a0f;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px;
  position: relative;
  overflow: hidden;
}

.auth-bg-glow {
  position: absolute;
  top: -200px; left: 50%;
  transform: translateX(-50%);
  width: 600px; height: 600px;
  background: radial-gradient(circle, rgba(124,58,237,0.12) 0%, transparent 70%);
  pointer-events: none;
}
```

---

## OAuth Callback Loading State

**File:** `frontend/src/routes/(auth)/oauth-callback/+page.svelte`

Shown while the OAuth redirect is being processed:
```
┌──────────────────────────────────────┐
│                                      │
│    [spinning sphere animation]        │
│                                      │
│    Signing you in...                 │
│                                      │
└──────────────────────────────────────┘
```

- Mini sphere (80px) with CSS spin animation
- Text: Inter 400, 16px, #9ca3af
- Handles error state: "Sign in failed. [Try again]"

---

## Forgot Password Flow

**File:** `frontend/src/routes/(auth)/forgot-password/+page.svelte`

Step 1 — Enter email:
```
Enter your email address and we'll send
you a link to reset your password.

[Email input]
[Send reset link →]
```

Step 2 — Confirmation:
```
📧 Check your inbox!
We sent reset instructions to alice@example.com
[Back to Sign In]
```

Step 3 — New password form (from email link):
```
New password
[Password input + strength meter]
Confirm password
[Password input]
[Reset Password →]
```
