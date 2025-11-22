# Weatherust System Architecture

## Overview

Weatherust is a Rust-based infrastructure monitoring and automation platform designed as a microservices architecture running in Docker containers, scheduled by Ofelia.

## System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Docker Host                              │
│                                                                  │
│  ┌──────────────┐                                               │
│  │   Ofelia     │  ← Cron-like scheduler                        │
│  │  Scheduler   │                                               │
│  └──────┬───────┘                                               │
│         │ Executes on schedule                                  │
│         ├──────────────┬──────────────┬──────────────┐         │
│         ▼              ▼              ▼              ▼         │
│  ┌─────────────┐ ┌──────────┐ ┌─────────────┐ ┌───────────┐  │
│  │ weatherust  │ │speedynotify│ │  healthmon  │ │ updatemon │  │
│  │   _runner   │ │  _runner   │ │  _runner    │ │  _runner  │  │
│  └─────────────┘ └──────────┘ └─────────────┘ └───────────┘  │
│         │              │              │              │         │
│         └──────────────┴──────────────┴──────────────┘         │
│                        │                                        │
│                        ▼                                        │
│              ┌──────────────────┐                              │
│              │  Notification    │                              │
│              │   Services       │                              │
│              │  ┌────────────┐  │                              │
│              │  │  Gotify    │  │                              │
│              │  └────────────┘  │                              │
│              │  ┌────────────┐  │                              │
│              │  │  ntfy.sh   │  │                              │
│              │  └────────────┘  │                              │
│              └──────────────────┘                              │
│                                                                 │
│  ┌─────────────────┐           ┌──────────────────┐           │
│  │ updatectl_runner│           │updatectl_webhook │           │
│  │  (Interactive)  │           │   (Web Server)   │           │
│  └────────┬────────┘           └────────┬─────────┘           │
│           │                              │                     │
│           ├──────────────────────────────┤                     │
│           │      SSH to Remote Servers   │                     │
│           ▼                              │                     │
│  ┌──────────────────┐                    │                     │
│  │  Docker Socket   │◄───────────────────┘                     │
│  │   (RW Access)    │                                          │
│  └──────────────────┘                                          │
│                                                                 │
│           │                                                     │
│           ▼                                                     │
│  ┌──────────────────┐                                          │
│  │ Remote Servers   │                                          │
│  │  via SSH         │                                          │
│  └──────────────────┘                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Component Architecture

### 1. Monitoring Services (Read-Only)

#### weatherust
- **Purpose**: Weather monitoring and notifications
- **Schedule**: Daily at 05:30 (configurable)
- **Dependencies**: OpenWeatherMap API
- **Notifications**: Gotify, ntfy.sh
- **Key Features**:
  - Fetches current weather and 7-day forecast
  - ZIP code or city-based location
  - Configurable units (imperial/metric)

#### speedynotify
- **Purpose**: Internet speed testing
- **Schedule**: Daily at 02:10 (configurable)
- **Dependencies**: Ookla Speedtest CLI
- **Notifications**: Gotify, ntfy.sh with threshold alerts
- **Key Features**:
  - Download/upload speed monitoring
  - Configurable thresholds for alerts
  - Historical tracking capability

#### healthmon
- **Purpose**: Docker container health monitoring
- **Schedule**: Every 5 minutes (configurable)
- **Dependencies**: Docker socket (read-only)
- **Notifications**: Gotify, ntfy.sh
- **Key Features**:
  - Container state monitoring (running/stopped)
  - Health check status tracking
  - CPU/Memory threshold alerts (configurable)
  - Ignore list for noisy containers

#### updatemon
- **Purpose**: System and Docker update monitoring
- **Schedule**: Daily at 03:00 (configurable)
- **Dependencies**: Docker socket (read-only), SSH access to servers
- **Notifications**: Gotify, ntfy.sh with actionable buttons
- **Key Features**:
  - OS package update detection (apt/dnf/pacman)
  - Docker image update detection via registry API
  - Multi-server parallel checking
  - No modifications - read-only operations

### 2. Action Services (Write Operations)

#### updatectl
- **Purpose**: System and Docker update application
- **Access Patterns**:
  - Interactive CLI via `updatectl_runner`
  - Webhook-triggered via `updatectl_webhook`
  - Manual execution via Docker Compose exec
- **Dependencies**: Docker socket (read-write), SSH access to servers
- **Notifications**: Gotify, ntfy.sh
- **Key Features**:
  - OS package updates (apt/dnf/pacman)
  - Docker image updates and pulls
  - Docker cleanup operations (images, networks, volumes, build cache)
  - OS cleanup operations (package cache, autoremove)
  - Dry-run mode for safety
  - Interactive confirmation prompts
  - Server name resolution
  - Webhook API for automation

#### updatectl_webhook
- **Purpose**: HTTP webhook server for remote-triggered updates
- **Port**: 8080 (exposed via Traefik)
- **Security**: Token-based authentication (constant-time comparison)
- **Key Features**:
  - RESTful API for update operations
  - Secure token verification
  - Action buttons in ntfy notifications
  - Rate limiting (TODO)
  - Request validation

