**Feature Scaffold**

- Goal: Keep adding small, self-contained features that reuse the shared `common` crate for `.env` loading and Gotify notifications, each shipping as its own Docker image and optional Ofelia-scheduled job.

**Quick Scaffold**

- Run: `scripts/scaffold_feature.sh <name> "Short description"`
- Generates:
  - `<name>/` Rust bin crate wired to `common`
  - `Dockerfile.<name>` for a minimal distroless image
  - `.github/workflows/docker-<name>.yml` to publish `ghcr.io/<owner>/<name>`
  - Adds the crate to `Cargo.toml` workspace members

Next steps after scaffolding:
- Implement `<name>/src/main.rs` logic. Use `common::send_gotify(&client, title, body)` to notify.
- If your binary needs OS deps, switch the runtime base image or use a two-stage Debian slim image like `Dockerfile.speedynotify`.
- Commit and push to `main` (or a feature branch); the workflow will publish your image to GHCR.
- Optionally, add an Ofelia job to `docker-compose.yml` to schedule the new image, following the existing labels pattern.

**Compose Integration (Ofelia job pattern)**

- Add a new service pointing to your GHCR image (optional for one-off runs):
  - `services.<name>.image = ghcr.io/<owner>/<name>:latest`
  - `env_file: .env` if you prefer dotenv in the container
- Schedule a job under the `ofelia` service labels:
  - `ofelia.job-run.<name>.schedule=0 0 3 * * *`
  - `ofelia.job-run.<name>.image=ghcr.io/<owner>/<name>:latest`
  - `ofelia.job-run.<name>.command=--quiet ...`
  - `ofelia.job-run.<name>.env=GOTIFY_KEY=${GOTIFY_KEY}|GOTIFY_URL=${GOTIFY_URL}|...`
  - `ofelia.job-run.<name>.volume=${ENV_FILE_HOST_PATH}:/app/.env:ro` to let dotenv work

**Env/Secrets**

- Reuse `GOTIFY_URL` and `GOTIFY_KEY` (or `GOTIFY_KEY_FILE`) from `.env`.
- Add any feature-specific envs to `.env.example` to document them.

**Guidelines**

- Keep binaries focused; push shared utilities into `common`.
- Prefer `reqwest` with `rustls` and non-root images.
- Add retries/backoff for network calls if the feature is critical.

