# Claude Code Development Guide

> **For AI Assistants**: This file provides comprehensive context for working effectively with the weatherust codebase. Read this file first when starting any development task.

## Table of Contents

- [Project Overview](#project-overview)
- [Architecture Quick Reference](#architecture-quick-reference)
- [Development Workflows](#development-workflows)
- [Code Standards](#code-standards)
- [Common Tasks](#common-tasks)
- [Testing Strategy](#testing-strategy)
- [Deployment](#deployment)

## Project Overview

**Weatherust** is a Rust workspace containing multiple tools for infrastructure monitoring, updates, and notifications:

- **weatherust** - Weather monitoring with OpenWeatherMap integration
- **speedynotify** - Internet speed test monitoring with Ookla Speedtest
- **healthmon** - Docker container health and resource monitoring
- **updatemon** - Multi-server OS and Docker update monitoring (read-only)
- **updatectl** - Multi-server update controller and cleanup tool (write operations)
- **common** - Shared library with errors, retry logic, security utilities

### Key Design Principles

1. **Workspace Architecture**: Monorepo with shared `common` crate
2. **Docker-First**: All services run in containers, scheduled by Ofelia
3. **Read/Write Separation**: `updatemon` monitors, `updatectl` applies changes
4. **Dual Notification**: Gotify and ntfy.sh support in all services
5. **Security-Focused**: Constant-time comparisons, minimal key exposure
6. **Observability**: Structured logging with tracing, metrics support

## Architecture Quick Reference

### Crate Dependency Graph

```
weatherust (binary)        ─┐
speedynotify (binary)      ─┤
healthmon (binary)         ─┼──→ common (library)
updatemon (binary)         ─┤
updatectl (binary)         ─┘
```

### Common Library Modules

```rust
common/
├── src/
│   ├── lib.rs              // Notification functions, Server types
│   ├── error.rs            // Structured error types (thiserror)
│   ├── constants.rs        // All magic numbers and env var names
│   ├── security.rs         // Constant-time comparison, token verification
│   ├── retry.rs            // Exponential backoff retry utilities
│   ├── executor.rs         // Remote SSH command execution
│   └── metrics.rs          // Metrics recording helpers
```

### Error Type Hierarchy

```rust
AppError (top-level)
├── NotificationError      // Gotify/ntfy failures
├── RemoteExecutionError   // SSH and remote commands
├── DockerError           // Docker API operations
├── ServerConfigError     // Server parsing
├── UpdateError          // Update operations
├── WebhookError        // Webhook auth/processing
└── HealthCheckError   // Container health
```

## Development Workflows

### Starting a New Task

1. **Read Relevant Documentation**:
   ```bash
   # For architecture understanding
   cat ARCHITECTURE.md

   # For specific service
   cat updatectl/README.md

   # For recent changes
   cat MODERNIZATION_SUMMARY.md
   ```

2. **Check Existing Patterns**:
   ```bash
   # Find similar implementations
   rg "pattern_name" --type rust

   # Look for error handling examples
   rg "NotificationError" common/src/
   ```

3. **Verify Build Environment**:
   ```bash
   cargo check --workspace
   ```

### Making Changes

1. **Use Structured Errors**:
   ```rust
   // GOOD: Specific error with context
   return Err(NotificationError::SendFailed {
       backend: "gotify".to_string(),
       message: response.status().to_string(),
   });

   // BAD: Generic error
   return Err(anyhow!("failed to send"));
   ```

2. **Add Instrumentation**:
   ```rust
   #[instrument(skip(client, body), fields(service = %service_name))]
   async fn my_function(client: &Client, body: &str, service_name: &str) {
       // Function implementation
   }
   ```

3. **Use Constants**:
   ```rust
   use common::constants::*;

   // GOOD
   timeout(Duration::from_secs(SSH_COMMAND_TIMEOUT_SECS), operation).await

   // BAD
   timeout(Duration::from_secs(300), operation).await
   ```

4. **Add Retry Logic for Network Operations**:
   ```rust
   use common::retry::retry_async_when;
   use common::retry::is_retryable_http_error;

   let result = retry_async_when(
       || async { fetch_data().await },
       is_retryable_http_error
   ).await?;
   ```

### Testing Changes

```bash
# Check compilation
cargo check --workspace

# Run tests
cargo test --workspace

# Check specific service
cargo check --package updatectl

# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace -- -D warnings
```

## Code Standards

### Error Handling

**Always use structured errors from `common::error`:**

```rust
use common::{NotificationError, RemoteExecutionError, Result};

async fn send_notification(
    client: &Client,
    title: &str,
    body: &str
) -> Result<(), NotificationError> {
    // Implementation
}
```

### Logging and Tracing

**Use `tracing` for structured logging:**

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(sensitive_data))]
async fn process_request(id: &str, sensitive_data: &str) -> Result<()> {
    info!("Processing request");

    match do_work().await {
        Ok(result) => {
            info!(result = %result, "Work completed");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Work failed");
            Err(e)
        }
    }
}
```

### Constants

**All magic numbers must be in `common/src/constants.rs`:**

```rust
// Add to constants.rs
pub const NEW_TIMEOUT_SECS: u64 = 120;

// Use in code
use common::constants::NEW_TIMEOUT_SECS;
let timeout = Duration::from_secs(NEW_TIMEOUT_SECS);
```

### Security

**Always use constant-time comparison for tokens:**

```rust
use common::security::verify_webhook_token;

if !verify_webhook_token(&provided, &expected, Some(&request_id)) {
    return Err(WebhookError::Unauthorized);
}
```

## Common Tasks

### Adding a New Service

1. **Create the binary crate**:
   ```bash
   cargo new --bin myservice
   ```

2. **Add to workspace** in root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       "common",
       "myservice",
       # ... existing members
   ]
   ```

3. **Add common dependency** in `myservice/Cargo.toml`:
   ```toml
   [dependencies]
   common = { path = "../common" }
   ```

4. **Follow the pattern**:
   - Use `common::dotenv_init()` for environment variables
   - Use `common::http_client()` for HTTP client
   - Use service-specific notification functions
   - Add structured error types to `common/src/error.rs`

### Adding a New Error Type

1. **Define in `common/src/error.rs`**:
   ```rust
   #[derive(Error, Debug)]
   pub enum MyNewError {
       #[error("specific error: {details}")]
       Specific { details: String },

       #[error(transparent)]
       Other(#[from] anyhow::Error),
   }
   ```

2. **Add to `AppError`**:
   ```rust
   #[derive(Error, Debug)]
   pub enum AppError {
       // ... existing variants

       #[error(transparent)]
       MyNew(#[from] MyNewError),
   }
   ```

3. **Export from `lib.rs`**:
   ```rust
   pub use error::{
       // ... existing exports
       MyNewError,
   };
   ```

### Adding Retry to an Operation

```rust
use common::retry::retry_async;

// Simple retry with defaults
let result = retry_async(|| async {
    perform_operation().await
}).await?;

// Custom retry condition
use common::retry::retry_async_when;

let result = retry_async_when(
    || async { perform_operation().await },
    |e| should_retry(e)
}).await?;
```

### Adding a Constant

1. **Add to appropriate section in `common/src/constants.rs`**:
   ```rust
   // In constants.rs
   pub const MY_NEW_TIMEOUT_SECS: u64 = 300;
   ```

2. **Use in code**:
   ```rust
   use common::constants::MY_NEW_TIMEOUT_SECS;
   ```

### Updating Docker Compose

**When adding services that need SSH access:**

```yaml
myservice_runner:
  image: ghcr.io/jsprague84/myservice:${MYSERVICE_TAG:-latest}
  container_name: myservice_runner
  env_file:
    - .env
  volumes:
    # SECURITY: Mount only the specific SSH key
    - ${UPDATE_SSH_KEY}:/ssh/id_key:ro
    # For Docker operations
    - /var/run/docker.sock:/var/run/docker.sock:ro
  entrypoint: ["/bin/sh", "-c", "sleep infinity"]
  restart: unless-stopped
```

## Testing Strategy

### Unit Tests

Place in same file as code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        assert_eq!(my_function(input), expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = my_async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/integration_test.rs
use common::*;

#[tokio::test]
async fn test_full_workflow() {
    // Test complete workflow
}
```

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific package
cargo test --package common

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name
```

## Deployment

### Local Development

```bash
# Build
cargo build --release

# Run specific service
./target/release/updatectl list servers

# With environment
source .env
./target/release/updatemon --docker --local
```

### Docker Build

```bash
# Build specific service
docker build -t weatherust:local .
docker build -f Dockerfile.updatectl -t updatectl:local .

# Test locally
docker run --rm -it weatherust:local --help
```

### Docker Compose

```bash
# Start all services
docker compose up -d

# View logs
docker compose logs -f

# Restart specific service
docker compose restart updatectl_webhook

# Execute command in runner
docker compose exec updatectl_runner /app/updatectl list servers
```

## File Organization

### Source Files

```
weatherust/
├── common/               # Shared library
│   └── src/
│       ├── lib.rs       # Main exports, notification functions
│       ├── error.rs     # All error types
│       ├── constants.rs # All constants
│       ├── security.rs  # Security utilities
│       ├── retry.rs     # Retry logic
│       ├── executor.rs  # Remote execution
│       └── metrics.rs   # Metrics helpers
├── updatectl/           # Update controller
├── updatemon/           # Update monitor
├── healthmon/           # Health monitor
├── speedynotify/        # Speed test
└── src/                 # Weather (main binary)
```

### Documentation Files

```
weatherust/
├── README.md                    # Main project documentation
├── CLAUDE_CODE_GUIDE.md        # This file - for AI assistants
├── ARCHITECTURE.md             # System architecture
├── CONTRIBUTING.md             # Development workflow
├── MODERNIZATION_SUMMARY.md    # Recent improvements
├── MODERNIZATION_EXAMPLES.md   # Code examples
├── docs/                       # Additional documentation
│   ├── API.md                 # API documentation
│   ├── DEPLOYMENT.md          # Deployment guide
│   └── TROUBLESHOOTING.md     # Common issues
```

## Quick Reference Commands

```bash
# Check everything compiles
cargo check --workspace

# Run all tests
cargo test --workspace

# Format code
cargo fmt --all

# Lint
cargo clippy --workspace

# Build release binaries
cargo build --release --workspace

# Generate documentation
cargo doc --open

# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Clean build artifacts
cargo clean
```

## Important Patterns

### Notification Pattern

```rust
use common::{send_gotify_myservice, send_ntfy_myservice, http_client};

async fn notify(title: &str, body: &str) {
    let client = http_client();

    // Try Gotify
    if let Err(e) = send_gotify_myservice(&client, title, body).await {
        warn!("Gotify failed: {}", e);
    }

    // Try ntfy
    if let Err(e) = send_ntfy_myservice(&client, title, body, None).await {
        warn!("ntfy failed: {}", e);
    }
}
```

### SSH Execution Pattern

```rust
use common::RemoteExecutor;

let executor = RemoteExecutor::new(ssh_key_path)?;
let output = executor.execute(&server, "ls -la").await?;
```

### Docker API Pattern

```rust
use bollard::Docker;

let docker = Docker::connect_with_unix_defaults()?;
let containers = docker.list_containers(None).await?;
```

## Context for AI Assistants

When working with this codebase:

1. **Always check `common/src/error.rs`** for appropriate error types
2. **Use constants from `common/src/constants.rs`** instead of magic numbers
3. **Add `#[instrument]`** to functions that would benefit from tracing
4. **Use retry logic** for network operations
5. **Follow security patterns** - constant-time comparisons, minimal key exposure
6. **Maintain backward compatibility** - especially with .env configuration
7. **Document breaking changes** - update relevant README files
8. **Add tests** for new functionality
9. **Update this guide** when patterns change

## Recent Modernizations

See `MODERNIZATION_SUMMARY.md` for details on recent improvements:

- ✅ Structured error handling with thiserror
- ✅ Constants module for magic numbers
- ✅ Enhanced tracing and observability
- ✅ Constant-time token comparison
- ✅ Retry logic with exponential backoff
- ✅ Improved SSH key security in Docker Compose

## Getting Help

- **Architecture Questions**: See `ARCHITECTURE.md`
- **Development Workflow**: See `CONTRIBUTING.md`
- **Code Examples**: See `MODERNIZATION_EXAMPLES.md`
- **Service-Specific**: See service README (e.g., `updatectl/README.md`)
- **API Reference**: Generated via `cargo doc --open`
