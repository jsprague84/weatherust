//! Common constants used across the weatherust project

use std::time::Duration;

// Notification priorities
pub const GOTIFY_DEFAULT_PRIORITY: u8 = 5;
pub const NTFY_DEFAULT_PRIORITY: u8 = 4;

// Timeouts
pub const SSH_CONNECTION_TIMEOUT_SECS: u64 = 30;
pub const SSH_COMMAND_TIMEOUT_SECS: u64 = 300; // 5 minutes
pub const DOCKER_OPERATION_TIMEOUT_SECS: u64 = 60;
pub const HTTP_REQUEST_TIMEOUT_SECS: u64 = 30;

// Retry configuration
pub const DEFAULT_MAX_RETRIES: usize = 3;
pub const RETRY_MIN_DELAY_MS: u64 = 100;
pub const RETRY_MAX_DELAY_MS: u64 = 30000; // 30 seconds

// Health check thresholds
pub const DEFAULT_CPU_WARN_PCT: f64 = 85.0;
pub const DEFAULT_MEM_WARN_PCT: f64 = 90.0;

// Docker stats sampling
pub const DOCKER_STATS_SAMPLE_TIMEOUT: Duration = Duration::from_secs(3);

// Token masking
pub const TOKEN_MASK_PREFIX_LEN: usize = 3;
pub const TOKEN_MASK_SUFFIX_LEN: usize = 3;
pub const TOKEN_MIN_LENGTH_FOR_MASKING: usize = 6;

// Environment variable names
pub mod env {
    // Gotify
    pub const GOTIFY_URL: &str = "GOTIFY_URL";
    pub const GOTIFY_KEY_FILE: &str = "GOTIFY_KEY_FILE";
    pub const GOTIFY_DEBUG: &str = "GOTIFY_DEBUG";

    // ntfy.sh
    pub const NTFY_URL: &str = "NTFY_URL";
    pub const NTFY_AUTH: &str = "NTFY_AUTH";
    pub const NTFY_DEBUG: &str = "NTFY_DEBUG";

    // Server configuration
    pub const UPDATE_SERVERS: &str = "UPDATE_SERVERS";
    pub const UPDATE_SSH_KEY: &str = "UPDATE_SSH_KEY";
    pub const UPDATE_LOCAL_NAME: &str = "UPDATE_LOCAL_NAME";
    pub const UPDATE_LOCAL_DISPLAY: &str = "UPDATE_LOCAL_DISPLAY";

    // Webhook
    pub const UPDATECTL_WEBHOOK_SECRET: &str = "UPDATECTL_WEBHOOK_SECRET";
    pub const UPDATECTL_WEBHOOK_URL: &str = "UPDATECTL_WEBHOOK_URL";

    // Health monitoring
    pub const HEALTH_NOTIFY_ALWAYS: &str = "HEALTH_NOTIFY_ALWAYS";
    pub const CPU_WARN_PCT: &str = "CPU_WARN_PCT";
    pub const MEM_WARN_PCT: &str = "MEM_WARN_PCT";
    pub const HEALTHMON_IGNORE: &str = "HEALTHMON_IGNORE";
}
