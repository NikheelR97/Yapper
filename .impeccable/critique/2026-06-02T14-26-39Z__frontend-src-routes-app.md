---
target: frontend/src/routes/(app)
total_score: 25
p0_count: 0
p1_count: 3
timestamp: 2026-06-02T14-26-39Z
slug: frontend-src-routes-app
---
# Critique: App routes (frontend/src/routes/(app))

Scope: app shell layout, channel + DM message routes, DM index, explore, settings, servers index, AppSidebar, TopNav. Browser injection skipped — the app is auth-gated behind Signal device-trust and no dev server was running. Assessment B satisfied via `detect.mjs` (clean on the routes; prior-run warnings live in chat sub-components, out of this scope).

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Presence, typing, reconnect banner, skeletons, sending states all present |
| 2 | Match System / Real World | 3 | Familiar #channel / DM / server vocabulary; "Go GoPro" copy is off |
| 3 | User Control and Freedom | 2 | DM view force-scrolls to bottom on every update; no Esc on create-server modal; mobile settings hides logout |
| 4 | Consistency and Standards | 2 | Two parallel color systems (tokens vs hardcoded hex); emoji icons vs SVG; pill buttons abandoned for rounded rects; off-brand pink gradient |
| 5 | Error Prevention | 2 | Delete-account confirm is good; "Disable Account" is a dead button; no Esc dismissal |
| 6 | Recognition Rather Than Recall | 3 | Text labels in top-nav and quick-nav; icon-only server strip is tooltipped |
| 7 | Flexibility and Efficiency | 3 | Ctrl+/ shortcuts modal, debounced search, grid/list toggle |
| 8 | Aesthetic and Minimalist Design | 3 | Clean dark shell; gradient Pro card and emoji add noise |
| 9 | Error Recovery | 2 | Channel route has a retry button; DM route's "Failed to load messages." has none |
| 10 | Help and Documentation | 2 | Shortcuts modal + teaching empty states; no contextual/searchable help |
| **Total** | | **25/40** | **Acceptable** |

## Anti-Patterns Verdict

**LLM assessment:** This does not read as AI slop. It is a real, hand-built product that passes the product slop test — a category-fluent user would trust the shell. The failures are not strangeness; they are *drift*: surfaces built at different times against different rules. The channel route is polished (scroll-anchoring, skeletons, retry, reduced-motion); the DM and settings routes look like earlier drafts of the same app.

Two genuine absolute-ban hits:
- **Side-stripe border**: `.conv-btn.active` uses `border-left: 2px solid var(--color-brand)` (AppSidebar:971) — the exact banned pattern, and inconsistent with how active state is shown everywhere else (background tint + bottom indicator).
- **Off-brand second hue**: the Pro card and button use `linear-gradient(135deg, #7c3aed, #db2777)` (settings:727,757). Pink `#db2777` is not in the palette; DESIGN.md's One-Glow Rule says violet does all identity work alone.

**Deterministic scan:** `detect.mjs --json` on `frontend/src/routes/(app)` returned `[]` — clean. The prior run's 4 warnings (bounce easing in TypingIndicator, `transition: width` in recorder/message components) are in chat sub-components outside this target.

**Visual overlays:** None. The surface is auth + device-trust gated and no dev server was available, so no reliable in-browser overlay was produced. Fallback signal: source review + CLI scan only.

## Overall Impression

The bones are strong and the dark "Lit Room" identity is real on the shell, top-nav, and channel route. The single biggest opportunity is **consistency**: a large fraction of the surface (settings entirely, DM index, parts of explore) bypasses the design tokens with hardcoded hex, uses emoji where the rest uses SVG icons, and drops the signature pill button for generic rounded rectangles. Fixing token adherence would simultaneously fix the broken light theme, several contrast failures, and the visual drift — one root cause behind three symptoms.

## What's Working

- **The channel reading experience.** Scroll-anchoring that only follows when you're already at the bottom, a "Jump to latest" affordance, skeleton loaders, an inline retry on load failure, and a `prefers-reduced-motion` branch. This is the bar the rest of the app should meet.
- **The secure-storage failure state** (+layout:1159) — a real, role="alert" error surface with Retry and Sign Out. High-stakes moment handled with reassurance, exactly what the parent/guardian audience needs.
- **Status vocabulary.** Presence dots, typing indicator, reconnecting banner, sending-disabled inputs — the "real-time feel is the product" principle shows up consistently in the shell.

## Priority Issues