### 3. Shared Library (common)

The `common` crate provides shared functionality used by all services:

```
common/
├── error.rs          # Structured error types
├── constants.rs      # Configuration constants
├── security.rs       # Security utilities
├── retry.rs          # Retry logic with backoff
├── executor.rs       # SSH remote execution
├── metrics.rs        # Metrics recording
└── lib.rs           # Notification functions, Server types
```

#### Key Modules:

**error.rs** - Domain-specific error types:
- `NotificationError` - Gotify/ntfy failures
- `RemoteExecutionError` - SSH execution failures
- `DockerError` - Docker API errors
- `ServerConfigError` - Server parsing errors
- `UpdateError` - Update operation errors
- `WebhookError` - Webhook authentication/processing
- `HealthCheckError` - Container health errors

**constants.rs** - Configuration values:
- Notification priorities
- Timeout values (SSH, Docker, HTTP)
- Retry configuration
- Health check thresholds
- Environment variable names

**security.rs** - Security primitives:
- Constant-time token comparison
- Webhook token verification with audit logging

**retry.rs** - Resilience patterns:
- Exponential backoff retry logic
- HTTP-specific retry helpers
- Configurable retry policies

**executor.rs** - Remote operations:
- SSH connection pooling (TODO)
- Remote command execution
- Output capture and parsing

## Data Flow

### Notification Flow

```
Service → http_client()
          ↓
    ┌─────┴─────┐
    ▼           ▼
send_gotify  send_ntfy
    │           │
    ▼           ▼
  Gotify     ntfy.sh
  Server     Server
    │           │
    └─────┬─────┘
          ▼
    Mobile/Desktop
    Notifications
```

### Update Monitoring Flow

```
updatemon
    ↓
Parallel Tasks (per server)
    ├─→ SSH → check_os_updates
    │         └─→ apt/dnf/pacman check
    │
    └─→ SSH → check_docker_updates
              └─→ Registry API manifest comparison
    ↓
Aggregate Results
    ↓
Format Notification
    ├─→ Gotify (full details)
    └─→ ntfy (per-server with action buttons)
```

### Update Application Flow

```
updatectl command
    ↓
Parse Arguments
    ↓
Resolve Servers (name → connection string)
    ↓
Confirmation Prompt (unless --yes)
    ↓
Parallel Execution (per server)
    ├─→ SSH → execute_os_update
    │         └─→ apt/dnf/pacman upgrade
    │
    └─→ SSH → execute_docker_update
              └─→ docker pull + docker-compose up
    ↓
Aggregate Results
    ↓
Report Success/Failures
    └─→ Notifications
```

### Webhook Flow

```
External System
    ↓
HTTP POST /webhook/update
    ↓
Verify Token (constant-time)
    ↓
Parse Payload
    ↓
Resolve Server
    ↓
Execute Update Operation
    ↓
Return Status
```

## Security Model

### Authentication

1. **Webhook API**: Token-based (constant-time comparison prevents timing attacks)
2. **SSH Access**: Key-based authentication (single key mounted per service)
3. **Docker Socket**: Unix socket with file permissions

### Authorization

1. **Container Permissions**:
   - `healthmon`: Docker socket read-only
   - `updatemon`: Docker socket read-only, SSH read-only operations
   - `updatectl`: Docker socket read-write, SSH write operations

2. **SSH Key Exposure**:
   - Only specific key mounted (not entire .ssh directory)
   - Read-only mount
   - Principle of least privilege

### Audit Trail

1. **Structured Logging**: All operations logged with context
2. **Metrics**: Success/failure counts per operation
3. **Notification Records**: All notifications recorded in logs
4. **Webhook Logs**: Failed auth attempts logged with request IDs

## Configuration

### Environment Variables

Organized in `common/src/constants.rs` under `env` module:

```rust
pub mod env {
    // Gotify
    pub const GOTIFY_URL: &str = "GOTIFY_URL";
    pub const GOTIFY_KEY_FILE: &str = "GOTIFY_KEY_FILE";
    pub const GOTIFY_DEBUG: &str = "GOTIFY_DEBUG";

    // ntfy.sh
    pub const NTFY_URL: &str = "NTFY_URL";
    pub const NTFY_AUTH: &str = "NTFY_AUTH";
    pub const NTFY_DEBUG: &str = "NTFY_DEBUG";

    // Service-specific keys
    // WEATHERUST_GOTIFY_KEY, UPDATEMON_GOTIFY_KEY, etc.
    // WEATHERUST_NTFY_TOPIC, UPDATEMON_NTFY_TOPIC, etc.
}
```

### Service Configuration Hierarchy

1. **CLI Arguments** (highest priority)
2. **Environment Variables** (`.env` file)
3. **Default Values** (in code via constants)

## Docker Architecture

### Image Strategy

- **Base**: Debian 12 (bookworm)
- **Runtime**: Distroless (cc variant) - minimal attack surface
- **User**: Non-root by default
- **Multi-arch**: amd64, arm64 (built via GitHub Actions)

### Container Patterns

#### Runner Pattern
Long-running containers with `sleep infinity`:
- Allows Ofelia to exec commands with full environment
- Inherits env_file configuration
- No restart loops
- Clean shutdown

