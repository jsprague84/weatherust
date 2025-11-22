# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Weatherust** is a Rust-based infrastructure monitoring and automation platform consisting of multiple services that run in Docker containers, scheduled by Ofelia.

### Quick Links

- 📚 **[Claude Code Guide](./docs/development/CLAUDE_CODE_GUIDE.md)** - Comprehensive development guide (READ THIS FIRST)
- 🏗️ **[Architecture](./docs/architecture/ARCHITECTURE.md)** - System architecture and design
- 🤝 **[Contributing](./docs/development/CONTRIBUTING.md)** - Development workflow and standards
- 📖 **[README.md](./README.md)** - User documentation and quick start
- 📑 **[Documentation Index](./docs/README.md)** - Complete documentation navigation

## Workspace Structure

This is a Rust workspace with 6 crates:

```
weatherust/
├── common/          # Shared library (errors, retry, security, constants)
├── weatherust/      # Weather monitoring (OpenWeatherMap)
├── speedynotify/    # Speed test monitoring (Ookla)
├── healthmon/       # Docker health monitoring
├── updatemon/       # Update monitoring (read-only)
└── updatectl/       # Update controller (write operations)
```

## Key Architecture Points

1. **Shared Common Library**: All services depend on `common` crate
2. **Docker-First**: Designed to run in containers with Ofelia scheduling
3. **Dual Notifications**: Gotify and ntfy.sh support throughout
4. **Structured Errors**: Domain-specific error types (no generic anyhow)
5. **Security-Conscious**: Constant-time comparisons, minimal key exposure
6. **Observable**: Structured logging with `tracing` crate

## Build and Development Commands

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Check compilation
cargo check --workspace

# Format code
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings

# Generate docs
cargo doc --open
```

## Essential Code Patterns

### 1. Error Handling (REQUIRED)

**Always use structured errors from `common::error`:**

```rust
use common::{NotificationError, Result};

async fn send_notification() -> Result<(), NotificationError> {
    // Use specific error variants with context
    Err(NotificationError::SendFailed {
        backend: "gotify".to_string(),
        message: "connection refused".to_string(),
    })
}
```

### 2. Constants (REQUIRED)

**Use constants from `common::constants` - no magic numbers:**

```rust
use common::constants::SSH_CONNECTION_TIMEOUT_SECS;
use std::time::Duration;

let timeout = Duration::from_secs(SSH_CONNECTION_TIMEOUT_SECS);
```

### 3. Tracing (RECOMMENDED)

**Add instrumentation to functions:**

```rust
use tracing::{info, instrument};

#[instrument(skip(password), fields(user = %username))]
async fn login(username: &str, password: &str) -> Result<()> {
    info!("Starting login");
    // Implementation
}
```

### 4. Retry Logic (FOR NETWORK OPS)

**Use retry utilities for network operations:**

```rust
use common::retry::{retry_async_when, is_retryable_http_error};

let result = retry_async_when(
    || async { fetch_data().await },
    is_retryable_http_error
).await?;
```

### 5. Security (CRITICAL)

**Use constant-time comparison for secrets:**

```rust
use common::security::verify_webhook_token;

if !verify_webhook_token(&provided, &expected, Some(&request_id)) {
    return Err(WebhookError::Unauthorized);
}
```

## File Organization

### Adding New Code

- **Errors**: Add to `common/src/error.rs`
- **Constants**: Add to `common/src/constants.rs`
- **Shared Utilities**: Add to `common/src/`
- **Service-Specific**: Add to service directory

### Module Structure

```rust
// In service/src/main.rs
mod types;          // Service-specific types
mod handlers;       // Request handlers
mod operations;     // Business logic

use common::{        // Import from common
    error::*,
    constants::*,
    retry::*,
};
```

## Testing Requirements

1. **Unit Tests**: Add in same file with code
2. **Integration Tests**: Add in `tests/` directory
3. **Run Before Committing**: `cargo test --workspace`
4. **Coverage**: Aim for >80% on new code

Example:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_function() {
        let result = my_function().await;
        assert!(result.is_ok());
    }
}
```

## Documentation Standards

### Code Documentation

Every public function should have:

