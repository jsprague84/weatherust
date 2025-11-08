# CLI Commands Reference

Complete command reference for all services in the weatherust workspace.

## Table of Contents

- [weatherust](#weatherust) - Weather monitoring
- [speedynotify](#speedynotify) - Internet speed test monitoring
- [healthmon](#healthmon) - Docker container health monitoring
- [updatemon](#updatemon) - Multi-server update monitoring
- [updatectl](#updatectl) - Multi-server update controller & cleanup tool

---

## weatherust

Weather monitoring with OpenWeatherMap integration.

### Basic Usage

```bash
# Using docker compose (recommended for production)
docker compose run --rm weatherust --zip 52726 --units imperial

# Using cargo (local development)
cargo run --bin weatherust -- --zip 52726 --units imperial
```

### Command-Line Options

| Option | Description | Example |
|--------|-------------|---------|
| `--zip <ZIP>` | ZIP code (optional country code) | `--zip 52726` or `--zip 52726,US` |
| `--location <LOCATION>` | Free-form location string | `--location "Davenport,IA,US"` |
| `--units <UNITS>` | Temperature units (imperial/metric) | `--units metric` |
| `--quiet` | Suppress stdout output | `--quiet` |

### Examples

```bash
# Imperial units (Fahrenheit)
weatherust --zip 52726 --units imperial

# Metric units (Celsius)
weatherust --zip 10001 --units metric

# Free-form location
weatherust --location "London,UK" --units metric

# Quiet mode (notifications only)
weatherust --zip 52726 --quiet
```

### Environment Defaults

```bash
# Set defaults in .env
DEFAULT_ZIP=52726
DEFAULT_UNITS=imperial

# Now you can run without flags
weatherust
```

---

## speedynotify

Internet speed test monitoring using Ookla Speedtest CLI.

### Basic Usage

```bash
# Using docker compose
docker compose run --rm speedynotify --min-down 300 --min-up 20

# Using cargo
cargo run --bin speedynotify -- --min-down 300 --min-up 20
```

### Command-Line Options

| Option | Description | Example |
|--------|-------------|---------|
| `--min-down <MBPS>` | Minimum acceptable download speed | `--min-down 300` |
| `--min-up <MBPS>` | Minimum acceptable upload speed | `--min-up 20` |
| `--server-id <ID>` | Pin to specific Speedtest server | `--server-id 12345` |
| `--quiet` | Suppress stdout output | `--quiet` |

### Examples

```bash
# Basic speed test with thresholds
speedynotify --min-down 300 --min-up 20

# Pin to specific server
speedynotify --min-down 500 --min-up 50 --server-id 12345

# Quiet mode
speedynotify --quiet --min-down 300 --min-up 20
```

### Environment Defaults

```bash
# Set defaults in .env
SPEEDTEST_MIN_DOWN=300
SPEEDTEST_MIN_UP=20
SPEEDTEST_SERVER_ID=12345  # optional

# Now you can run without flags
speedynotify
```

---

## healthmon

Docker container health and resource usage monitoring.

### Basic Usage

```bash
# Using docker compose (recommended)
docker compose exec healthmon_runner /app/healthmon health

# Using cargo
cargo run --bin healthmon -- health
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--quiet` | Suppress stdout output | false |
| `--cpu-warn-pct <PCT>` | CPU warning threshold | 85 or `CPU_WARN_PCT` env |
| `--mem-warn-pct <PCT>` | Memory warning threshold | 90 or `MEM_WARN_PCT` env |
| `--notify-always` | Notify even when all OK | false or `HEALTH_NOTIFY_ALWAYS` env |
| `--ignore <NAMES>` | Ignore containers (comma-separated) | `HEALTHMON_IGNORE` env |

### Examples

```bash
# Basic health check
healthmon health

# Custom thresholds
healthmon health --cpu-warn-pct 90 --mem-warn-pct 95

# Always notify (even when everything is OK)
healthmon health --notify-always

# Ignore specific containers
healthmon health --ignore "ofelia,traefik,portainer"

# Quiet mode
healthmon health --quiet

# Combine options
healthmon health --quiet --cpu-warn-pct 90 --ignore "ofelia"
```

---

## updatemon

Multi-server update monitoring (read-only).

### Basic Usage

```bash
# Using docker compose (recommended)
docker compose exec updatemon_runner /app/updatemon --docker

# Using cargo
cargo run --bin updatemon -- --docker
```

### Command-Line Options

| Option | Description |
|--------|-------------|
| `--local` | Include localhost in check |
| `--servers <LIST>` | Comma-separated server list (overrides `UPDATE_SERVERS`) |
| `--docker` | Check Docker images for updates (default: true) |
| `--ssh-key <PATH>` | SSH key path (overrides `UPDATE_SSH_KEY`) |
| `--quiet` | Suppress stdout output |

### Examples

```bash
# Check all configured servers (from UPDATE_SERVERS env)
updatemon --docker

# Check localhost only
updatemon --local --docker

# Check specific servers
updatemon --servers "Cloud VM1,Cloud VM2" --docker

# Check local + specific remote servers
updatemon --local --servers "Cloud VM1" --docker

# OS updates only (skip Docker)
updatemon --local

# Quiet mode (notifications only)
updatemon --docker --quiet

# Custom SSH key
updatemon --docker --ssh-key /path/to/custom_key
```

### Server Format

```bash
# In UPDATE_SERVERS environment variable:

# With name
Cloud VM1:ubuntu@cloud-vm1.js-node.com

# Without name (uses hostname)
ubuntu@192.168.1.10

# Localhost (NEW)
docker-vm:local
```

---

## updatectl

Multi-server update controller and cleanup tool (applies changes).

### Discovery Commands

```bash
# List configured servers
updatectl list servers

# Show usage examples
updatectl list examples
```

### Update Commands

#### OS Package Updates

```bash
# Dry-run (preview only, safe)
updatectl os --dry-run --local

# Update localhost
updatectl os --yes --local

# Update specific server
updatectl os --yes --servers "Cloud VM1"

# Update multiple servers
updatectl os --yes --servers "Cloud VM1,Cloud VM2"

# Update all configured servers
updatectl os --yes
```

#### Docker Image Updates

```bash
# Update all Docker images on localhost
updatectl docker --all --yes --local

# Update specific images
updatectl docker --images nginx:latest,redis:latest --yes --local

# Update all images on specific server
updatectl docker --all --yes --servers "Cloud VM1"

# Dry-run first (preview)
updatectl docker --all --dry-run --local
```

#### Combined Updates

```bash
# Update OS + Docker on localhost
updatectl all --yes --local

# Update OS + Docker on specific server
updatectl all --yes --servers "Cloud VM1"

# Update all configured servers
updatectl all --yes

# Dry-run first
updatectl all --dry-run
```

### Cleanup Commands

#### Docker Cleanup

```bash
# Analysis only (safe, default)
updatectl clean-docker --local

# Execute with conservative profile (safest)
# Removes: dangling images + unused networks
updatectl clean-docker --local --execute --profile conservative

# Execute with moderate profile
# Removes: dangling images + unused networks + build cache
updatectl clean-docker --local --execute --profile moderate

# Execute with aggressive profile
# Removes: dangling images + unused networks + build cache + old containers (30+ days)
updatectl clean-docker --local --execute --profile aggressive

# Remote server cleanup
updatectl clean-docker --servers "Cloud VM1" --execute --profile conservative

# Quiet mode (notifications only)
updatectl clean-docker --local --quiet
```

#### OS Cleanup

```bash
# Analysis only (default)
updatectl clean-os --local

# Clean package cache only
updatectl clean-os --local --execute --cache

# Remove unused packages only
updatectl clean-os --local --execute --autoremove

# Clean all (cache + autoremove)
updatectl clean-os --local --execute --all

# Remote server cleanup
updatectl clean-os --servers "Cloud VM1" --execute --all

# Dry-run first
updatectl clean-os --local --dry-run --all
```

### Webhook Server

```bash
# Start webhook server (docker compose handles this)
updatectl serve --port 8080

# Manual start (for testing)
cargo run --bin updatectl -- serve --port 8080
```

### Command-Line Options Reference

#### Update Commands Options

| Option | Description |
|--------|-------------|
| `--local` | Target localhost only |
| `--servers <LIST>` | Target specific servers (comma-separated) |
| `--yes` / `-y` | Skip confirmation prompt |
| `--dry-run` | Preview changes without applying |
| `--quiet` | Suppress stdout output |
| `--ssh-key <PATH>` | SSH key path (overrides `UPDATE_SSH_KEY`) |

#### Docker Update Specific Options

| Option | Description |
|--------|-------------|
| `--all` | Update all Docker images |
| `--images <LIST>` | Update specific images (comma-separated) |

#### Docker Cleanup Specific Options

| Option | Description |
|--------|-------------|
| `--profile <PROFILE>` | Cleanup profile: conservative, moderate, aggressive |
| `--execute` | Actually perform cleanup (default is analysis only) |

#### OS Cleanup Specific Options

| Option | Description |
|--------|-------------|
| `--cache` | Clean package manager cache |
| `--autoremove` | Remove unused packages |
| `--all` | Clean cache + autoremove |
| `--execute` | Actually perform cleanup (default is analysis only) |

### Full Examples

```bash
# Safe workflow: dry-run first, then execute
updatectl all --dry-run --local                    # Preview
updatectl all --yes --local                         # Execute

# Update specific server with confirmation
updatectl os --servers "Cloud VM1"                  # Prompts for confirmation
updatectl os --yes --servers "Cloud VM1"            # Auto-confirm

# Cleanup workflow
updatectl clean-docker --local                      # Analyze
updatectl clean-docker --local --execute --profile conservative  # Execute safe cleanup

# Multi-server operations
updatectl os --yes --servers "Cloud VM1,Cloud VM2"  # Update 2 servers
updatectl os --yes                                  # Update ALL servers

# Advanced: Local + remote in one command
updatectl docker --all --yes --local --servers "Cloud VM1"
```

---

## Common Patterns

### Running in Docker Compose

All services have long-running "runner" containers that make CLI usage easier:

```bash
# Pattern: docker compose exec <service>_runner /app/<binary> <command>

# healthmon
docker compose exec healthmon_runner /app/healthmon health

# updatemon
docker compose exec updatemon_runner /app/updatemon --docker

# updatectl
docker compose exec updatectl_runner /app/updatectl list servers
```

### Running One-Off Jobs

For services without runners, use `run --rm`:

```bash
# weatherust
docker compose run --rm weatherust --zip 52726

# speedynotify
docker compose run --rm speedynotify --min-down 300 --min-up 20
```

### Using Shell Aliases

Add to `~/.bashrc` for easier CLI usage (see [BASH_ALIASES.md](BASH_ALIASES.md)):

```bash
alias healthmon='docker compose -f ~/path/to/docker-compose.yml exec healthmon_runner /app/healthmon'
alias updatemon='docker compose -f ~/path/to/docker-compose.yml exec updatemon_runner /app/updatemon'
alias updatectl='docker compose -f ~/path/to/docker-compose.yml exec updatectl_runner /app/updatectl'
```

Then simply:

```bash
healthmon health
updatemon --docker
updatectl list servers
```

### Quiet Mode for Automation

All services support `--quiet` for use in cron/Ofelia schedules:

```bash
# Notifications only, no stdout
weatherust --quiet
speedynotify --quiet
healthmon health --quiet
updatemon --docker --quiet
updatectl os --yes --local --quiet
```

### Dry-Run for Safety

Preview changes before applying:

```bash
# Update commands
updatectl os --dry-run --local
updatectl docker --all --dry-run --local
updatectl all --dry-run --local

# Cleanup commands (default is analysis-only, so dry-run not needed)
updatectl clean-docker --local                    # Already safe (analysis only)
updatectl clean-docker --local --execute ...      # Add --execute to actually clean
```

---

## Environment Variables

All services share common environment variable patterns. See `.env.example` for complete list.

### Notification Backends

```bash
# Gotify (self-hosted)
GOTIFY_URL=https://gotify.example.com/message
WEATHERUST_GOTIFY_KEY=token1
HEALTHMON_GOTIFY_KEY=token2
UPDATEMON_GOTIFY_KEY=token3
UPDATECTL_GOTIFY_KEY=token4
SPEEDY_GOTIFY_KEY=token5

# ntfy.sh (hosted or self-hosted)
NTFY_URL=https://ntfy.sh
WEATHERUST_NTFY_TOPIC=weather
HEALTHMON_NTFY_TOPIC=docker-health
UPDATEMON_NTFY_TOPIC=updates
UPDATECTL_NTFY_TOPIC=update-actions
SPEEDY_NTFY_TOPIC=speedtest
```

### Server Configuration (updatemon & updatectl)

```bash
# Server list
UPDATE_SERVERS=docker-vm:local,Cloud VM1:ubuntu@cloud-vm1.js-node.com

# SSH key for remote servers
UPDATE_SSH_KEY=/home/ubuntu/.ssh/id_ed25519

# Optional: Customize localhost display
UPDATE_LOCAL_NAME=docker-vm
UPDATE_LOCAL_DISPLAY=192.168.1.100
```

### Service-Specific Defaults

```bash
# weatherust
OWM_API_KEY=your_openweathermap_key
DEFAULT_ZIP=52726
DEFAULT_UNITS=imperial

# speedynotify
SPEEDTEST_MIN_DOWN=300
SPEEDTEST_MIN_UP=20

# healthmon
CPU_WARN_PCT=85
MEM_WARN_PCT=90
HEALTH_NOTIFY_ALWAYS=false
HEALTHMON_IGNORE=ofelia,traefik
```

---

## Getting Help

Each binary supports `--help`:

```bash
weatherust --help
speedynotify --help
healthmon --help
healthmon health --help
updatemon --help
updatectl --help
updatectl os --help
updatectl clean-docker --help
```

For more detailed documentation, see the README in each service directory:

- [weatherust/README.md](README.md)
- [healthmon/README.md](healthmon/README.md)
- [updatemon/README.md](updatemon/README.md)
- [updatectl/README.md](updatectl/README.md)
