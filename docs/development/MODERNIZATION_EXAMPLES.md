# Weatherust Modernization - Code Examples

This document provides practical examples of how to use the new modernization features.

## 1. Structured Error Handling

### Before
```rust
use anyhow::Result;

async fn send_notification(client: &Client, title: &str, body: &str) -> Result<()> {
    let response = client.post(&url)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to send notification"));
    }
    Ok(())
}
```

### After
```rust
use common::{NotificationError, Result};

async fn send_notification(client: &Client, title: &str, body: &str) -> Result<(), NotificationError> {
    let response = client.post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| NotificationError::HttpError(e))?;

    if !response.status().is_success() {
        return Err(NotificationError::SendFailed {
            backend: "gotify".to_string(),
            message: format!("HTTP {}", response.status()),
        });
    }
    Ok(())
}
```

**Benefits**: Error messages now include which backend failed and specific context.

## 2. Retry with Exponential Backoff

### Before
```rust
async fn fetch_docker_image_manifest(image: &str) -> Result<Manifest> {
    let response = reqwest::get(&url).await?;
    let manifest = response.json().await?;
    Ok(manifest)
}
```

### After
```rust
use common::retry::{retry_async_when, is_retryable_http_error};

async fn fetch_docker_image_manifest(image: &str) -> Result<Manifest> {
    retry_async_when(
        || async {
            let response = reqwest::get(&url).await?;
            let manifest = response.json().await?;
            Ok(manifest)
        },
        is_retryable_http_error
    ).await
}
```

**Benefits**: Automatically retries on timeouts, connection errors, and 5xx responses with exponential backoff.

## 3. Secure Webhook Token Verification

### Before (SECURITY VULNERABILITY!)
```rust
fn verify_token(provided: &str, expected: &str) -> bool {
    provided == expected  // ⚠️ Vulnerable to timing attacks!
}
```

### After
```rust
use common::security::verify_webhook_token;

fn verify_token(provided: &str, expected: &str, request_id: &str) -> bool {
    verify_webhook_token(provided, expected, Some(request_id))
}
```

**Benefits**: Constant-time comparison prevents timing attacks, includes audit logging.

## 4. Using Constants

### Before
```rust
async fn connect_ssh(host: &str) -> Result<Session> {
    let timeout = Duration::from_secs(30);  // Magic number
    let session = timeout(timeout, Session::connect(host)).await??;
    Ok(session)
}
```

### After
```rust
use common::constants::SSH_CONNECTION_TIMEOUT_SECS;
use std::time::Duration;

async fn connect_ssh(host: &str) -> Result<Session> {
    let timeout = Duration::from_secs(SSH_CONNECTION_TIMEOUT_SECS);
    let session = timeout(timeout, Session::connect(host)).await??;
    Ok(session)
}
```

**Benefits**: Single source of truth, easy to change, self-documenting.

## 5. Enhanced Tracing

### Before
```rust
async fn check_server(server: &Server) -> Result<Report> {
    println!("Checking server: {}", server.name);
    let result = do_check(server).await?;
    println!("Check complete for: {}", server.name);
    Ok(result)
}
```

### After
```rust
use tracing::{instrument, info};

#[instrument(fields(server_name = %server.name, server_host = %server.display_host()))]
async fn check_server(server: &Server) -> Result<Report> {
    info!("Starting server check");
    let result = do_check(server).await?;
    info!("Server check complete");
    Ok(result)
}
```

**Benefits**: Automatic span creation, structured logging, distributed tracing support.

## 6. Environment Variable Access

### Before
```rust
let gotify_url = env::var("GOTIFY_URL")
    .unwrap_or_else(|_| "http://localhost:8080/message".to_string());
```

### After
```rust
use common::constants::env as env_keys;
use std::env as std_env;

let gotify_url = std_env::var(env_keys::GOTIFY_URL)
    .unwrap_or_else(|_| "http://localhost:8080/message".to_string());
```

**Benefits**: Typo-proof environment variable names, centralized env var documentation.

## 7. Comprehensive Error Context

### Before
```rust
async fn update_server(server: &Server) -> Result<()> {
    check_updates(server).await?;  // Which server? What failed?
    apply_updates(server).await?;  // No context!
    Ok(())
}
```

### After
```rust
use common::{UpdateError, ServerConfigError};
use anyhow::Context;

async fn update_server(server: &Server) -> Result<()> {
    check_updates(server).await
        .context(format!("Failed to check updates on {}", server.name))?;

    apply_updates(server).await
        .map_err(|e| UpdateError::ApplyFailed {
            server: server.name.clone(),
            message: e.to_string(),
        })?;

    Ok(())
}
```

**Benefits**: Error messages include server name and operation context.

## 8. Custom Retry Configuration

### Default Retry (Most Common)
```rust
use common::retry::retry_async;

let result = retry_async(|| async {
    fetch_data().await
}).await?;
```

### Custom Retry Configuration
```rust
use common::retry::backoff_with_config;
use backon::Retryable;

let result = fetch_data
    .retry(backoff_with_config(
        50,     // min_delay_ms
        10000,  // max_delay_ms
        5       // max_retries
    ))
    .sleep(tokio::time::sleep)
    .when(|e| e.to_string().contains("temporary"))
    .notify(|err, dur| {
        warn!("Retry after {:?}: {}", dur, err);
    })
    .await?;
```

