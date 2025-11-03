use dotenvy::dotenv;
use reqwest::Client;
use std::env;

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

pub async fn send_gotify_speedynotify(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_gotify_with_key(client, title, body, "SPEEDY_GOTIFY_KEY").await
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
