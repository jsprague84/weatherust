**Weatherust**

- Rust CLI that pulls current weather and 7-day outlook from OpenWeatherMap and optionally sends a Gotify notification.
- Supports ZIP or free-form location (e.g., "City,ST,US").
- Designed to run non-interactively in Docker, scheduled by Ofelia (no host cron/systemd required).

**Quick Start**

- Copy env and fill keys: `cp .env.example .env` (set `OWM_API_KEY`, `GOTIFY_KEY`, `GOTIFY_URL`; optional `DEFAULT_ZIP`, `DEFAULT_UNITS`).
- Start stack: `docker compose pull && docker compose up -d`
- Verify scheduler: `docker compose logs -f ofelia`
- Test once now: `docker compose run --rm weatherust` (uses `DEFAULT_*` from `.env`; add `--zip 52726 --units imperial` if not set)


**Prerequisites**

- OpenWeatherMap API key (`OWM_API_KEY`).
- Gotify server app key (`GOTIFY_KEY`) and message endpoint URL (`GOTIFY_URL`).
- Toolchain: Rust 1.90.0 (pinned via `rust-toolchain.toml`).

**Local Run**

- Copy `.env.example` to `.env` and fill in keys (do not commit `.env`).
- Build: `cargo build --release`
- Examples:
  - `./target/release/weatherust --zip 52726 --units imperial`
  - `./target/release/weatherust --location "Davenport,IA,US" --units metric --quiet`
 - Note: Local runs are primarily for development and testing; production deployments should use the Docker + Ofelia stack below.

**Docker**

- Compose (preferred):
  - Ensure `.env` exists with secrets.
  - The stack uses `ghcr.io/jsprague84/weatherust:latest` by default. For production, pin a release tag (see below).
  - Pull/update and start: `docker compose pull && docker compose up -d`
  - Manual run (ad-hoc): `docker compose run --rm weatherust --zip 52726 --units imperial --quiet`

**Scheduling with Ofelia (in Docker)**

- The compose stack includes an `ofelia` service that schedules a one-off run at 05:30 daily.
- Defaults in compose:
  - Location: ZIP `52726`, `--units imperial --quiet` passed as `command`.
  - Timezone: `America/Chicago`.
- Configure secrets in `.env` (same directory as compose):
  - `OWM_API_KEY`, `GOTIFY_KEY`, `GOTIFY_URL`.
  - Optional defaults: `DEFAULT_ZIP` or `DEFAULT_LOCATION`, and `DEFAULT_UNITS`.
    - If CLI flags are omitted, the app uses these defaults; `DEFAULT_ZIP` takes precedence over `DEFAULT_LOCATION`.
- How env is passed to the job:
  - Ofelia job-run does not inherit the service’s `env_file`.
  - We mount your host `.env` into the Ofelia service at `/ofelia/.env` and use the label `ofelia.job-run.<name>.env-file=/ofelia/.env` so the job container receives all variables.
  - Set `ENV_FILE_HOST_PATH` in `.env` to the absolute host path of your `.env` file (used by the Ofelia service volume).
- Start the stack:
  - `docker compose up -d`
- Logs:
  - `docker compose logs -f ofelia` (shows job runs and any errors)

Adjusting schedule:
- The schedule is defined as a label on the `ofelia` service in `docker-compose.yml`:
  - `ofelia.job-run.weatherust.schedule: "0 30 5 * * *"` (sec min hour day month weekday)
- Update it as needed and re-run `docker compose up -d`.

**GitHub / CI**

- This repo includes `.github/workflows/docker.yml` to build and publish a multi-arch Docker image to GHCR.
- Steps:
  - Push to `main` to publish `ghcr.io/jsprague84/weatherust:latest`.
  - Create a tag like `v0.1.0` to also publish `ghcr.io/jsprague84/weatherust:v0.1.0`.
  - Branch/PR builds publish testing tags so you can pull pre-merge images:
    - `:feature-branch-name` (branch name slugified)
    - `:pr-<number>` (on PRs)
    - `:sha-<short>` (commit SHA)
  - docker-compose supports overriding tags via `.env`:
    - `WEATHERUST_TAG` and `SPEEDYNOTIFY_TAG` (see `.env.example`).
  - If the GHCR package is private, configure a registry login on the host running compose.

**Releases, Pinning, and Pre‑Merge Testing**

- Recommended for stability: pin to a published release tag instead of `latest`.
- For pre‑merge testing, set env overrides in `.env`:
  - `WEATHERUST_TAG=feature-feature-scaffold`
  - `SPEEDYNOTIFY_TAG=feature-feature-scaffold`
  - Then: `docker compose pull && docker compose up -d`
  - Unset the overrides (or set to a release tag) after testing.

**Notes**

- Runtime image is distroless (cc variant) on Debian 12, running as non-root, which includes required libgcc runtime.
- Toolchain pinned to Rust 1.90.0 for reproducible builds.

**Security/Secrets**

- `.env` is gitignored. Do not commit real API tokens.
- Rotate tokens if they were ever exposed.
- Explicit Gotify app tokens per tool (recommended):
  - `GOTIFY_URL` points to your server (e.g., `https://gotify.example.com/message`).
  - `GOTIFY_KEY` → weatherust app token.
  - `SPEEDY_GOTIFY_KEY` → speedynotify app token (the binary sets `GOTIFY_KEY` internally).
  - `DOCKERMON_GOTIFY_KEY` → dockermon app token (the binary sets `GOTIFY_KEY` internally).
  - If you prefer a single app, set all three to the same value.
  - Optional: `GOTIFY_KEY_FILE` path to a file containing only a token (fallback).

