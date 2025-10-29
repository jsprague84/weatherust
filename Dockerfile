# Multi-stage build to produce a small runtime image

FROM rust:1.90-bookworm AS builder
WORKDIR /app

# Build dependencies (openssl for historical lockfiles; harmless if unused)
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy full workspace so Cargo can resolve all members reliably
COPY . .
RUN cargo build --release

# --- Runtime ---
FROM debian:12-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/weatherust /app/weatherust

ENV RUST_LOG=info
ENTRYPOINT ["/app/weatherust"]