### [P1] `--color-text-muted` used for primary navigation and body copy fails WCAG AA
- **Why it matters:** `#52525B` on `#0F0F0F` is ~2.5:1 — well under the 4.5:1 floor. It's used for **inactive channel names** in the sidebar (`.channel-btn`, AppSidebar:851) — that's the core navigation of the app — plus empty-state body copy (servers index `<p>`, explore `.empty-msg`/`.no-results`, DM "Offline" status). DESIGN.md explicitly says text-muted is for timestamps/disabled only, never body. Minors are in the audience; the a11y bar is not optional.
- **Fix:** Use `--color-text-secondary` (#A1A1AA, ~7.5:1) for inactive channel labels and all empty-state/body copy. Reserve `--color-text-muted` for timestamps and truly decorative text. Verify each against its actual surface step.
- **Suggested command:** `/impeccable colorize`

### [P1] Token system is bypassed across whole surfaces, breaking the light theme
- **Why it matters:** The entire settings page, the DM index, and parts of explore are written in hardcoded hex (`#0f1117`, `#9ca3af`, `#d1d5db`, `#6b7280`, `rgba(255,255,255,0.04)`…) instead of CSS variables. DESIGN.md's Light-Theme Parity Rule requires every surface to define both themes — these surfaces will render dark-on-dark or stay locked to dark when `[data-theme='light']` is active. It also guarantees the visual drift will keep widening.
- **Fix:** Replace hardcoded values with the existing tokens (`--color-bg-surface`, `--color-text-secondary`, `--color-border`, etc.). The tokens already exist and already handle light + Tauri vibrancy; the work is mechanical substitution. Audit settings, dm/+page, explore user rows.
- **Suggested command:** `/impeccable audit`

### [P1] Settings is unusable on small screens
- **Why it matters:** At `<900px` the right sidebar is `display:none` — and it's the *only* place to log out, manage/revoke devices, export data, or delete the account. Those actions vanish entirely on tablet/mobile. At `<600px` the left nav collapses to 56px and hides `.nav-label`, but the nav items have no icons — you get a 56px rail of blank, unlabeled buttons. Navigation is effectively broken. Yapper is a Capacitor mobile app; this is a primary platform.
- **Fix:** Move account/device/danger actions into the responsive flow (a section in the main column, or a sheet) rather than hiding them. Give nav items icons before collapsing to icon-only, or keep labels in a horizontal scroll/select on mobile.
- **Suggested command:** `/impeccable adapt`

### [P2] DM route force-scrolls to bottom on every update; no retry on error
- **Why it matters:** `afterUpdate` in dm/[conversationId] unconditionally sets `scrollTop = scrollHeight` (line 62-66). Reading DM history is impossible — any incoming message, presence change, or store update yanks you back down. The channel route already solved this exact problem; the DM route didn't get the fix. Same route's "Failed to load messages." has no retry, while the channel route does.
- **Fix:** Port the channel route's `measureAtBottom` / `atBottom` / "Jump to latest" pattern to the DM route, and add the same retry affordance.
- **Suggested command:** `/impeccable polish`

### [P2] Emoji icons and the pink gradient undercut the brand on safety surfaces
- **Why it matters:** Settings and the Family Controls section use emoji (📦 🚪 🗑 ⏸ 🚀 🛡 🌐 ⏱ 🔒) as the icon vocabulary, while the rest of the app uses clean stroked SVGs. Emoji render differently per OS and read as childish — directly against the "not childish / safety reads as adult trust" anti-reference, and it's the *guardian* surface using them. The Pro card's violet→pink gradient introduces an off-palette hue and a gradient button the system doesn't have.
- **Fix:** Replace emoji with the same SVG icon set used elsewhere. Drop the pink; use the single violet (`--color-brand`) flat or with the sanctioned brand glow. Restore pill (`--radius-full`) buttons to match the system signature.
- **Suggested command:** `/impeccable colorize`

## Persona Red Flags

**Sam (Accessibility-Dependent):** Inactive channel names and empty-state copy sit at ~2.5:1 contrast — unreadable for low vision. The DM "Offline" status is muted-on-dark. Active conversation is signalled partly by a color-only left border. Settings devices show trust state as raw text ("trusted"/"pending_trust") with no non-color cue but at least it's text.

**Casey (Distracted Mobile):** Can't log out, revoke a device, or delete the account on a phone — the sidebar holding those is `display:none` under 900px. Under 600px the settings nav is a column of blank unlabeled buttons. The DM auto-scroll means returning to a conversation after an interruption snaps away from where they were reading.

**Alex (Power User):** Create-server modal has no `Esc` to dismiss (backdrop-click only) and no focus trap. Active-state nav uses `aria-current` correctly, good — but the "Disable Account" button has no handler and does nothing, which an exploring power user will hit immediately.

**Guardian / Parent (project persona, from PRODUCT.md):** The Family Controls surface — the one screen meant to read as adult, trustworthy safety — leads with emoji (🛡 🌐 ⏱ 🔒) and rounded-rect buttons rather than the product's pill+SVG language. It undersells the "safety reads as trust" principle precisely where it matters most.

## Minor Observations

- Create-server modal uses `z-index: 100`; DESIGN.md's semantic z-scale puts modal at 300. The reconnecting banner is also 100. Adopt the named scale.
- `.section-title` eyebrows in explore (uppercase, 0.07em tracking, 11–12px, muted) are borderline the AI-eyebrow trope; acceptable as section headers in product context but watch the cadence.
- "Go GoPro" reads oddly (GoPro is a camera brand). Likely meant "Go Pro" / "Upgrade to Premium".
- Server strip is icon-only; tooltips cover recognition, but consider the active server's name being visible without hover.
- `.e2ee-badge` "⚠ Verify" mixes an emoji-ish glyph into an otherwise SVG-iconed header.

## Questions to Consider

- What if the channel route's scroll + retry + skeleton pattern were extracted into one shared message-view component, so DM and channel literally cannot drift again?
- Does the settings page need three columns at all, or would a single scrollable column with sectioned groups serve every breakpoint better and kill the mobile-hidden-actions bug?
- If violet is the only brand color, what is the pink gradient earning that a violet glow wouldn't?
- What would the Family Controls screen look like if it were the *most* adult-legible surface in the app instead of the most emoji-heavy?
