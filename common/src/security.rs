//! Security utilities for safe operations

use subtle::ConstantTimeEq;
use tracing::warn;

/// Perform constant-time comparison of two strings to prevent timing attacks
///
/// This function uses constant-time comparison to prevent attackers from
/// using timing information to deduce the secret value.
///
/// # Examples
///
/// ```
/// use common::security::constant_time_compare;
///
/// let secret = "my_secret_token";
/// let provided = "my_secret_token";
/// assert!(constant_time_compare(provided, secret));
///
/// let wrong = "wrong_token";
/// assert!(!constant_time_compare(wrong, secret));
/// ```
pub fn constant_time_compare(a: &str, b: &str) -> bool {
    // First check lengths in constant time
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    if a_bytes.len() != b_bytes.len() {
        // Lengths don't match - still do a comparison to avoid timing leak
        // Compare against a dummy value of the same length as 'a'
        let dummy = vec![0u8; a_bytes.len()];
        let _ = a_bytes.ct_eq(&dummy);
        return false;
    }

    // Perform constant-time comparison of the actual values
    a_bytes.ct_eq(b_bytes).into()
}

/// Verify a webhook token with constant-time comparison and logging
///
/// This function wraps `constant_time_compare` and adds logging for security events.
pub fn verify_webhook_token(provided: &str, expected: &str, request_id: Option<&str>) -> bool {
    let valid = constant_time_compare(provided, expected);

    if !valid {
        warn!(
            request_id = request_id.unwrap_or("unknown"),
            "Webhook authentication failed: invalid token"
        );
    }

    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare_equal() {
        assert!(constant_time_compare("secret123", "secret123"));
    }

    #[test]
    fn test_constant_time_compare_not_equal() {
        assert!(!constant_time_compare("secret123", "secret456"));
    }

    #[test]
    fn test_constant_time_compare_different_lengths() {
        assert!(!constant_time_compare("short", "this_is_longer"));
    }

    #[test]
    fn test_constant_time_compare_empty() {
        assert!(constant_time_compare("", ""));
    }

    #[test]
    fn test_constant_time_compare_one_empty() {
        assert!(!constant_time_compare("", "nonempty"));
        assert!(!constant_time_compare("nonempty", ""));
    }

    #[test]
    fn test_verify_webhook_token() {
        assert!(verify_webhook_token("token123", "token123", Some("req-1")));
        assert!(!verify_webhook_token("wrong", "token123", Some("req-2")));
    }
}
