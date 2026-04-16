# ─── Planner stage — computes the dependency recipe ──────────────────────────
# Pin to a specific cargo-chef release to guarantee reproducible builds.
# cargo-chef 0.1.71 · Rust 1.84 · Alpine 3.21 (MSRV = 1.80; image may use newer toolchain).
FROM lukemathwalker/cargo-chef:0.1.71-rust-1.84-alpine3.21 AS planner
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── Builder stage — caches deps then builds source ──────────────────────────
FROM lukemathwalker/cargo-chef:0.1.71-rust-1.84-alpine3.21 AS builder
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
