use anyhow::{anyhow, Result};
use dotenvy::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

pub mod executor;
pub use executor::RemoteExecutor;

pub fn dotenv_init() {
    let _ = dotenv();
}

pub fn http_client() -> Client {
    Client::new()
}

// Generic send_gotify (deprecated - prefer service-specific functions)
// Checks GOTIFY_KEY → GOTIFY_KEY_FILE for backward compatibility
pub async fn send_gotify(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "GOTIFY_KEY").await
}

// Service-specific functions - each checks only its own key + GOTIFY_KEY_FILE fallback
pub async fn send_gotify_weatherust(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "WEATHERUST_GOTIFY_KEY").await
}

pub async fn send_gotify_updatemon(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "UPDATEMON_GOTIFY_KEY").await
}

pub async fn send_gotify_dockermon(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "DOCKERMON_GOTIFY_KEY").await
}

pub async fn send_gotify_healthmon(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "HEALTHMON_GOTIFY_KEY").await
}

pub async fn send_gotify_speedynotify(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "SPEEDY_GOTIFY_KEY").await
}

pub async fn send_gotify_updatectl(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "UPDATECTL_GOTIFY_KEY").await
}

// Internal helper: checks a specific key, then GOTIFY_KEY_FILE fallback
async fn send_gotify_with_key(
    client: &Client,
    title: &str,
    body: &str,
    key_var: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let gotify_url =
        env::var("GOTIFY_URL").unwrap_or_else(|_| "http://localhost:8080/message".to_string());

    // Resolve key with precedence:
    // 1) Specific key (e.g., WEATHERUST_GOTIFY_KEY)
    // 2) GOTIFY_KEY_FILE (file-based fallback)
    let mut key_source = "";
    let gotify_key = if let Ok(v) = env::var(key_var) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            key_source = key_var;
            v
        } else {
            String::new()
        }
    } else if let Ok(path) = env::var("GOTIFY_KEY_FILE") {
        match std::fs::read_to_string(&path) {
            Ok(s) => {
                key_source = "GOTIFY_KEY_FILE";
                s.trim().to_string()
            }
            Err(e) => {
                eprintln!("GOTIFY_KEY_FILE read error from {}: {}", path, e);
                return Ok(());
            }
        }
    } else {
        String::new()
    };

    if gotify_key.is_empty() {
        eprintln!("{} not set (also checked GOTIFY_KEY_FILE); skipping Gotify notification.", key_var);
        return Ok(());
    }

    // Optional debug output (mask token)
    let debug = env::var("GOTIFY_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if debug {
        let masked: String = if gotify_key.len() > 6 {
            format!(
                "{}***{}",
                &gotify_key[..3],
                &gotify_key[gotify_key.len() - 3..]
            )
        } else {
            "***".to_string()
        };
        eprintln!(
            "[gotify] url={} key_source={} key={} bytes_title={} bytes_body={}",
            gotify_url,
            key_source,
            masked,
            title.len(),
            body.len()
        );
    }

    client
        .post(&gotify_url)
        .header("X-Gotify-Key", gotify_key)
        .json(&serde_json::json!({
            "title": title,
            "message": body,
            "priority": 5
        }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

// ============================================================================
// ntfy.sh Support
// ============================================================================

/// ntfy.sh action button configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtfyAction {
    pub action: String,      // "view", "http", "broadcast"
    pub label: String,       // Button text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>, // URL for http/view actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>, // HTTP method (GET, POST, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<serde_json::Value>, // Custom headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>, // Request body for POST
}

impl NtfyAction {
    /// Create a simple view URL action button
    pub fn view(label: &str, url: &str) -> Self {
        NtfyAction {
            action: "view".to_string(),
            label: label.to_string(),
            url: Some(url.to_string()),
            method: None,
            headers: None,
            body: None,
        }
    }

    /// Create an HTTP POST action button
    pub fn http_post(label: &str, url: &str) -> Self {
        NtfyAction {
            action: "http".to_string(),
            label: label.to_string(),
            url: Some(url.to_string()),
            method: Some("POST".to_string()),
            headers: None,
            body: None,
        }
    }

    /// Add custom headers to the action
    pub fn with_headers(mut self, headers: serde_json::Value) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Add body to the action
    pub fn with_body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }
}

// Service-specific ntfy functions
pub async fn send_ntfy_weatherust(
    client: &Client,
    title: &str,
    body: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    send_ntfy_with_topic(client, title, body, "WEATHERUST_NTFY_TOPIC", actions).await
}

pub async fn send_ntfy_updatemon(
    client: &Client,
    title: &str,
    body: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    send_ntfy_with_topic(client, title, body, "UPDATEMON_NTFY_TOPIC", actions).await
}

pub async fn send_ntfy_dockermon(
    client: &Client,
    title: &str,
    body: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    send_ntfy_with_topic(client, title, body, "DOCKERMON_NTFY_TOPIC", actions).await
}

