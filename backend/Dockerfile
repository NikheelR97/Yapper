# ─── Build stage ─────────────────────────────────────────────────────────────
FROM rust:1.93-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Cache dependencies separately from source for faster rebuilds
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Build actual source
COPY . .
# Touch main.rs to force rebuild (dependency cache is warm)
RUN touch src/main.rs
ENV SQLX_OFFLINE=true
RUN cargo build --release

# ─── Runtime stage ────────────────────────────────────────────────────────────
FROM alpine:3.19

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
