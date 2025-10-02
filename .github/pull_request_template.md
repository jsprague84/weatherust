Title: <type(scope)>: short summary

Summary
- What does this change do? Why?

Changes
- Key changes in bullets (code, Dockerfile, CI, docs)

Image/CI
- New image name (if any): `ghcr.io/<owner>/<image>`
- Workflow file: `.github/workflows/docker-<image>.yml`
- Release/tag notes (if not using latest):

Compose
- docker-compose.yml changes (service + Ofelia labels):
  - Service name:
  - Schedule:
  - Command:
  - Env keys:

Env Vars
- New env keys (documented in `.env.example`):

Testing
- How did you test locally? (`cargo run`, Docker build/run, Ofelia run)

Checklist
- [ ] Updated `.env.example` if adding envs
- [ ] Added/updated README or FEATURES docs
- [ ] CI passes for all workflows
- [ ] Image pulls/runs via `docker compose`
