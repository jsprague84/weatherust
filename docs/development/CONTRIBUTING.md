# Contributing to Weatherust

Thank you for your interest in contributing to Weatherust! This document provides guidelines and workflows for development.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

## Getting Started

### Prerequisites

- **Rust**: 1.90.0 (automatically enforced via `rust-toolchain.toml`)
- **Docker**: For building and testing containers
- **Docker Compose**: For integration testing
- **Git**: For version control

### Quick Start

```bash
# Clone the repository
git clone https://github.com/jsprague84/weatherust.git
cd weatherust

# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Check formatting and lints
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

## Development Setup

### 1. Environment Configuration

```bash
# Copy example environment
cp .env.example .env

# Edit .env and fill in:
# - OWM_API_KEY (for weatherust)
# - Notification keys (GOTIFY_URL, *_GOTIFY_KEY, or *_NTFY_TOPIC)
# - UPDATE_SSH_KEY (path to your SSH key for remote operations)
# - UPDATE_SERVERS (comma-separated list of servers)
```

### 2. IDE Setup

#### VS Code (Recommended)

Install extensions:
- `rust-analyzer` - Rust language support
- `CodeLLDB` - Debugging
- `Even Better TOML` - TOML syntax
- `Error Lens` - Inline errors

Recommended settings (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true
}
```

#### Other IDEs

- **IntelliJ IDEA / CLion**: Install Rust plugin
- **Emacs**: Use `rustic-mode`
- **Vim/Neovim**: Use `rust.vim` + LSP

### 3. Git Hooks (Optional)

```bash
# Install pre-commit hooks
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --all -- --check

# Clippy check
cargo clippy --workspace -- -D warnings

# Test check
cargo test --workspace --quiet

echo "✓ Pre-commit checks passed"
EOF

chmod +x .git/hooks/pre-commit
```

## Development Workflow

### Branch Strategy

- `main` - Production-ready code, protected
- `develop` - Integration branch (if using GitFlow)
- `feature/*` - Feature branches
- `fix/*` - Bug fix branches
- `docs/*` - Documentation updates

### Creating a Feature Branch

```bash
# Update main
git checkout main
git pull origin main

# Create feature branch
git checkout -b feature/my-feature-name

# Make changes
# ...

# Commit with conventional commits
git commit -m "feat: add new monitoring capability"

# Push and create PR
git push -u origin feature/my-feature-name
```

### Conventional Commits

Use conventional commit messages:

```
feat: add new feature
fix: bug fix
docs: documentation changes
style: formatting, missing semicolons, etc.
refactor: code refactoring
perf: performance improvements
test: adding tests
chore: updating build tasks, package manager configs, etc.
```

Examples:
```bash
git commit -m "feat(updatectl): add cleanup profile support"
git commit -m "fix(healthmon): handle containers without health checks"
git commit -m "docs: update architecture diagram"
git commit -m "refactor(common): extract retry logic to module"
```

## Code Standards

### Rust Style Guide

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

1. **Naming Conventions**:
   - Types: `PascalCase`
   - Functions/variables: `snake_case`
   - Constants: `SCREAMING_SNAKE_CASE`
   - Modules: `snake_case`

2. **Code Formatting**:
   ```bash
   # Format all code
   cargo fmt --all

   # Check format without modifying
   cargo fmt --all -- --check
   ```

3. **Linting**:
   ```bash
   # Run Clippy
   cargo clippy --workspace -- -D warnings

   # Fix automatically where possible
   cargo clippy --workspace --fix
   ```

### Project-Specific Standards

#### 1. Error Handling

**Always use structured errors from `common::error`:**

```rust
// ✅ GOOD
use common::{NotificationError, Result};

async fn send_notification() -> Result<(), NotificationError> {
    Err(NotificationError::SendFailed {
        backend: "gotify".to_string(),
        message: "connection refused".to_string(),
    })
}

// ❌ BAD
async fn send_notification() -> Result<()> {
    Err(anyhow!("failed to send notification"))
}
```

#### 2. Constants

**Extract all magic numbers:**

```rust
// ✅ GOOD
use common::constants::SSH_CONNECTION_TIMEOUT_SECS;
let timeout = Duration::from_secs(SSH_CONNECTION_TIMEOUT_SECS);

// ❌ BAD
let timeout = Duration::from_secs(30);
```

#### 3. Logging

**Use `tracing` with instrumentation:**

```rust
// ✅ GOOD
use tracing::{info, instrument};

#[instrument(skip(password))]
async fn login(username: &str, password: &str) -> Result<()> {
    info!("Starting login");
    // ...
}

// ❌ BAD
async fn login(username: &str, password: &str) -> Result<()> {
    println!("Logging in: {}", username);
    // ...
}
```

#### 4. Security

**Use constant-time comparisons for secrets:**

```rust
// ✅ GOOD
use common::security::verify_webhook_token;

if !verify_webhook_token(&provided, &expected, Some(&req_id)) {
    return Err(WebhookError::Unauthorized);
}

// ❌ BAD
if provided != expected {
    return Err(WebhookError::Unauthorized);
}
```

#### 5. Async/Await

**Use async where appropriate:**