## 9. Testing Patterns

### Testing with New Error Types
```rust
#[tokio::test]
async fn test_notification_error_handling() {
    let result = send_notification(&client, "Test", "Body").await;

    match result {
        Err(NotificationError::NotConfigured { backend, key }) => {
            assert_eq!(backend, "gotify");
            assert_eq!(key, "GOTIFY_KEY");
        }
        _ => panic!("Expected NotConfigured error"),
    }
}
```

### Testing Retry Logic
```rust
#[tokio::test]
async fn test_retry_eventually_succeeds() {
    let mut attempt = 0;

    let result = retry_async(|| async {
        attempt += 1;
        if attempt < 3 {
            Err(anyhow!("temporary error"))
        } else {
            Ok("success")
        }
    }).await;

    assert!(result.is_ok());
    assert_eq!(attempt, 3);
}
```

### Testing Constant-Time Comparison
```rust
#[test]
fn test_timing_attack_resistance() {
    use std::time::Instant;
    use common::security::constant_time_compare;

    let secret = "very_secret_token_12345";

    // Both comparisons should take similar time
    let start = Instant::now();
    constant_time_compare("a", secret);
    let short_time = start.elapsed();

    let start = Instant::now();
    constant_time_compare("very_secret_token_12344", secret);
    let long_time = start.elapsed();

    // Times should be within same order of magnitude
    assert!(long_time.as_micros() < short_time.as_micros() * 10);
}
```

## 10. Webhook Implementation Example

```rust
use axum::{
    routing::post,
    Router,
    Json,
    http::{StatusCode, HeaderMap},
};
use common::{
    security::verify_webhook_token,
    WebhookError,
};

async fn webhook_handler(
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> Result<StatusCode, WebhookError> {
    // Extract authorization header
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(WebhookError::Unauthorized)?;

    // Secure token verification
    let expected = std::env::var("UPDATECTL_WEBHOOK_SECRET")
        .map_err(|_| WebhookError::ServerError("Secret not configured".to_string()))?;

    if !verify_webhook_token(token, &expected, Some(&payload.request_id)) {
        return Err(WebhookError::Unauthorized);
    }

    // Process webhook
    process_update_request(&payload).await
        .map_err(|e| WebhookError::ExecutionFailed(e.to_string()))?;

    Ok(StatusCode::OK)
}
```

## Common Patterns

### Pattern: Operation with Retry and Timeout
```rust
use tokio::time::{timeout, Duration};
use common::retry::retry_async;
use common::constants::SSH_COMMAND_TIMEOUT_SECS;

async fn execute_remote_command(server: &Server, command: &str) -> Result<String> {
    let operation = || async {
        let session = connect_ssh(server).await?;
        let output = session.exec(command).await?;
        Ok(output)
    };

    timeout(
        Duration::from_secs(SSH_COMMAND_TIMEOUT_SECS),
        retry_async(operation)
    ).await
    .map_err(|_| RemoteExecutionError::Timeout {
        host: server.display_host(),
        timeout_secs: SSH_COMMAND_TIMEOUT_SECS,
    })?
}
```

### Pattern: Graceful Degradation
```rust
async fn send_notifications(client: &Client, title: &str, body: &str) {
    // Try Gotify first
    if let Err(e) = send_gotify(client, title, body).await {
        warn!("Gotify notification failed: {}", e);
    }

    // Try ntfy as fallback
    if let Err(e) = send_ntfy(client, title, body, None).await {
        warn!("ntfy notification failed: {}", e);
    }

    // Both failed, but we continue - notifications are not critical
}
```

### Pattern: Error Aggregation
```rust
use common::{UpdateError, AppError};

async fn update_all_servers(servers: &[Server]) -> Vec<Result<(), AppError>> {
    let mut results = Vec::new();

    for server in servers {
        let result = update_server(server).await
            .map_err(|e| AppError::Update(UpdateError::ApplyFailed {
                server: server.name.clone(),
                message: e.to_string(),
            }));

        results.push(result);
    }

    results
}
```

## Performance Tips

1. **Retry Strategy**: Use `is_retryable_http_error` for HTTP operations
2. **Tracing**: Use `skip` parameter to avoid logging large payloads
3. **Constants**: Accessed at compile-time, zero runtime overhead
4. **Error Types**: Zero-cost abstractions in release mode

## Migration Checklist

- [ ] Update `Cargo.toml` to include new dependencies
- [ ] Replace magic numbers with constants from `common::constants`
- [ ] Add retry logic to network operations
- [ ] Use structured errors instead of `anyhow!` macros
- [ ] Add `#[instrument]` to key functions
- [ ] Replace token comparisons with `constant_time_compare`
- [ ] Update Docker Compose to use `UPDATE_SSH_KEY`
- [ ] Add tests for error cases
- [ ] Update documentation with new patterns

## References

- [thiserror documentation](https://docs.rs/thiserror)
- [backon documentation](https://docs.rs/backon)
- [tracing documentation](https://docs.rs/tracing)
- [subtle documentation](https://docs.rs/subtle)