Example (Docker secrets-style mounting):
- Create a file with only the key, e.g., `/opt/secrets/gotify_key`.
- Mount it into the job container and set `GOTIFY_KEY_FILE` via Ofelia labels:
  - `ofelia.job-run.weatherust.volume=/opt/secrets/gotify_key:/run/secrets/gotify_key:ro`
  - `ofelia.job-run.weatherust.env=GOTIFY_KEY_FILE=/run/secrets/gotify_key|...`

**CLI Reference**

- Flags:
  - `--zip <ZIP[,CC]>` e.g., `52726` or `52726,US`.
  - `--location <free-form>` e.g., `Davenport,IA,US`.
  - `--units <imperial|metric>` (default `DEFAULT_UNITS` or `imperial`).
  - `--quiet` suppresses stdout (useful in scheduled runs).

Environment defaults:
- `DEFAULT_ZIP` or `DEFAULT_LOCATION` provide a default location when CLI flags are not set.
- `DEFAULT_UNITS` sets default units when `--units` is not specified.
- If neither CLI nor env provide a location, the app prompts interactively.

**Ideas for Future Enhancements**

- Resilience: retry/backoff for transient HTTP errors; clearer non-zero exit on fatal failures (for monitoring).
- Severe weather: optional alert mode (high Gotify priority) if daily description matches severe conditions.
- Config: accept defaults via env (e.g., `DEFAULT_ZIP`, `DEFAULT_UNITS`) to reduce args in compose labels.
- Output: compact vs verbose templates; optional emoji/icons; configurable Gotify priority/title.
- Multi-location: support multiple ZIPs/locations in one run with aggregated message.
- Logging/metrics: structured logs and a simple success/failure metric (stdout) for scraping.
- Tests/CI: add unit tests around parsing/formatting and a lint step in Actions.

**Additional Tools (Workspace)**

This repo is now a Rust workspace with a shared helper crate. A second binary, `speedynotify`, runs the Ookla Speedtest CLI and sends a Gotify summary.

Added: `dockermon` — checks Docker containers for health issues and high CPU/MEM and sends a Gotify summary. Designed for Ofelia to run every 5 minutes. It uses Ofelia's `env-file` label for reliable environment passing.

- Enable in compose:
  - Image: `ghcr.io/jsprague84/speedynotify:latest` (publish separately).
  - Ofelia labels included for a daily run at 02:10.
  - Configure thresholds in `.env`: `SPEEDTEST_MIN_DOWN`, `SPEEDTEST_MIN_UP`, optional `SPEEDTEST_SERVER_ID`.
  - Reuses the same `GOTIFY_*` envs and `.env` mount via `ENV_FILE_HOST_PATH`.

Build locally:
- Weather: `docker build -t weatherust:local .`
- Speedtest: `docker build -f Dockerfile.speedynotify -t speedynotify:local .`
 - Docker monitor: `docker build -f Dockerfile.dockermon -t dockermon:local .`

Publish images (CI):
- Weather image is built by `.github/workflows/docker.yml` -> `ghcr.io/<owner>/weatherust`.
- Speedtest image is built by `.github/workflows/docker-speedynotify.yml` -> `ghcr.io/<owner>/speedynotify`.
 - Docker monitor image is built by `.github/workflows/docker-dockermon.yml` -> `ghcr.io/<owner>/dockermon`.
- After first successful publish, make the GHCR package public in GitHub Packages so compose hosts can pull without auth.

**Scaffolding New Features**

- To create another small feature that sends messages to Gotify, use the scaffold:
  - `scripts/scaffold_feature.sh <name> "Short description"`
  - Then implement `<name>/src/main.rs`, and adjust `Dockerfile.<name>` if OS deps are needed.
  - A GitHub Action is generated at `.github/workflows/docker-<name>.yml` to publish `ghcr.io/<owner>/<name>`.
  - See `FEATURES.md` for details and the Ofelia label pattern to schedule your new image.

**Docker Monitor (dockermon)**

- Purpose: Alert when any container is not running, has failing health, or exceeds CPU/MEM thresholds.
- Env:
  - `HEALTH_NOTIFY_ALWAYS` (default `false`) — notify even when all OK.
  - `CPU_WARN_PCT` (default `85`) — CPU percentage threshold.
  - `MEM_WARN_PCT` (default `90`) — memory percentage threshold.
  - `DOCKERMON_GOTIFY_KEY` (optional) — tool-specific token; falls back to `GOTIFY_KEY`/`GOTIFY_KEY_FILE`.
  - `GOTIFY_DEBUG` (optional) — set to `true`/`1` to print debug info in logs (URL, token source).
- Compose integration:
  - Service mounts the Docker socket read-only.
- Ofelia mounts the host `.env` inside the container at `/ofelia/.env`; pointing `env-file=/ofelia/.env` at each job-run keeps scheduled runs aligned with the service dotenv entries.
- Job mounts the Docker socket via a single `volume` label.
- Tag override: set `DOCKERMON_TAG` in `.env` for pre-merge testing.

Runtime pattern (robust):
- This compose keeps a lightweight `dockermon_runner` container (same image, entrypoint overridden to `sleep infinity`) alive so Ofelia can `job-exec` into it and automatically inherit the full `.env` plus socket mount:
  - `ofelia.job-exec.dockermon.container=dockermon_runner`
  - `ofelia.job-exec.dockermon.command=/app/dockermon --quiet`
