**Weatherust**

- Rust CLI that pulls current weather and 7-day outlook from OpenWeatherMap and optionally sends notifications via Gotify and/or ntfy.sh.
- Supports ZIP or free-form location (e.g., "City,ST,US").
- Designed to run non-interactively in Docker, scheduled by Ofelia (no host cron/systemd required).

**Quick Start**

- Copy env and fill keys: `cp .env.example .env` (set `OWM_API_KEY` and notification backend - see Notifications section below)
- Start stack: `docker compose pull && docker compose up -d`
- Verify scheduler: `docker compose logs -f ofelia`
- Test once now: `docker compose run --rm weatherust` (uses `DEFAULT_*` from `.env`; add `--zip 52726 --units imperial` if not set)


**Prerequisites**

- OpenWeatherMap API key (`OWM_API_KEY`).
- Notification backend (choose one or both):
  - **Gotify**: Server URL (`GOTIFY_URL`) and app keys (service-specific: `WEATHERUST_GOTIFY_KEY`, `UPDATEMON_GOTIFY_KEY`, etc.)
  - **ntfy.sh**: Server URL (`NTFY_URL`, defaults to https://ntfy.sh) and topics (service-specific: `WEATHERUST_NTFY_TOPIC`, `UPDATEMON_NTFY_TOPIC`, etc.)
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

**Notifications**

All services support two notification backends: **Gotify** and **ntfy.sh**. You can use one or both simultaneously.

**Gotify Configuration**

Gotify is a self-hosted notification server. Each service checks for its own service-specific key:

```bash
# Gotify server URL (shared by all services)
GOTIFY_URL=https://gotify.example.com/message

# Service-specific Gotify app tokens
WEATHERUST_GOTIFY_KEY=your_weatherust_token
UPDATEMON_GOTIFY_KEY=your_updatemon_token
UPDATECTL_GOTIFY_KEY=your_updatectl_token
HEALTHMON_GOTIFY_KEY=your_healthmon_token
SPEEDY_GOTIFY_KEY=your_speedynotify_token
```

Setup options:
- **Simple**: Use the same token for all services (all notifications in one Gotify app)
- **Organized**: Create separate Gotify apps and tokens for each service

Optional:
- `GOTIFY_KEY_FILE=/run/secrets/gotify_key` - Path to file containing token (fallback if service key not set)
- `GOTIFY_DEBUG=true` - Enable debug logging (masks token, shows URL/message lengths)

**ntfy.sh Configuration**

ntfy.sh is an open-source notification service supporting action buttons. Each service publishes to its own topic:

```bash
# ntfy server URL (defaults to https://ntfy.sh if not set)
NTFY_URL=https://ntfy.js-node.com

# Optional: Authentication token for self-hosted ntfy servers
NTFY_AUTH=your_ntfy_auth_token

# Service-specific ntfy topics
WEATHERUST_NTFY_TOPIC=weatherust
UPDATEMON_NTFY_TOPIC=updates
UPDATECTL_NTFY_TOPIC=update-actions
HEALTHMON_NTFY_TOPIC=docker-health
SPEEDY_NTFY_TOPIC=speedtest
```

Setup options:
- **Simple**: Use the same topic for all services (all notifications in one feed)
- **Organized**: Use different topics per service for separate notification feeds

Optional:
- `NTFY_DEBUG=true` - Enable debug logging (masks token, shows URL/topic/message lengths)

**Using Both Backends**

Services will send to both Gotify and ntfy if both are configured. Errors in one backend don't affect the other. If a service-specific key/topic is not set, that backend is silently skipped for that service.

Example (dual configuration in `.env`):
```bash
# Both backends active
GOTIFY_URL=https://gotify.example.com/message
WEATHERUST_GOTIFY_KEY=gotify_token_here
NTFY_URL=https://ntfy.js-node.com
WEATHERUST_NTFY_TOPIC=weather-alerts
```

**Security/Secrets**

- `.env` is gitignored. Do not commit real API tokens or ntfy auth tokens.
- Rotate tokens if they were ever exposed.
- See the **Notifications** section above for detailed configuration of Gotify and ntfy.sh backends.
- Each service checks for its own service-specific key/topic (e.g., `WEATHERUST_GOTIFY_KEY`, `UPDATEMON_NTFY_TOPIC`).

Example (Docker secrets-style mounting for Gotify):
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

This repo is now a Rust workspace with a shared helper crate. A second binary, `speedynotify`, runs the Ookla Speedtest CLI and sends notifications via Gotify and/or ntfy.sh.

Added: `healthmon` (renamed from `dockermon`) — checks Docker containers for health issues and high CPU/MEM and sends notifications via Gotify and/or ntfy.sh. Designed for Ofelia to run every 5 minutes. It uses Ofelia's `env-file` label for reliable environment passing.

- Enable in compose:
  - Image: `ghcr.io/jsprague84/speedynotify:latest` (publish separately).
  - Ofelia labels included for a daily run at 02:10.
  - Configure thresholds in `.env`: `SPEEDTEST_MIN_DOWN`, `SPEEDTEST_MIN_UP`, optional `SPEEDTEST_SERVER_ID`.
  - Reuses the same `GOTIFY_*` envs and `.env` mount via `ENV_FILE_HOST_PATH`.

Build locally:
- Weather: `docker build -t weatherust:local .`
- Speedtest: `docker build -f Dockerfile.speedynotify -t speedynotify:local .`
- Health monitor: `docker build -f Dockerfile.healthmon -t healthmon:local .`

Publish images (CI):
- All images are built by `.github/workflows/build-all-services.yml` in parallel
- Images published: weatherust, speedynotify, healthmon, updatemon, updatectl
- Push to `main` publishes `latest` tags, push tags like `v1.0.0` for versioned releases
- After first successful publish, make the GHCR packages public in GitHub Packages so compose hosts can pull without auth.

**Scaffolding New Features**

- To create another small feature that sends messages to Gotify, use the scaffold:
  - `scripts/scaffold_feature.sh <name> "Short description"`
  - Then implement `<name>/src/main.rs`, and adjust `Dockerfile.<name>` if OS deps are needed.
  - A GitHub Action is generated at `.github/workflows/docker-<name>.yml` to publish `ghcr.io/<owner>/<name>`.
  - See `FEATURES.md` for details and the Ofelia label pattern to schedule your new image.

**Health Monitor (healthmon)**

healthmon (formerly `dockermon`) monitors Docker container health and resource usage. After refactoring, **all Docker cleanup functionality has been moved to updatectl**.

- Purpose: Alert when any container is not running, has failing health, or exceeds CPU/MEM thresholds.
- Env:
  - `HEALTH_NOTIFY_ALWAYS` (default `false`) — notify even when all OK.
  - `CPU_WARN_PCT` (default `85`) — CPU percentage threshold.
  - `MEM_WARN_PCT` (default `90`) — memory percentage threshold.
  - `HEALTHMON_GOTIFY_KEY` (optional) — Gotify token for this service.
  - `HEALTHMON_NTFY_TOPIC` (optional) — ntfy.sh topic for this service.
  - `GOTIFY_DEBUG` / `NTFY_DEBUG` (optional) — set to `true`/`1` to print debug info in logs.
  - `HEALTHMON_IGNORE` (optional) — comma-separated list of container names/IDs/service names to skip (case-insensitive).
- Compose integration:
  - Service mounts the Docker socket read-only.
- Tag override: set `HEALTHMON_TAG` in `.env` for pre-merge testing.

Runtime pattern (robust):
- This compose keeps a lightweight `healthmon_runner` container (same image, entrypoint overridden to `sleep infinity`) alive so Ofelia can `job-exec` into it and automatically inherit the full `.env` plus socket mount. Use the `HEALTHMON_IGNORE` env (or `--ignore` CLI flag) to suppress noise from short-lived containers (e.g., the one-off weather/speedy jobs).
  - `ofelia.job-exec.healthmon-health.container=healthmon_runner`
  - `ofelia.job-exec.healthmon-health.command=/app/healthmon health --quiet`

For Docker cleanup operations, see **updatectl** below.

**Update Monitor (updatemon) & Update Controller (updatectl)**

Two companion tools for managing updates and cleanup across your infrastructure:

**updatemon** - Multi-server update monitoring (read-only)
- Checks OS packages (apt/dnf/pacman) and Docker images for available updates
- Parallel execution across multiple servers via SSH
- Notifications via Gotify and/or ntfy.sh with detailed update summaries
- Runs daily (3:00 AM) to keep you informed
- Safe for automation - never modifies anything
- [Full documentation](updatemon/README.md)

**updatectl** - Multi-server update controller & cleanup tool (applies changes)
- **OS Updates**: Apply package updates across servers (apt/dnf/pacman)
- **Docker Updates**: Pull updated Docker images across servers
- **Docker Cleanup**: Clean up dangling images, unused networks, old containers, build cache
- **OS Cleanup**: Clean package cache and remove unused packages (apt clean/autoremove)
- Server name resolution for easy CLI usage
- Interactive confirmation prompts (skip with `--yes` for automation)
- Dry-run mode to preview changes before applying
- Parallel execution with error isolation per server
- Discovery commands: `list servers`, `list examples`
- Automated updates via Ofelia (disabled by default for safety)
- [Full documentation](updatectl/README.md)

Configuration (shared between both tools):
```bash
# Server list (format: name:user@host or just user@host)
UPDATE_SERVERS=Office-HP-WS:jsprague@192.168.1.189,Cloud VM1:ubuntu@cloud-vm1.js-node.com

# SSH key for passwordless authentication
UPDATE_SSH_KEY=/home/ubuntu/.ssh/id_ed25519

# Notification backends (choose Gotify, ntfy.sh, or both)
UPDATEMON_GOTIFY_KEY=your_updatemon_token
UPDATEMON_NTFY_TOPIC=updates
UPDATECTL_GOTIFY_KEY=your_updatectl_token
UPDATECTL_NTFY_TOPIC=update-actions
```

Quick start:
```bash
# Set up shell alias for easy CLI usage (recommended)
echo 'alias updatectl="docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatectl_runner /app/updatectl"' >> ~/.bashrc
source ~/.bashrc

# List configured servers
updatectl list servers

# Check for updates on all servers (automated daily)
docker compose exec updatemon_runner /app/updatemon --docker

# Preview what would be updated on localhost (safe)
updatectl all --dry-run --local

# Apply updates to specific server
updatectl os --yes --servers "Cloud VM1"

# Update Docker images on localhost only
updatectl docker --all --yes --local

# Update all configured servers
updatectl os --yes
```

Workflow:
1. updatemon runs daily (automated) → sends Gotify notifications
2. Review notifications to see what needs updating
3. Run updatectl manually to apply updates (or enable automated schedule)
