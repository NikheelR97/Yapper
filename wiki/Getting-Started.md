# Getting Started

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.85+ | Install via [rustup](https://rustup.rs) |
| Node.js | 20+ | Use [nvm](https://github.com/nvm-sh/nvm) or [fnm](https://github.com/Schniz/fnm) |
| PostgreSQL | 16+ | Local install or Docker |
| sqlx-cli | latest | `cargo install sqlx-cli --no-default-features --features postgres` |

Optional (for desktop builds):
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS

Optional (for mobile builds):
- Xcode 15+ (macOS only) for iOS
- Android Studio for Android

---

## 1. Clone the repo

```bash
git clone https://github.com/NikheelR97/Yapper.git
cd Yapper
```

---

## 2. Backend setup

### 2a. Create the database

```bash
createdb yapper
```

Or with Docker:
```bash
docker run -d \
  -e POSTGRES_USER=yapper \
  -e POSTGRES_PASSWORD=yapper_dev \
  -e POSTGRES_DB=yapper \
  -p 5432:5432 \
  postgres:16-alpine
```

### 2b. Configure environment

```bash
cp backend/.env.example backend/.env
```

Edit `backend/.env`:

```env
DATABASE_URL=postgres://yapper:yapper_dev@localhost:5432/yapper

# JWT keys — generate once and keep in backend/secrets/
JWT_PRIVATE_KEY_PATH=secrets/jwt_private.pem
JWT_PUBLIC_KEY_PATH=secrets/jwt_public.pem

# Optional — leave empty to disable locally
RESEND_API_KEY=
DISCORD_CLIENT_ID=
DISCORD_CLIENT_SECRET=
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
SENTRY_DSN=
HUBSPOT_ACCESS_TOKEN=

# Cloudflare R2 (optional — media uploads disabled without these)
R2_ACCOUNT_ID=
R2_ACCESS_KEY_ID=
R2_SECRET_ACCESS_KEY=
R2_BUCKET_NAME=
R2_ENDPOINT=
R2_PUBLIC_URL=

# CORS — comma-separated list of allowed origins
CORS_ORIGINS=http://localhost:5173
```

### 2c. Generate JWT keys

```bash
mkdir -p backend/secrets
openssl genrsa -out backend/secrets/jwt_private.pem 2048
openssl rsa -in backend/secrets/jwt_private.pem -pubout -out backend/secrets/jwt_public.pem
```

### 2d. Run migrations

```bash
cd backend
sqlx migrate run
```

### 2e. Start the backend

```bash
cargo run
```

The API is now available at `http://localhost:8080`. Health check: `curl http://localhost:8080/health`.

---

## 3. Frontend setup

```bash
cd frontend
npm install
```

Create `frontend/.env`:

```env
VITE_API_URL=http://localhost:8080
VITE_WS_URL=ws://localhost:8080/ws
VITE_FIREBASE_VAPID_KEY=   # optional — push notifications
VITE_SENTRY_DSN=           # optional — error monitoring
```

Start the dev server:

```bash
npm run dev
```

The app is available at `http://localhost:5173`.

---

## 4. Run tests

### Backend unit tests

```bash
cd backend
cargo test
```

### Frontend unit tests (Vitest)

```bash
cd frontend
npm run test
```

### E2E tests (Playwright)

Requires two test accounts on a running backend. Configure `frontend/.env.test`:

```env
BASE_URL=http://localhost:5173
VITE_API_URL=http://localhost:8080
E2E_EMAIL=your@test.email
E2E_PASSWORD=yourpassword
E2E_EMAIL_2=second@test.email
E2E_PASSWORD_2=secondpassword
```

```bash
cd frontend
npx playwright test
```

To run with the interactive UI:

```bash
npx playwright test --ui
```

---

## 5. Optional: Desktop build (Tauri)

```bash
cd frontend
npm run tauri dev
```

To build a release installer:

```bash
npm run tauri build
```

Output: `src-tauri/target/release/bundle/`

---

## Common issues

| Problem | Fix |
|---------|-----|
| `sqlx migrate run` fails | Check `DATABASE_URL` is set and the DB exists |
| `cargo build` fails on OpenSSL | Install `pkg-config` and `libssl-dev` (Linux) |
| WebSocket won't connect | Ensure `VITE_WS_URL` matches the backend port |
| `cargo sqlx prepare` fails | Use the direct (non-pooler) Neon endpoint; PgBouncer does not support prepared statements |
| Tauri build fails on Windows | Ensure Visual Studio Build Tools and WebView2 are installed |
