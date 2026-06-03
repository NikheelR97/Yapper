---
target: app shell + channel route (frontend/src/routes/(app))
total_score: 26
p0_count: 0
p1_count: 3
timestamp: 2026-06-02T13-57-13Z
slug: frontend-src-routes-app
---
# Critique: App shell + channel route (frontend/src/routes/(app))

Browser injection skipped: app is auth-gated behind Signal device-trust, no dev server running. Assessment B satisfied via detect.mjs.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Presence/typing/reconnect present; loading is plain text not skeletons |
| 2 | Match System / Real World | 3 | Familiar #channels/DMs/servers vocabulary |
| 3 | User Control and Freedom | 2 | afterUpdate force-scrolls to bottom on every update; can't read history |
| 4 | Consistency and Standards | 3 | ARIA tab roles misused for nav links, inline styles, hardcoded hex |
| 5 | Error Prevention | 2 | Join disabled when empty; little else in scope |
| 6 | Recognition Rather Than Recall | 3 | Top-nav text-labeled; server strip icon-only but tooltipped |
| 7 | Flexibility and Efficiency | 3 | Ctrl+/ shortcuts modal, keyboard shortcuts |
| 8 | Aesthetic and Minimalist Design | 3 | Clean dark shell; always-visible join-invite form adds clutter |
| 9 | Error Recovery | 2 | "Failed to load messages." has no retry; secure-store error done right |
| 10 | Help and Documentation | 2 | Shortcuts modal + some teaching empty states; no contextual help |
| Total | | 26/40 | Acceptable |

## Anti-Patterns Verdict
Does not look AI-generated; real hand-built product, passes product slop test. detect.mjs: clean on routes; 4 warnings in chat components — bounce-easing TypingIndicator:68; layout-transition (transition: width) in ClipRecorder:310, YapRecorder:312, YapMessage:187 (likely audio progress bars). No browser overlay (auth-gated, no server).

## What's Working
1. Real-time status first-class (presence, typing, reconnect banner, read receipts).
2. Secure-storage failure screen is a model error state (role=alert, plain language, Retry/Sign Out).
3. Restrained legible dark shell; aria-current=page, role=list, real buttons.

## Priority Issues
- [P1] Forced auto-scroll destroys history reading. channel/+page.svelte:79-81 afterUpdate sets scrollTop=scrollHeight on every update. Fix: only auto-scroll if near bottom + on initial load. -> /impeccable harden
- [P1] Profile avatar is non-keyboard, non-SR control. TopNav.svelte:89-105 div on:click with a11y warning suppressed. Fix: make it a button/anchor. -> /impeccable audit
- [P1] No recovery on message load failure. channel/+page.svelte:138 dead-end text + hardcoded #fca5a5. Fix: add Try again calling prepareAndLoad, use --color-error. -> /impeccable harden
- [P2] ARIA tab roles on navigation links. TopNav.svelte:50-61 role=tablist/tab/aria-selected on route links. Fix: drop roles, use nav + aria-current=page. -> /impeccable audit
- [P2] Loading states are bare text not skeletons (Setting up encryption…, Loading…). Product register wants skeletons. -> /impeccable harden or onboard

## Persona Red Flags
- Sam: can't tab to profile avatar; nav announces as tab but behaves as links; auto-scroll fights SR review.
- Riley: scroll up yanked to bottom on presence/typing; message-load failure has no exit; empty channel no create path.
- Casey: Live Canvas defaults open (showCanvas=true), 360px competes with chat on narrow viewport first paint.

## Minor Observations
- Inline style in AppSidebar.svelte:214.
- TopNav logo is generic circle-and-dot, not the signature Sphere.
- Reconnecting banner hardcoded #b45309 instead of --color-warning.
- Channel join-by-invite form pinned to bottom of every channel panel; competes with channel list.
