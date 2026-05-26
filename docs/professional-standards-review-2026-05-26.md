# Yapper Professional Standards Review

Date: 2026-05-26
Branch: `audit/application-review-report`
Baseline: clean `origin/main`, after merging Phase 1-3 audit PRs

## Scope

This review is a local-first professional standards pass across repository structure, documentation, tests, CI configuration, dependency posture, git hygiene, security signals, and UI/accessibility behavior. It is not a production penetration test and does not include deploys, production migrations, secret rotation, or broad folder restructuring.

## Current Project Structure

The repository is broad but understandable:

- `backend/`: Rust 1.80 MSRV Axum API, SQLx migrations, integration tests, Fly config, and tracked SQLx offline cache.
- `frontend/`: SvelteKit app, Playwright E2E suite under `frontend/tests`, Vitest unit tests next to source, Tauri v2 shell, and Capacitor platforms.
- `marketing/`: Astro marketing site.
- `.github/workflows/`: CI, E2E smoke/nightly/security, CodeQL, Neon storage, and desktop release workflows.
- `docs/`, `wiki/`, `ui-specs/`, `dev docs/`: public docs, wiki source, UI specs, and internal handover/audit notes.
- `scripts/`: local setup and runner maintenance utilities.

The high-level layout matches the stack and domain boundaries. No broad move is recommended. The main maintainability issue is documentation drift and the current `dev docs/` ignore pattern, not source organization.

## Phase PR Status

| PR | Branch | Status | Purpose |
|---|---|---|---|
| #128 | `audit/docs-git-hygiene` | Merged | Align docs and `.gitignore`; add tracked Tauri lockfile; trim stale cargo audit ignores. |
| #129 | `audit/e2e-ci-hygiene` | Merged | Replace noisy E2E `console.log` diagnostics with `console.debug`. |
| #130 | `audit/ui-accessibility-polish` | Merged | Fix Canvas modal accessibility warnings. |

Latest known `main` checks before these PRs were green: CI run `26446798043`, CodeQL run `26446797096`, E2E Security run `26446798045`, and E2E Smoke run `26446798015`.

## Folder Structure Findings

- Source directories are conventional for the chosen frameworks: Rust modules live under `backend/src`, SvelteKit route groups under `frontend/src/routes`, reusable frontend code under `frontend/src/lib`, E2E tests under `frontend/tests`, and migrations under `backend/migrations`.
- `frontend/tests` is large but organized with specs, helpers, fixtures, and page objects. No low-risk folder move is justified right now.
- `dev docs/` contains formal tracked docs and untracked local planning docs. `.gitignore` now re-includes the formal tracked docs so `git ls-files -ci --exclude-standard` is clean.
- `backend/.sqlx` is intentionally tracked for SQLx offline checks, and `.gitignore` now documents and re-includes it.
- Generated and local artifacts are ignored: `.svelte-kit`, build output, `node_modules`, logs, Playwright auth state, runner caches, Tauri/Capacitor build outputs, local envs, and local assistant state.

## Documentation Findings

- `README.md`, `docs/development.md`, and wiki docs now align the Rust guidance, current migration count, and infrastructure posture with the repository baseline. Evidence: `backend/Cargo.toml` has `rust-version = "1.80"`, and `backend/migrations` currently contains 37 SQL files.
- Release automation is correctly documented in `.github/workflows/ci.yml` as paused during stabilization. The deploy jobs are commented out and gated by backend/frontend/security checks plus production environment approval when re-enabled.
- E2EE implementation docs and frontend dependencies support the pure TypeScript `@noble/*` implementation (`frontend/package.json`, `frontend/src/lib/signal/*`).

## Code Quality And Configuration Findings

- CI is reasonably structured and now separates heavy work: backend, frontend, marketing, repository security scans, and self-hosted E2E smoke.
- `frontend/playwright.config.ts` uses `fullyParallel: true`, CI workers set to 4, and PR smoke is sharded in `.github/workflows/e2e-pr-smoke.yml`.
- `frontend/package.json` provides lint, check, test, build, and E2E scripts. `marketing/package.json` has build/dev/preview scripts. `Makefile` still contains stale docs-style comments and broad deploy targets, but no immediate code-risk issue.
- Build warning remains: Vite reports `frontend/src/lib/signal/index.ts` is both dynamically and statically imported, so dynamic imports from `frontend/src/lib/stores/ws.ts` do not split that module into a separate chunk. This is not failing builds but is worth a later performance cleanup.

