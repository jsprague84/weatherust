# Multi-stage build to produce a small runtime image

FROM rust:1.81-bullseye AS builder
WORKDIR /app

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs \
    && cargo build --release \
    && rm -rf target/release/deps/weatherust*

# Build
COPY src ./src
RUN cargo build --release

# --- Runtime ---
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Non-root user
RUN useradd -m -u 10001 appuser

COPY --from=builder /app/target/release/weatherust /app/weatherust
USER appuser

ENV RUST_LOG=info
ENTRYPOINT ["/app/weatherust"]
CMD ["--help"]

