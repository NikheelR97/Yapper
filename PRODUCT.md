# Product

## Register

product

> Yapper has two surfaces: the **app** (SvelteKit PWA + Tauri desktop + Capacitor
> mobile) and the **marketing site** (Astro, `yapperhq.com`). The default register
> is `product` because the app is the core of the experience. When working on the
> marketing site, override to `brand` for that task.

## Users

People who want group and 1:1 chat the way Discord does it — servers, channels,
DMs, voice/video, live canvas widgets — but without trusting the operator with
their plaintext. Two notable cohorts:

- **Everyday social users** moving communities over from Discord/Slack. They are
  on the app for long sessions, often on multiple devices, switching between
  servers and DMs while multitasking.
- **Minors and their parents** (COPPA flows). Kids use the same app surface;
  parents interact with approval/screen-time controls. The product must feel
  safe and legible to a guardian without feeling like a kids-only app.

The job to be done: keep up with my people in real time, privately, across all
my devices, without friction.

## Product Purpose

Yapper is an end-to-end-encrypted, Discord-like communication platform.
Every DM and channel message is encrypted on-device (Signal Protocol: X3DH +
Double Ratchet for DMs, Sender Keys for channels); the server stores only
ciphertext. It ships as web PWA, Tauri desktop, and Capacitor mobile, and is
deployable for $0/month.

Success looks like: real-time chat that feels as fluid and fun as the
mainstream alternatives, where privacy is the default rather than a setting, and
where the encryption never shows up as friction in the user's path.

## Brand Personality

Playful, private, modern. The voice is social and a little irreverent ("A New
Way to Yap"), but it is anchored by genuine privacy rather than performing it.
Energetic without being noisy; confident without lecturing about security.
Emotional goals: belonging and ease for everyday users, reassurance and control
for parents.

## Anti-references

- **Generic Discord clone.** Shares the feature set (servers, channels, presence)
  but must not read as a literal copy of Discord's chrome, layout, or blurple.
  Earn the conventions; don't inherit the skin.
- **Crypto / web3 aesthetic.** Despite the E2EE core, avoid the neon-on-black,
  "blockchain trust" visual cliché. Privacy is communicated through clarity and
  restraint, not lock icons and circuit-board gradients.
- **Childish / kiddie.** COPPA controls exist, but the product is for everyone.
  No primary-color mascots or playground styling; safety should read as
  trustworthy and adult-legible.

## Design Principles

- **Privacy is felt, not announced.** The encrypted-by-default reality should
  show up as calm confidence and zero friction, not as badges and warnings.
- **Real-time feel is the product.** Typing, presence, read receipts, and message
  delivery must feel instantaneous and alive; latency or jank breaks the core
  promise more than any single feature.
- **One identity across many devices.** Web, desktop, and mobile are the same
  product; layouts and interactions should adapt without feeling like three
  different apps.
- **Safety reads as trust, for two audiences at once.** Parental and safety
  surfaces must be legible and reassuring to a guardian while staying out of the
  way of the everyday user.
- **Playful, but never at the cost of legibility.** Personality lives in motion,
  copy, and accents — not in anything that compromises reading a long, dense
  conversation.

## Accessibility & Inclusion

Target WCAG 2.2 AA across both surfaces: ≥4.5:1 contrast for body text (≥3:1 for
large text), full keyboard operability, visible focus, and accessible names for
interactive controls. Honor `prefers-reduced-motion` with a non-motion
alternative for every animation. Because minors are part of the audience, hold
this bar consistently rather than treating it as optional polish.