```rust
/// Brief one-line description.
///
/// Longer description if needed explaining the purpose,
/// behavior, and any important details.
///
/// # Arguments
///
/// * `param1` - Description of param1
/// * `param2` - Description of param2
///
/// # Errors
///
/// Returns `MyError` if X happens.
///
/// # Examples
///
/// ```no_run
/// let result = my_function(arg1, arg2).await?;
/// ```
#[instrument(skip(sensitive_param))]
pub async fn my_function(param1: &str, sensitive_param: &str) -> Result<()> {
    // Implementation
}
```

### File Documentation

Every file should start with:

```rust
//! Brief module description.
//!
//! Longer description of what this module does and why it exists.
```

## Common Tasks Reference

### Adding a New Service

1. Create binary crate: `cargo new --bin servicename`
2. Add to workspace in root `Cargo.toml`
3. Add `common` dependency
4. Follow existing service patterns
5. Add Dockerfile
6. Update docker-compose.yml
7. Add service README

### Adding an Error Type

1. Define in `common/src/error.rs`
2. Add to `AppError` enum
3. Export from `common/src/lib.rs`
4. Use in service code

### Adding a Constant

1. Add to `common/src/constants.rs` in appropriate section
2. Use throughout codebase
3. Never hard-code the value again

## Security Considerations

1. **SSH Keys**: Only mount specific key (not entire `.ssh/`)
2. **Tokens**: Always use constant-time comparison
3. **Secrets**: Never log or print sensitive data
4. **Permissions**: Minimal required (read-only where possible)
5. **Audit**: Log authentication failures

## Performance Guidelines

1. **Use async/await** for I/O operations
2. **Avoid blocking** in async context
3. **Parallel execution** for independent operations
4. **Connection pooling** (TODO for SSH)
5. **Retry with backoff** for transient failures

## AI Assistant Guidelines

When working with this codebase:

1. **Read Context First**:
   - Start with `CLAUDE_CODE_GUIDE.md` for comprehensive context
   - Check `ARCHITECTURE.md` for system design
   - Review service README for service-specific details

2. **Follow Patterns**:
   - Use existing error types (don't create new ones unnecessarily)
   - Follow existing code organization
   - Match the style of surrounding code

3. **Maintain Quality**:
   - Run `cargo check` before suggesting code
   - Add tests for new functionality
   - Update documentation for changes
   - Follow security best practices

4. **Be Specific**:
   - Provide file paths in responses (`common/src/error.rs:45`)
   - Show before/after for changes
   - Explain rationale for design decisions

5. **Safety First**:
   - Prefer compile-time safety
   - Use Result types, not panics
   - Validate inputs
   - Handle all error cases

## Recent Major Changes

See [Modernization Summary](./docs/development/MODERNIZATION_SUMMARY.md) for recent improvements:

- ✅ **Structured Error Handling**: thiserror-based errors throughout
- ✅ **Constants Module**: All magic numbers extracted
- ✅ **Enhanced Tracing**: Instrumentation added to key functions
- ✅ **Security Improvements**: Constant-time comparisons, minimal key exposure
- ✅ **Retry Logic**: Exponential backoff for network operations
- ✅ **SSH Key Security**: Docker Compose updated for minimal exposure

## Environment Variables

All environment variable names are defined in `common/src/constants.rs`:

```rust
use common::constants::env as env_keys;

// Usage
let url = std::env::var(env_keys::GOTIFY_URL)?;
```

Service-specific keys:
- `WEATHERUST_GOTIFY_KEY` / `WEATHERUST_NTFY_TOPIC`
- `UPDATEMON_GOTIFY_KEY` / `UPDATEMON_NTFY_TOPIC`
- `UPDATECTL_GOTIFY_KEY` / `UPDATECTL_NTFY_TOPIC`
- `HEALTHMON_GOTIFY_KEY` / `HEALTHMON_NTFY_TOPIC`
- `SPEEDY_GOTIFY_KEY` / `SPEEDY_NTFY_TOPIC`

## Deployment

### Local Development

```bash
cargo run --bin updatectl -- list servers
```

### Docker

```bash
# Build
docker build -t weatherust:local .

# Run
docker run --rm -it weatherust:local --help
```

### Docker Compose

```bash
# Start all services
docker compose up -d

# View logs
docker compose logs -f

# Execute command
docker compose exec updatectl_runner /app/updatectl list servers
```

## Getting More Help

- **Architecture Questions**: See [Architecture](./docs/architecture/ARCHITECTURE.md)
- **Development Workflow**: See [Contributing](./docs/development/CONTRIBUTING.md)
- **Code Examples**: See [Modernization Examples](./docs/development/MODERNIZATION_EXAMPLES.md)
- **Service Specifics**: See service README (e.g., `updatectl/README.md`)
- **API Docs**: Run `cargo doc --open`
- **Complete Index**: See [Documentation Index](./docs/README.md)

## Important: Read These First

Before starting any development:

1. **[Claude Code Guide](./docs/development/CLAUDE_CODE_GUIDE.md)** - Complete development guide
2. **[Architecture](./docs/architecture/ARCHITECTURE.md)** - System architecture
3. **[Modernization Summary](./docs/development/MODERNIZATION_SUMMARY.md)** - Recent changes

These documents provide essential context for making appropriate changes to the codebase.
