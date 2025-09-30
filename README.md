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
  - Location: ZIP `52726` with `--units imperial --quiet`.
  - Timezone: `America/Chicago`.
- Configure secrets in `.env`:
  - `OWM_API_KEY`, `GOTIFY_KEY`, `GOTIFY_URL`.
  - Optional defaults: `DEFAULT_ZIP` or `DEFAULT_LOCATION`, and `DEFAULT_UNITS`.
    - If CLI flags are omitted, the app uses these defaults; `DEFAULT_ZIP` takes precedence over `DEFAULT_LOCATION`.
  - Note: Ofelia’s job-run containers do not inherit the service’s `env_file`; this compose passes the needed env vars explicitly via labels.
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
  - docker-compose here is already set to `ghcr.io/jsprague84/weatherust:latest` for both the service and Ofelia job.
  - If the GHCR package is private, configure a registry login on the host running compose.

**Releases and Version Pinning**

- Recommended for stability: pin to a published release tag instead of `latest`.
- In `docker-compose.yml` change both references:
  - `weatherust.image: ghcr.io/jsprague84/weatherust:v0.1.0`
  - `ofelia.job-run.weatherust.image: ghcr.io/jsprague84/weatherust:v0.1.0`
- Apply: `docker compose pull && docker compose up -d`

**Notes**

- Runtime image is distroless (cc variant) on Debian 12, running as non-root, which includes required libgcc runtime.
- Toolchain pinned to Rust 1.90.0 for reproducible builds.

**Security/Secrets**

- `.env` is gitignored. Do not commit real API keys.
- If keys were previously committed, rotate them in OWM and Gotify.

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
