# healthmon - Docker Container Health Monitor

Monitor Docker container health, running status, and resource usage (CPU/memory) with notifications via Gotify and/or ntfy.sh.

## What It Does

healthmon checks Docker containers for issues and sends alerts when problems are detected:

- **Health Status**: Detect containers with failing health checks
- **Running State**: Alert when containers are stopped unexpectedly
- **CPU Usage**: Warn when containers exceed CPU thresholds
- **Memory Usage**: Warn when containers exceed memory thresholds
- **Flexible Notifications**: Support for both Gotify and ntfy.sh
- **Container Filtering**: Ignore specific containers by name, ID, or service

## Quick Start

### 1. Configure Notifications

Add notification settings to your `.env` file:

```bash
# Choose Gotify, ntfy.sh, or both

# Gotify (self-hosted)
GOTIFY_URL=https://gotify.example.com/message
HEALTHMON_GOTIFY_KEY=your_healthmon_token

# ntfy.sh (hosted or self-hosted)
NTFY_URL=https://ntfy.sh
HEALTHMON_NTFY_TOPIC=docker-health

# Optional: Customize thresholds
CPU_WARN_PCT=85   # Alert when container CPU exceeds 85%
MEM_WARN_PCT=90   # Alert when container memory exceeds 90%

# Optional: Ignore specific containers
HEALTHMON_IGNORE=ofelia,traefik
```

### 2. Start the Service

```bash
docker compose up -d healthmon_runner
```

### 3. Run Manually

```bash
# Check all containers
docker compose exec healthmon_runner /app/healthmon health

# Quiet mode (notifications only, no stdout)
docker compose exec healthmon_runner /app/healthmon health --quiet
```

## Automated Monitoring via Ofelia

healthmon is configured to run every 5 minutes in `docker-compose.yml`:

```yaml
- "ofelia.job-exec.healthmon-health.schedule=0 */5 * * * *"
- "ofelia.job-exec.healthmon-health.container=healthmon_runner"
- "ofelia.job-exec.healthmon-health.command=/app/healthmon health --quiet"
```

This sends automatic notifications when container issues are detected.

## Command-Line Options

### Basic Usage

```bash
healthmon health [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--quiet` | Suppress stdout output (notifications only) | false |
| `--cpu-warn-pct <PCT>` | CPU warning threshold percentage | 85 (or `CPU_WARN_PCT` env) |
| `--mem-warn-pct <PCT>` | Memory warning threshold percentage | 90 (or `MEM_WARN_PCT` env) |
| `--notify-always` | Send notification even when all containers are OK | false (or `HEALTH_NOTIFY_ALWAYS` env) |
| `--ignore <NAMES>` | Ignore specific containers (comma-separated) | `HEALTHMON_IGNORE` env |

### Examples

```bash
# Custom CPU threshold
healthmon health --cpu-warn-pct 90

# Custom memory threshold
healthmon health --mem-warn-pct 95

# Always notify (even when everything is OK)
healthmon health --notify-always

# Ignore specific containers
healthmon health --ignore "weatherust_runner,speedynotify_runner"

# Combine options
healthmon health --quiet --cpu-warn-pct 90 --mem-warn-pct 95
```

## Notification Examples

### When Issues Are Detected

```
Title: Docker Health: Issues

Message:
1 issue(s) detected
postgres_main (a1b2c3d4e5f6) | CPU 92.3% | MEM 78.5% | state: running | health: unhealthy
```

### When Everything Is OK

