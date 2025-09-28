# Multi-stage build to produce a small runtime image

FROM rust:1.81-bullseye AS builder
WORKDIR /app

# Build dependencies (openssl for historical lockfiles; harmless if unused)
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache deps
COPY Cargo.toml Cargo.lock ./
# Prefetch dependencies to leverage layer caching without compiling them
RUN mkdir -p src && echo "fn main(){}" > src/main.rs \
    && cargo fetch

# Build
COPY src ./src
RUN cargo build --release

# --- Runtime ---
FROM gcr.io/distroless/base-debian12:nonroot AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/weatherust /app/weatherust

ENV RUST_LOG=info
ENTRYPOINT ["/app/weatherust"]
CMD ["--help"]
