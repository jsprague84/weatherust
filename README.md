**Weatherust**

- Rust CLI that pulls current weather and 7-day outlook from OpenWeatherMap and optionally sends a Gotify notification.
- Supports ZIP or free-form location (e.g., "City,ST,US").
- Designed to run non-interactively under cron or systemd timers.

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

**Docker**

- Compose (preferred):
  - Ensure `.env` exists with secrets.
  - The stack uses `ghcr.io/jsprague84/weatherust:latest` by default.
  - Pull/update and start: `docker compose pull && docker compose up -d`
  - Manual run (ad-hoc): `docker compose run --rm weatherust --zip 52726 --units imperial --quiet`

**Scheduling with Ofelia (in Docker)**

- The compose stack includes an `ofelia` service that schedules a one-off run at 05:30 daily.
- Defaults in compose:
  - Location: ZIP `52726` with `--units imperial --quiet`.
  - Timezone: `America/Chicago`.
- Configure secrets in `.env`:
  - `OWM_API_KEY`, `GOTIFY_KEY`, `GOTIFY_URL`.
- Start the stack:
  - `docker compose up -d --build` (builds `weatherust:local` and starts Ofelia)
- Logs:
  - `docker compose logs -f ofelia` (shows job runs and any errors)

Adjusting schedule:
- The schedule is defined as a label on the `ofelia` service in `docker-compose.yml`:
  - `ofelia.job-run.weatherust.schedule: "0 30 5 * * *"` (sec min hour day month weekday)
- Update it as needed and re-run `docker compose up -d`.

**GitHub / CI**

- This repo includes `.github/workflows/docker.yml` to build and publish a multi-arch Docker image to GHCR.
- Steps:
  - Create a GitHub repository named `weatherust` and push this repo.
  - Ensure the repository visibility is set as desired.
  - After pushing to default branch, CI will publish `ghcr.io/<org-or-user>/weatherust:latest`.
  - docker-compose here is already set to `ghcr.io/jsprague84/weatherust:latest` for both the service and Ofelia job.
  - If the GHCR package is private, configure a registry login on the host running compose.

**Security/Secrets**

- `.env` is gitignored. Do not commit real API keys.
- If keys were previously committed, rotate them in OWM and Gotify.

**CLI Reference**

- Flags:
  - `--zip <ZIP[,CC]>` e.g., `52726` or `52726,US`.
  - `--location <free-form>` e.g., `Davenport,IA,US`.
  - `--units <imperial|metric>` (default `imperial`).
  - `--quiet` suppresses stdout (useful in scheduled runs).