pub async fn send_ntfy_healthmon(
    client: &Client,
    title: &str,
    body: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    send_ntfy_with_topic(client, title, body, "HEALTHMON_NTFY_TOPIC", actions).await
}

pub async fn send_ntfy_speedynotify(
    client: &Client,
    title: &str,
    body: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    send_ntfy_with_topic(client, title, body, "SPEEDY_NTFY_TOPIC", actions).await
}

pub async fn send_ntfy_updatectl(
    client: &Client,
    title: &str,
    body: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    send_ntfy_with_topic(client, title, body, "UPDATECTL_NTFY_TOPIC", actions).await
}

// Internal helper: send ntfy notification with optional actions
async fn send_ntfy_with_topic(
    client: &Client,
    title: &str,
    body: &str,
    topic_var: &str,
    actions: Option<Vec<NtfyAction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get ntfy server URL
    let ntfy_url = env::var("NTFY_URL")
        .unwrap_or_else(|_| "https://ntfy.sh".to_string());

    // Get topic for this service
    let topic = match env::var(topic_var) {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            // ntfy not configured for this service - skip silently
            return Ok(());
        }
    };

    // Get optional auth token
    let auth_token = env::var("NTFY_AUTH").ok();

    // Optional debug output
    let debug = env::var("NTFY_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if debug {
        eprintln!(
            "[ntfy] url={}/{} topic_var={} bytes_title={} bytes_body={} actions={}",
            ntfy_url,
            topic,
            topic_var,
            title.len(),
            body.len(),
            actions.as_ref().map(|a| a.len()).unwrap_or(0)
        );
    }

    // Build the request with markdown enabled for better formatting
    let mut json_body = serde_json::json!({
        "topic": topic,
        "title": title,
        "message": body,
        "priority": 4,
        "markdown": true,
    });

    // Add actions if provided
    if let Some(acts) = actions {
        if !acts.is_empty() {
            json_body["actions"] = serde_json::to_value(acts)?;
        }
    }

    if debug {
        eprintln!("[ntfy] JSON payload: {}", serde_json::to_string_pretty(&json_body)?);
    }

    // When using JSON, post to base URL (topic is in JSON body)
    let url = ntfy_url.trim_end_matches('/').to_string();
    let mut request = client.post(&url).json(&json_body);

    // Add auth if configured
    if let Some(token) = auth_token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    request
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

// ============================================================================
// Server Configuration (shared across updatemon, updatectl, dockermon)
// ============================================================================

/// Represents a server to check/monitor
#[derive(Debug, Clone)]
pub struct Server {
    pub name: String,
    pub ssh_host: Option<String>, // None = local, Some = user@host
}

impl Server {
    /// Create a local server instance with optional custom name
    pub fn local() -> Self {
        let name = std::env::var("UPDATE_LOCAL_NAME")
            .unwrap_or_else(|_| "localhost".to_string());

        Server {
            name,
            ssh_host: None,
        }
    }

    /// Parse server from string
    /// Format: "name:user@host" or "user@host" (name derived from host)
    /// Special: "name:local" or "name:localhost" creates a localhost server with custom name
    pub fn parse(input: &str) -> Result<Self> {
        // Trim all whitespace including newlines
        let input = input.trim();
        let parts: Vec<&str> = input.split(':').collect();

        match parts.len() {
            1 => {
                let part = parts[0].trim();

                // Check if this is a localhost indicator
                if part.eq_ignore_ascii_case("local") || part.eq_ignore_ascii_case("localhost") {
                    return Ok(Server::local());
                }

                // Otherwise it's "user@host"
                let ssh_host = part.to_string();
                let name = ssh_host.split('@').last().unwrap_or("unknown").to_string();
                Ok(Server {
                    name,
                    ssh_host: Some(ssh_host),
                })
            }
            2 => {
                let name = parts[0].trim();
                let host = parts[1].trim();

                // Check if host part is localhost indicator
                if host.eq_ignore_ascii_case("local") || host.eq_ignore_ascii_case("localhost") {
                    return Ok(Server {
                        name: name.to_string(),
                        ssh_host: None,
                    });
                }

                // Normal "name:user@host"
                Ok(Server {
                    name: name.to_string(),
                    ssh_host: Some(host.to_string()),
                })
            }
            _ => Err(anyhow!("Invalid server format: {}. Expected 'name:user@host' or 'user@host'", input)),
        }
    }

    /// Is this the local system?
    pub fn is_local(&self) -> bool {
        self.ssh_host.is_none()
    }

    /// Get display host string
    pub fn display_host(&self) -> String {
        if self.is_local() {
            // Check for custom localhost display
            std::env::var("UPDATE_LOCAL_DISPLAY")
                .unwrap_or_else(|_| "local".to_string())
        } else {
            self.ssh_host.clone().unwrap()
        }
    }
}

/// Parse comma-separated server list
pub fn parse_servers(server_str: &str) -> Result<Vec<Server>> {
    server_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(Server::parse)
        .collect()
}
