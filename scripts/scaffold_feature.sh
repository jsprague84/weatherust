#!/usr/bin/env bash
set -euo pipefail

# Scaffold a new feature crate and supporting files based on the existing structure.
#
# Usage: scripts/scaffold_feature.sh <feature-name> "Short description"
# Example: scripts/scaffold_feature.sh airquality "AQI fetch -> Gotify"
#
# This creates:
# - <feature-name>/ (Rust bin crate wired to shared `common` crate)
# - Dockerfile.<feature-name>
# - .github/workflows/docker-<feature-name>.yml
# - Adds the crate to workspace members in Cargo.toml
#
# After generation:
# - Customize the new crate's src/main.rs to implement its function
# - Tweak the Dockerfile.<feature-name> to install any needed OS deps
# - Optionally add Ofelia labels in docker-compose.yml

if [[ ${#} -lt 1 ]]; then
  echo "Usage: $0 <feature-name> [\"Short description\"]" >&2
  exit 1
fi

NAME="$1"
DESC="${2:-New feature -> Gotify}"

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CRATE_DIR="$ROOT_DIR/$NAME"

if [[ -d "$CRATE_DIR" ]]; then
  echo "Error: directory $CRATE_DIR already exists" >&2
  exit 1
fi

echo "Scaffolding feature crate: $NAME"
mkdir -p "$CRATE_DIR/src"

cat >"$CRATE_DIR/Cargo.toml" <<EOF
[package]
name = "$NAME"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
common = { path = "../common" }
EOF

cat >"$CRATE_DIR/src/main.rs" <<'EOF'
use clap::Parser;
use common::{dotenv_init, http_client, send_gotify};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(about = "New feature -> Gotify")] 
struct Args {
    /// Suppress stdout; only send Gotify
    #[arg(long, default_value_t = false)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv_init();
    let args = Args::parse();

    // TODO: implement actual logic here; this is a stub
    let title = "New feature: OK";
    let body = "Replace this stub with real data";

    if !args.quiet {
        println!("{}\n{}", title, body);
    }

    let client = http_client();
    if let Err(e) = send_gotify(&client, title, body).await {
        eprintln!("Gotify send error: {e}");
    }
    Ok(())
}
EOF

# Update workspace members in root Cargo.toml
if ! grep -qE "^members = .*\b$NAME\b" "$ROOT_DIR/Cargo.toml"; then
  # Append the member, preserving array formatting
  awk -v name="$NAME" '
    BEGIN{in_ws=0}
    /^
\[workspace\]/{in_ws=1}
    in_ws && /^members\s*=\s*\[/ {open=1}
    {
      print $0
      if (open) {
        print "    \"" name "\",";
        open=0
      }
    }
  ' "$ROOT_DIR/Cargo.toml" >"$ROOT_DIR/Cargo.toml.tmp"
  mv "$ROOT_DIR/Cargo.toml.tmp" "$ROOT_DIR/Cargo.toml"
fi

# Dockerfile for this feature (multi-arch ready via GH Actions)
cat >"$ROOT_DIR/Dockerfile.$NAME" <<EOF
FROM rust:1.90-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY $NAME ./$NAME
RUN cargo build -p $NAME --release

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/$NAME /app/$NAME

ENV RUST_LOG=info
ENTRYPOINT ["/app/$NAME"]
EOF

# GitHub Action workflow for building and publishing this image to GHCR
mkdir -p "$ROOT_DIR/.github/workflows"
cat >"$ROOT_DIR/.github/workflows/docker-$NAME.yml" <<EOF
name: build-and-publish-$NAME

on:
  push:
    branches: [ main, master ]
    tags: [ 'v*.*.*' ]
  pull_request:
  workflow_dispatch:

jobs:
  docker:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up QEMU
        uses: docker/setup-qemu-action@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: \\${{ github.repository_owner }}
          password: \\${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata (tags, labels)
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/\\${{ github.repository_owner }}/$NAME
          tags: |
            type=raw,value=latest,enable={{is_default_branch}}
            type=ref,event=tag

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: Dockerfile.$NAME
          push: true
          platforms: linux/amd64,linux/arm64
          cache-from: type=gha
          cache-to: type=gha,mode=max
          tags: \\${{ steps.meta.outputs.tags }}
          labels: \\${{ steps.meta.outputs.labels }}
EOF

echo "Done. Next steps:"
echo "- Implement $NAME/src/main.rs logic"
echo "- Commit changes and push; Actions will publish ghcr.io/<owner>/$NAME"
echo "- Add an Ofelia job in docker-compose.yml to schedule runs (optional)"