```rust
// ✅ GOOD - I/O operations
async fn fetch_data() -> Result<String> {
    reqwest::get("https://example.com").await?.text().await
}

// ❌ BAD - CPU-bound work shouldn't be async
async fn calculate_hash(data: &[u8]) -> String {
    // CPU-intensive work
}
```

### Documentation

#### Code Documentation

```rust
/// Sends a notification via Gotify with retry logic.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `title` - Notification title
/// * `body` - Notification body
///
/// # Errors
///
/// Returns `NotificationError` if:
/// - Gotify server is unreachable
/// - Authentication fails
/// - Request times out
///
/// # Examples
///
/// ```no_run
/// use common::{send_gotify_weatherust, http_client};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = http_client();
///     send_gotify_weatherust(&client, "Title", "Body").await?;
///     Ok(())
/// }
/// ```
#[instrument(skip(client, body))]
pub async fn send_gotify_weatherust(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), NotificationError> {
    // Implementation
}
```

#### File Documentation

Every file should have a module-level docstring:

```rust
//! Notification utilities for Gotify and ntfy.sh.
//!
//! This module provides functions for sending notifications via multiple
//! backends with automatic retry and failover capabilities.

use reqwest::Client;
// ...
```

## Testing

### Unit Tests

Place tests in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_parsing() {
        let server = Server::parse("name:user@host").unwrap();
        assert_eq!(server.name, "name");
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
// tests/notification_integration.rs
use common::*;

#[tokio::test]
async fn test_notification_flow() {
    // Set up test environment
    // Execute notification
    // Verify results
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

# Test coverage (requires tarpaulin)
cargo tarpaulin --workspace --out Html
```

### Test Guidelines

1. **Test Naming**: Use descriptive names (`test_error_when_server_unavailable`)
2. **Arrange-Act-Assert**: Structure tests clearly
3. **Mock External Dependencies**: Use test doubles for HTTP, SSH, etc.
4. **Test Error Cases**: Don't just test happy paths
5. **Avoid Flaky Tests**: No time-based assertions, no network dependencies

## Documentation

### When to Update Documentation

Update documentation when you:
- Add new features
- Change behavior
- Fix bugs that weren't obvious
- Add new error types
- Change API signatures

### Documentation Files

- **README.md** - High-level project overview
- **CLAUDE_CODE_GUIDE.md** - For AI assistants
- **ARCHITECTURE.md** - System architecture
- **CONTRIBUTING.md** - This file
- **Service READMEs** - Service-specific documentation
- **MODERNIZATION_*.md** - Recent changes and examples

### Generating API Docs

```bash
# Generate and open documentation
cargo doc --open --no-deps

# Include private items
cargo doc --document-private-items
```

## Pull Request Process

### Before Creating a PR

1. **Ensure tests pass**:
   ```bash
   cargo test --workspace
   ```

2. **Check formatting**:
   ```bash
   cargo fmt --all -- --check
   ```

3. **Run Clippy**:
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

4. **Update documentation** if needed

5. **Add tests** for new functionality

### PR Template

When creating a PR, include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
How was this tested?

## Checklist
- [ ] Code follows project style
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No breaking changes (or documented)
- [ ] Commits follow conventional commits
```

### Review Process

1. PR is created
2. Automated checks run (GitHub Actions)
3. Code review by maintainer(s)
4. Address feedback
5. Approval and merge

### Merge Strategy

- **Squash and merge** - For feature branches
- **Rebase and merge** - For clean history when needed
- **Merge commit** - For important feature merges

## Release Process

### Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes

### Creating a Release

1. **Update version** in all `Cargo.toml` files:
   ```toml
   [package]
   version = "0.2.0"
   ```

2. **Update CHANGELOG.md**:
   ```markdown
   ## [0.2.0] - 2025-01-15

   ### Added
   - New cleanup profiles in updatectl

   ### Fixed
   - SSH key mounting security issue

   ### Changed
   - Improved error messages
   ```

3. **Commit and tag**:
   ```bash
   git add .
   git commit -m "chore: release v0.2.0"
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin main --tags
   ```

4. **GitHub Actions** will automatically:
   - Build multi-arch Docker images
   - Publish to GHCR
   - Create GitHub release

### Docker Image Tags

Images are published with multiple tags:
- `latest` - Latest main branch build
- `v0.2.0` - Specific version
- `sha-abc123` - Specific commit
- `pr-42` - Pull request builds
- `feature-branch-name` - Branch builds

## Development Tips

### Useful Commands

```bash
# Watch for changes and rebuild
cargo watch -x check

# Build with all features
cargo build --workspace --all-features

# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Audit dependencies for security issues
cargo audit

# Clean build artifacts
cargo clean
```

### Debugging

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run specific binary
cargo run --bin updatectl -- list servers

# Attach debugger (VS Code)
# Use "Debug" configuration in launch.json
```

### Docker Development

```bash
# Build local image
docker build -t weatherust:dev .

# Test local image
docker run --rm -it weatherust:dev --help

# Test compose stack locally
docker compose -f docker-compose.yml up -d

# View logs
docker compose logs -f
```

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: Open a GitHub Issue
- **Documentation**: Check existing docs first
- **Chat**: (If applicable)

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the code, not the person
- Help newcomers
- Credit others' work

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (see LICENSE file).
