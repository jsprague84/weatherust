# Multi-stage build to produce a small runtime image

FROM rust:1.83-bookworm AS builder
WORKDIR /app

# Build dependencies (openssl for historical lockfiles; harmless if unused)
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy full source and build (simpler, more reliable in CI)
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# --- Runtime ---
FROM gcr.io/distroless/base-debian12:nonroot AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/weatherust /app/weatherust

ENV RUST_LOG=info
ENTRYPOINT ["/app/weatherust"]
CMD ["--help"]
