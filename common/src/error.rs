//! Structured error types for the weatherust project
//!
//! This module provides domain-specific error types using thiserror,
//! making error handling more explicit and maintainable.

use thiserror::Error;

/// Errors related to notification delivery (Gotify, ntfy.sh)
#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("notification backend '{backend}' not configured (missing {key})")]
    NotConfigured { backend: String, key: String },

    #[error("failed to send notification to {backend}: {message}")]
    SendFailed { backend: String, message: String },

    #[error("invalid notification configuration: {0}")]
    InvalidConfig(String),

    #[error("failed to read key file at {path}: {source}")]
    KeyFileReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Errors related to remote server operations (SSH, Docker)
#[derive(Error, Debug)]
pub enum RemoteExecutionError {
    #[error("SSH connection to {host} failed: {message}")]
    SshConnectionFailed { host: String, message: String },

    #[error("SSH command execution failed on {host}: {message}")]
    SshCommandFailed { host: String, message: String },

    #[error("timeout executing command on {host} (exceeded {timeout_secs}s)")]
    Timeout { host: String, timeout_secs: u64 },

    #[error("authentication failed for {host}: {message}")]
    AuthenticationFailed { host: String, message: String },

    #[error("SSH key not found at {path}")]
    SshKeyNotFound { path: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Errors related to Docker operations
#[derive(Error, Debug)]
pub enum DockerError {
    #[cfg(feature = "docker")]
    #[error("Docker API error: {0}")]
    BollardError(#[from] bollard::errors::Error),

    #[error("container '{container}' not found")]
    ContainerNotFound { container: String },

    #[error("image '{image}' not found")]
    ImageNotFound { image: String },

    #[error("failed to connect to Docker daemon: {message}")]
    ConnectionFailed { message: String },

    #[error("operation timed out after {timeout_secs}s")]
    OperationTimeout { timeout_secs: u64 },

    #[error("invalid Docker response: {0}")]
    InvalidResponse(String),

    #[error("Docker error: {0}")]
    Other(String),
}

/// Errors related to server configuration and parsing
#[derive(Error, Debug)]
pub enum ServerConfigError {
    #[error("invalid server format: '{input}'. Expected 'name:user@host' or 'user@host'")]
    InvalidFormat { input: String },

    #[error("server '{name}' not found in configuration")]
    ServerNotFound { name: String },

    #[error("empty server list")]
    EmptyServerList,

    #[error("duplicate server name: '{name}'")]
    DuplicateServer { name: String },
}

/// Errors related to update operations
#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("failed to check for updates on {server}: {message}")]
    CheckFailed { server: String, message: String },

    #[error("failed to apply updates on {server}: {message}")]
    ApplyFailed { server: String, message: String },

    #[error("package manager '{0}' not supported")]
    UnsupportedPackageManager(String),

    #[error("no updates available")]
    NoUpdatesAvailable,

    #[error(transparent)]
    RemoteExecution(#[from] RemoteExecutionError),

    #[error(transparent)]
    Docker(#[from] DockerError),
}

/// Errors related to webhook operations
#[derive(Error, Debug)]
pub enum WebhookError {
    #[error("unauthorized: invalid token")]
    Unauthorized,

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("server error: {0}")]
    ServerError(String),
}

/// Errors related to health monitoring
#[derive(Error, Debug)]
pub enum HealthCheckError {
    #[error("failed to check container health: {0}")]
    ContainerCheckFailed(String),

    #[error("failed to get container stats: {0}")]
    StatsFailed(String),

    #[error(transparent)]
    Docker(#[from] DockerError),
}

/// General application errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Notification(#[from] NotificationError),

    #[error(transparent)]
    RemoteExecution(#[from] RemoteExecutionError),

    #[error(transparent)]
    Docker(#[from] DockerError),

    #[error(transparent)]
    ServerConfig(#[from] ServerConfigError),

    #[error(transparent)]
    Update(#[from] UpdateError),

    #[error(transparent)]
    Webhook(#[from] WebhookError),

    #[error(transparent)]
    HealthCheck(#[from] HealthCheckError),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("unexpected error: {0}")]
    Other(#[from] anyhow::Error),
}

// Type alias for common Result type
pub type Result<T, E = AppError> = std::result::Result<T, E>;