```
Title: Docker Health: OK

Message:
All containers OK (8 checked)
```

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `HEALTHMON_GOTIFY_KEY` | No* | Gotify API token for this service |
| `HEALTHMON_NTFY_TOPIC` | No* | ntfy.sh topic for this service |
| `GOTIFY_URL` | If using Gotify | Gotify server URL |
| `NTFY_URL` | If using ntfy | ntfy server URL (defaults to https://ntfy.sh) |
| `CPU_WARN_PCT` | No | CPU warning threshold (default: 85) |
| `MEM_WARN_PCT` | No | Memory warning threshold (default: 90) |
| `HEALTH_NOTIFY_ALWAYS` | No | Notify even when OK (default: false) |
| `HEALTHMON_IGNORE` | No | Comma-separated list of containers to ignore |

\* At least one notification backend (Gotify or ntfy) should be configured

### Notification Backends

healthmon supports two notification backends simultaneously:

**Gotify (Self-Hosted)**
```bash
GOTIFY_URL=https://gotify.example.com/message
HEALTHMON_GOTIFY_KEY=your_token_here
```

**ntfy.sh (Hosted or Self-Hosted)**
```bash
NTFY_URL=https://ntfy.sh  # or your self-hosted instance
HEALTHMON_NTFY_TOPIC=docker-health
```

You can use one or both. Messages are sent to all configured backends.

### Ignoring Containers

Ignore containers by name, ID, short ID, or Docker Compose service name:

**Via Environment Variable:**
```bash
HEALTHMON_IGNORE=ofelia,traefik,portainer
```

**Via CLI Flag:**
```bash
healthmon health --ignore "weatherust_runner,speedynotify_runner"
```

**Combined (CLI adds to env list):**
```bash
# .env
HEALTHMON_IGNORE=ofelia,traefik

# CLI adds more
healthmon health --ignore "test_container"
# Ignores: ofelia, traefik, test_container
```

## Docker Requirements

healthmon needs read-only access to the Docker socket:

```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock:ro
```

This allows it to:
- List all containers
- Inspect container state and health
- Read CPU and memory statistics

## Health Check Criteria

A container is considered problematic if any of these conditions are met:

1. **Not Running**: Container state is not "running"
2. **Unhealthy**: Docker health check reports "unhealthy" or "starting"
3. **High CPU**: CPU usage exceeds `CPU_WARN_PCT` threshold
4. **High Memory**: Memory usage exceeds `MEM_WARN_PCT` threshold

### Health Status Values

- `healthy` - Health check passing (no alert)
- `unhealthy` - Health check failing (alert)
- `starting` - Health check still initializing (alert)
- `none` - No health check configured (no alert for this)

## Resource Monitoring

### CPU Calculation

CPU percentage is calculated using Docker's CPU statistics:
- Compares current CPU usage to previous sample
- Accounts for number of CPU cores available
- Single sample per check (non-streaming)
- 2-second timeout per container

### Memory Calculation

Memory percentage is calculated as:
```
(container memory usage / container memory limit) * 100
```

If no memory limit is set, memory percentage is not reported.

## Scheduling Recommendations

### Every 5 Minutes (Default)

```yaml
ofelia.job-exec.healthmon-health.schedule=0 */5 * * * *
```

Good balance between responsiveness and system load.

### Every Minute (Aggressive)

```yaml
ofelia.job-exec.healthmon-health.schedule=0 * * * * *
```

Use for critical production environments requiring immediate alerts.

### Every 15 Minutes (Conservative)

```yaml
ofelia.job-exec.healthmon-health.schedule=0 */15 * * * *
```

Use for development environments or less critical systems.

## Troubleshooting

### No Notifications Received

1. Check notification configuration:
   ```bash
   docker compose exec healthmon_runner env | grep -E 'GOTIFY|NTFY'
   ```

2. Test notification manually:
   ```bash
   docker compose exec healthmon_runner /app/healthmon health
   ```

3. Enable debug logging:
   ```bash
   GOTIFY_DEBUG=true
   NTFY_DEBUG=true
   ```

### CPU/Memory Stats Not Showing

- Stats may be unavailable for recently started containers
- Check Docker socket permissions (should be `:ro`)
- Verify container has cgroup stats enabled

### "Permission denied" on Docker socket

```bash
# Check socket mount
docker compose exec healthmon_runner ls -la /var/run/docker.sock

# Should show: srw-rw---- 1 root docker
```

If needed, ensure the container user has Docker socket access.

### Containers Always Showing as "Unhealthy"

Check actual container health:
```bash
docker inspect <container_name> | grep -A 10 "Health"
```

Health check must be properly configured in the container's Dockerfile or docker-compose.yml.

## Comparison with Other Tools

| Feature | healthmon | updatectl | updatemon |
|---------|-----------|-----------|-----------|
| Purpose | **Monitor** container health | **Execute** updates & cleanup | **Check** for updates |
| Docker socket | Read-only | Read-write | Read-only |
| Modifies system | ❌ Never | ✅ Yes | ❌ Never |
| Typical schedule | Every 5 minutes | Weekly/manual | Daily |
| Notification key | `HEALTHMON_GOTIFY_KEY` | `UPDATECTL_GOTIFY_KEY` | `UPDATEMON_GOTIFY_KEY` |
| Primary focus | Real-time health | System maintenance | Update awareness |

## Relationship to updatectl

healthmon was formerly named `dockermon` and included Docker cleanup functionality. As of the major refactoring:

- **healthmon** - Health monitoring only (read-only)
- **updatectl** - All system modifications including:
  - OS updates (`updatectl os`)
  - Docker image updates (`updatectl docker`)
  - Docker cleanup (`updatectl clean-docker`)
  - OS cleanup (`updatectl clean-os`)

For cleanup operations, see [updatectl documentation](../updatectl).

## See Also

- [updatectl](../updatectl) - Multi-server update controller & cleanup tool
- [updatemon](../updatemon) - Multi-server update monitoring
- [weatherust](../) - Weather notifications
- [speedynotify](../speedynotify) - Internet speed test monitoring
