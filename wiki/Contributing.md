# Contributing

Thank you for your interest in contributing to Yapper.

## Before you start

- Check [open issues](https://github.com/NikheelR97/Yapper/issues) to avoid duplicate work
- For significant changes, open an issue first to discuss the approach
- Read the [Architecture](Architecture) and relevant development guides before writing code

## Development setup

See [Getting Started](Getting-Started) for a full local setup guide.

## Workflow

### 1. Fork and branch

```bash
git checkout -b feat/YAP-123-short-description
# or
git checkout -b fix/login-error-message
```

Branch naming:
- `feat/` — new feature
- `fix/` — bug fix
- `chore/` — tooling, deps, CI
- `docs/` — documentation only

### 2. Make your changes

- Keep pull requests focused — one logical change per PR
- Write tests for new behaviour (Rust unit tests or Playwright E2E)
- Run the full check suite locally before pushing (see below)

### 3. Check suite

**Backend:**
```bash
cd backend
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo sqlx prepare --check   # if you changed any queries
```

**Frontend:**
```bash
cd frontend
npm run check      # TypeScript + Svelte type check
npm run test       # Vitest unit tests
npm run lint       # ESLint + Prettier
```

### 4. Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(auth): add Apple OAuth provider
fix(parental): validate password length before hashing
chore(deps): bump axum from 0.7.4 to 0.7.5
docs(wiki): add E2EE implementation page
```

### 5. Open a pull request

- Target branch: `main`
- Fill in the PR template
- CI must pass before merge (fmt + clippy + tests + audit + build)

---

## Coding conventions

We follow a subset of the NASA/JPL Power of Ten rules. The most important ones:

| Rule | Summary |
|------|---------|
| 3 | Validate all inputs at module boundaries. Use `MAX_*` constants for buffer sizes. |
| 4 | No function exceeds 60 executable lines. Extract helpers rather than nesting. |
| 7 | Check every `Result` / `Option`. No silent failures in production paths. |
| 10 | Avoid type-unsafe casts. Use typed abstractions (`tauri-compat.ts`, `AppError`). |

For Rust specifically:
- Use `AppError::BadRequest` for client errors, `AppError::Internal` for unexpected failures
- Use `sqlx::query()` (not macros) for new queries to avoid requiring `cargo sqlx prepare` on every change
- Define `const MAX_FOO: usize = N;` at the module level for any limit you enforce

For TypeScript specifically:
- Never use `(window as any)` — use `isTauri()` from `tauri-compat.ts`
- Empty `catch {}` blocks must have a comment explaining why the error is intentionally ignored
- `void promise()` calls must have a `// fire-and-forget` comment

---

## Testing

### Unit tests

**Rust** — `cargo test` runs all tests in `backend/src/`

**TypeScript** — `npm run test` runs Vitest tests under `frontend/src/lib/signal/` and `frontend/src/lib/stores/`

### E2E tests (Playwright)

See [Frontend Development → Playwright E2E tests](Frontend-Development#playwright-e2e-tests) for setup.

E2E tests run nightly against production and can be triggered manually:
```bash
gh workflow run e2e-nightly.yml --field base_url=https://app.yapperhq.com
```

---

## Security issues

**Do not open a public issue for security vulnerabilities.** See the [Security](Security) page for the responsible disclosure process.

---

## Questions?

Open a [GitHub Discussion](https://github.com/NikheelR97/Yapper/discussions) for questions about the codebase or architecture.