#### Webhook Pattern
Standalone web server:
- Dedicated container for HTTP endpoints
- Traefik integration for HTTPS
- Health check disabled (distroless has no curl/wget)
- External monitoring via Traefik

### Volume Mounts

```yaml
# Docker socket (varies by service)
- /var/run/docker.sock:/var/run/docker.sock:ro  # read-only
- /var/run/docker.sock:/var/run/docker.sock:rw  # read-write

# SSH key (security-conscious)
- ${UPDATE_SSH_KEY}:/ssh/id_key:ro  # specific key only

# Environment file (for Ofelia)
- ${ENV_FILE_HOST_PATH}:/ofelia/.env:ro
```

## Scheduling

### Ofelia Configuration

Ofelia labels on the `ofelia` service define all schedules:

```yaml
# Format: "sec min hour day month weekday"
ofelia.job-exec.weatherust.schedule: "0 30 5 * * *"     # 05:30 daily
ofelia.job-exec.speedynotify.schedule: "0 10 2 * * *"   # 02:10 daily
ofelia.job-exec.healthmon-health.schedule: "0 */5 * * * *"  # Every 5 min
ofelia.job-exec.updatemon.schedule: "0 0 3 * * *"       # 03:00 daily
```

### Job Execution Pattern

```yaml
ofelia.job-exec.<job-name>.container: <container-name>
ofelia.job-exec.<job-name>.command: /app/<binary> <args>
ofelia.job-exec.<job-name>.schedule: <cron-expression>
```

## Performance Considerations

### Concurrency

1. **Parallel Server Checks**: `updatemon` and `updatectl` use `tokio::spawn` for parallel execution
2. **Connection Pooling**: TODO - SSH connection reuse
3. **Async I/O**: All network operations use async/await
4. **Bounded Concurrency**: Respects system limits (TODO: configurable)

### Resource Usage

1. **Memory**: Minimal - Rust's zero-cost abstractions
2. **CPU**: Low - event-driven architecture
3. **Disk**: Logs only (stdout/stderr, captured by Docker)
4. **Network**: Burst pattern during checks, idle otherwise

### Optimization Strategies

1. **Retry with Backoff**: Prevents overwhelming services
2. **Timeout Enforcement**: Prevents hanging operations
3. **Lazy Evaluation**: Connections established only when needed
4. **Structured Logging**: Minimizes overhead with `skip` parameters

## Failure Modes and Recovery

### Service Failures

1. **Notification Failures**: Graceful degradation - continue even if notifications fail
2. **SSH Failures**: Per-server isolation - one server failure doesn't affect others
3. **Docker API Failures**: Retry with exponential backoff
4. **Network Failures**: Automatic retry with configurable limits

### Data Integrity

1. **Idempotent Operations**: Safe to retry
2. **Dry-Run Mode**: Preview before applying changes
3. **Confirmation Prompts**: Prevent accidental operations
4. **Audit Logging**: Track all operations

### Disaster Recovery

1. **Stateless Services**: No local state to backup
2. **Configuration in .env**: Easy to restore
3. **Docker Images**: Versioned and immutable
4. **Git Repository**: Source of truth for all code

## Monitoring and Observability

### Metrics

Implemented via `metrics` crate:
```rust
metrics::counter!("updatemon.servers.checked").increment(1);
metrics::histogram!("updatemon.check.duration").record(duration);
```

### Structured Logging

Using `tracing` crate:
```rust
#[instrument(skip(sensitive), fields(server = %name))]
async fn check_server(name: &str, sensitive: &str) -> Result<()>
```

### Distributed Tracing

- **Current**: Basic span creation with `#[instrument]`
- **Future**: OpenTelemetry integration for distributed tracing

### Health Checks

- **Healthmon Service**: Monitors all containers
- **Docker Health Checks**: Disabled for distroless images
- **Traefik**: HTTP endpoint monitoring for webhook service

## Future Architecture Considerations

### Planned Improvements

1. **Connection Pooling**: Reuse SSH connections for multiple operations
2. **Rate Limiting**: Webhook API rate limiting
3. **Caching**: Short-term cache for registry API calls
4. **Metrics Dashboard**: Prometheus/Grafana integration
5. **Notification Trait**: Abstract notification backends
6. **Plugin System**: Extensible check/update providers

### Scalability

Current design supports:
- **Horizontal**: Multiple monitoring instances (different server sets)
- **Vertical**: Efficient resource usage scales with server count
- **Limitations**: Serial SSH operations per server (can be parallelized)

### Multi-Tenancy

Not currently supported, but possible with:
- Per-tenant .env files
- Isolated Docker Compose stacks
- Separate SSH keys per tenant
- Namespace separation in notifications

## References

- [Docker Compose Reference](./docker-compose.yml)
- [Ofelia Documentation](https://github.com/mcuadros/ofelia)
- [Service READMEs](./updatectl/README.md)
- [Development Guide](./CLAUDE_CODE_GUIDE.md)
- [Modernization Summary](./MODERNIZATION_SUMMARY.md)
