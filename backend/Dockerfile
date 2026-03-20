# ─── Planner stage — computes the dependency recipe ──────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS planner
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── Builder stage — caches deps then builds source ──────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# This layer is cached as long as dependencies don't change
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

# ─── Runtime stage ────────────────────────────────────────────────────────────
FROM alpine:3.21

RUN apk add --no-cache ca-certificates libgcc \
    && addgroup -S yapper \
    && adduser -S -G yapper -h /app yapper

WORKDIR /app

COPY --from=builder /app/target/release/yapper-server ./yapper-server
COPY --from=builder /app/migrations ./migrations
RUN chown -R yapper:yapper /app

EXPOSE 8080

ENV RUST_LOG=info

USER yapper

CMD ["./yapper-server"]
