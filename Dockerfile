# ── Stage 1: Build ─────────────────────────────────────────────────────────────
FROM rust:1.93-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static cmake make gcc g++ perl

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
# Cache deps
RUN mkdir src && echo "fn main(){}" > src/main.rs && echo "" > src/lib.rs && cargo build --release 2>/dev/null || true
COPY src/ src/
RUN touch src/main.rs src/lib.rs && cargo build --release

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM alpine:3.21

RUN apk add --no-cache git ca-certificates tini

COPY --from=builder /app/target/release/forge /usr/local/bin/forge

RUN adduser -D -u 1000 forge
USER forge
WORKDIR /data

ENV FORGE_HOST=0.0.0.0 \
    FORGE_PORT=3000 \
    FORGE_REPO_PATH=/data/repo \
    FORGE_DB_PATH=/data/forge.db \
    FORGE_SEARCH_INDEX_PATH=/data/search_index

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:3000/health || exit 1

ENTRYPOINT ["tini", "--"]
CMD ["forge"]
