//! Retry utilities with exponential backoff
//!
//! This module provides convenience functions for retrying operations
//! with exponential backoff using the backon crate.

use backon::{ExponentialBuilder, Retryable};
use std::time::Duration;
use tracing::warn;

use crate::constants::{DEFAULT_MAX_RETRIES, RETRY_MIN_DELAY_MS, RETRY_MAX_DELAY_MS};

/// Create a default exponential backoff builder
///
/// This provides sensible defaults for most use cases:
/// - Min delay: 100ms
/// - Max delay: 30s
/// - Max retries: 3
pub fn default_backoff() -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(RETRY_MIN_DELAY_MS))
        .with_max_delay(Duration::from_millis(RETRY_MAX_DELAY_MS))
        .with_max_times(DEFAULT_MAX_RETRIES)
}

/// Create a custom exponential backoff builder
pub fn backoff_with_config(
    min_delay_ms: u64,
    max_delay_ms: u64,
    max_retries: usize,
) -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(min_delay_ms))
        .with_max_delay(Duration::from_millis(max_delay_ms))
        .with_max_times(max_retries)
}

/// Retry an async operation with default backoff
///
/// # Examples
///
/// ```no_run
/// use common::retry::retry_async;
/// use anyhow::Result;
///
/// async fn fetch_data() -> Result<String> {
///     // Some fallible operation
///     Ok("data".to_string())
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let data = retry_async(fetch_data).await?;
///     Ok(())
/// }
/// ```
pub async fn retry_async<F, Fut, T, E>(operation: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    operation
        .retry(default_backoff())
        .sleep(tokio::time::sleep)
        .notify(|err, dur: Duration| {
            warn!(
                error = %err,
                retry_after_ms = dur.as_millis(),
                "Retrying after error"
            );
        })
        .await
}

/// Retry an async operation with custom retry condition
///
/// # Examples
///
/// ```no_run
/// use common::retry::retry_async_when;
/// use anyhow::{Result, anyhow};
///
/// async fn fetch_data() -> Result<String> {
///     Err(anyhow!("temporary error"))
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let data = retry_async_when(
///         fetch_data,
///         |e| e.to_string().contains("temporary")
///     ).await?;
///     Ok(())
/// }
/// ```
pub async fn retry_async_when<F, Fut, T, E, P>(
    operation: F,
    should_retry: P,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    P: FnMut(&E) -> bool,
{
    operation
        .retry(default_backoff())
        .sleep(tokio::time::sleep)
        .when(should_retry)
        .notify(|err, dur: Duration| {
            warn!(
                error = %err,
                retry_after_ms = dur.as_millis(),
                "Retrying after error"
            );
        })
        .await
}

/// Helper to determine if an HTTP error is retryable
pub fn is_retryable_http_error(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_connect() {
        return true;
    }

    if let Some(status) = error.status() {
        // Retry on 5xx server errors and 429 (rate limit)
        status.is_server_error() || status.as_u16() == 429
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, anyhow};

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
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt, 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts() {
        let mut attempt = 0;

        let result = retry_async(|| async {
            attempt += 1;
            Err::<String, _>(anyhow!("persistent error"))
        }).await;

        assert!(result.is_err());
        // Should try initial + 3 retries = 4 total
        assert_eq!(attempt, DEFAULT_MAX_RETRIES + 1);
    }

    #[tokio::test]
    async fn test_retry_with_condition() {
        let result = retry_async_when(
            || async { Err::<String, _>(anyhow!("non-retryable error")) },
            |e| e.to_string().contains("retryable")
        ).await;

        assert!(result.is_err());
    }
}