## E2E Test Findings

- E2E specs live under `frontend/tests`, with page objects in `frontend/tests/pages`, helpers in `frontend/tests/helpers`, and auth fixtures in `frontend/tests/fixtures`.
- Smoke coverage is broad: `rg "@smoke" frontend/tests` found 74 tagged smoke references.
- Skip usage is heavy and mixed. Some skips are environment-gated and appropriate (`E2E_EMAIL`, `TAURI_BINARY`, R2 unavailable), while some are explicit feature-not-implemented skips, especially Discord import and Tauri deep links.
- `frontend/tests/discord-import.spec.ts` contains three `test.skip` cases marked "feature not yet implemented".
- `frontend/tests/tauri-deep-links.spec.ts` contains a feature-not-implemented skip.
- `npm run lint` now exits cleanly with no E2E diagnostic `console.log` warnings.
- PR smoke currently depends on configured E2E credentials and a reachable API target. Dependabot-triggered runs are intentionally skipped when secrets are unavailable.

## Security Findings

- No committed real secret was confirmed in this pass. Targeted scans found placeholders in `.env.example`, expected GitHub Actions secret references, local test values, and code variables.
- `backend/secrets/`, `.env`, `.env.test`, local auth state, Firebase service account files, and local assistant state are ignored.
- `cargo audit` passed on `backend` at review time. The only prior active advisory was the known `rsa` advisory path; the ignore file and docs now match current audit reality.
- `npm audit --omit=dev --audit-level=high` passed for both `frontend` and `marketing`.
- `backend/migrations/20260328000031_fix_message_ciphertext_xor_plaintext.sql` now enforces the ciphertext/plaintext invariant for direct-content message rows while preserving v2 DM parent rows.
- WebSocket client code sends auth tokens in message frames, not query strings. Evidence: `frontend/src/lib/stores/ws.ts` sends `{ type: 'auth', token: ... }`, and `frontend/src/lib/stores/ws.test.ts` asserts socket URLs do not contain `token=`.
- S16 Canvas endpoints were not independently security-audited in this pass. Authorization, DTO validation, rate limiting, and collection caps should remain a dedicated security review item.

## UI And Accessibility Findings

- `npm run check` now exits cleanly with 0 Svelte warnings. The Canvas modal warnings were fixed by moving dialog semantics onto the modal panels, adding focusability, converting group labels to semantic text, and adding control labels/pressed states.
- Local Playwright smoke during Phase 3 verified:
  - `/login` desktop renders heading, email, password, sign-in, and forgot-password controls.
  - `/register` desktop renders username, email, and password controls.
  - `/forgot-password` desktop renders heading and email control.
  - `/login` mobile at 390x844 has no horizontal overflow.
- Authenticated Canvas modal visual inspection remains gated by live auth/data, so this pass verifies the static Svelte accessibility surface rather than full Canvas workflow behavior.

## Verification Run

| Command | Result |
|---|---|
| `cd backend; cargo audit` | Passed |
| `cd frontend; npm audit --omit=dev --audit-level=high` | Passed |
| `cd marketing; npm audit --omit=dev --audit-level=high` | Passed |
| `git ls-files -ci --exclude-standard` | Passed, no tracked ignored files |
| `cd frontend; npm run check` | Passed, 0 warnings |
| `cd frontend; npm run lint` | Passed, 0 warnings |
| `cd frontend; npm run test` | Passed, 24 files / 102 tests |
| `cd frontend; npm run build` | Passed, existing Vite chunking warnings |

## Remaining Risks And Follow-Up

1. Run a dedicated S16 Canvas security review: endpoint authorization, rate limits, DTO validation, membership checks, and collection caps.
2. Triage skipped E2E tests into three buckets: credential-gated, intentionally post-MVP, and accidentally incomplete.
3. Decide whether Discord import and Tauri deep links are post-MVP or launch blockers; tests currently mark parts as not implemented.
4. Clean up Vite signal module import strategy if bundle splitting matters for launch performance.
5. Consider documenting self-hosted runner bootstrap and disk maintenance in a single canonical operations doc once the runner setup has stabilized.

## Recommendation

The repo is in good shape structurally, and CI/security gates are much healthier than at the start of the stabilization work. The next best move is a focused Canvas security review and E2E skip triage. Those two follow-ups will give the cleanest signal on whether remaining work is product completion, test maintenance, or true launch risk.
