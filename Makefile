.PHONY: dev-backend dev-frontend migrate deploy test lint fmt audit help

# ─── Development ─────────────────────────────────────────────────────────────

dev-backend:
	cd backend && cargo watch -x run

dev-frontend:
	cd frontend && npm run dev

dev-tauri:
	cd frontend && npm run tauri dev

dev-marketing:
	cd marketing && npm run dev

# Start PostgreSQL (Docker) + backend in parallel
dev:
	docker compose up -d
	$(MAKE) dev-backend

# ─── Database ────────────────────────────────────────────────────────────────

db-up:
	docker compose up -d postgres

db-down:
	docker compose down

migrate:
	cd backend && sqlx migrate run

migrate-revert:
	cd backend && sqlx migrate revert

# Generate sqlx offline query cache (run before committing if queries changed)
sqlx-prepare:
	cd backend && cargo sqlx prepare

# ─── Testing ─────────────────────────────────────────────────────────────────

test:
	cd backend && cargo test
	cd frontend && npm run test

test-backend:
	cd backend && cargo test

test-frontend:
	cd frontend && npm run test

test-e2e:
	cd frontend && npx playwright test

# ─── Code Quality ────────────────────────────────────────────────────────────

lint:
	cd backend && cargo clippy -- -D warnings
	cd frontend && npm run lint

fmt:
	cd backend && cargo fmt
	cd frontend && npm run format

fmt-check:
	cd backend && cargo fmt --check
	cd frontend && npm run format:check

audit:
	cd backend && cargo audit
	cd frontend && npm audit

# ─── Deployment ──────────────────────────────────────────────────────────────

deploy-backend:
	cd backend && fly deploy

deploy-frontend:
	cd frontend && npm run build && npx wrangler pages deploy build

deploy-marketing:
	cd marketing && npm run build && npx wrangler pages deploy dist

deploy-worker:
	cd marketing && npx wrangler deploy

deploy: deploy-backend deploy-frontend

# ─── Help ────────────────────────────────────────────────────────────────────

help:
	@echo "Yapper Makefile targets:"
	@echo "  dev-backend      Hot-reload Rust backend (cargo watch)"
	@echo "  dev-frontend     SvelteKit dev server"
	@echo "  dev-tauri        Tauri desktop app (dev mode)"
	@echo "  dev-marketing    Astro marketing site dev server"
	@echo "  dev              Docker PostgreSQL + backend together"
	@echo "  db-up / db-down  Start/stop Docker PostgreSQL"
	@echo "  migrate          Run pending sqlx migrations"
	@echo "  sqlx-prepare     Cache sqlx queries for offline builds"
	@echo "  test             Run all tests (backend + frontend)"
	@echo "  lint             clippy + eslint"
	@echo "  fmt              cargo fmt + prettier"
	@echo "  audit            cargo audit + npm audit"
	@echo "  deploy           Deploy backend (Fly.io) + frontend (CF Pages)"
	@echo "  deploy-worker    Deploy wishlist Cloudflare Worker"
