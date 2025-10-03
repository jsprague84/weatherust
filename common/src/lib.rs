use dotenvy::dotenv;
use reqwest::Client;
use std::env;

pub fn dotenv_init() {
    let _ = dotenv();
}

pub fn http_client() -> Client {
    Client::new()
}

// Send a Gotify message if configured. If GOTIFY is not configured, log and return Ok.
pub async fn send_gotify(
    client: &Client,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let gotify_url =
        env::var("GOTIFY_URL").unwrap_or_else(|_| "http://localhost:8080/message".to_string());

    // Resolve key with precedence:
    // 1) DOCKERMON_GOTIFY_KEY
    // 2) SPEEDY_GOTIFY_KEY
    // 3) GOTIFY_KEY
    // 4) GOTIFY_KEY_FILE (path to file containing only the token)
    let gotify_key = if let Ok(v) = env::var("DOCKERMON_GOTIFY_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() { v } else { String::new() }
    } else if let Ok(v) = env::var("SPEEDY_GOTIFY_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() { v } else { String::new() }
    } else if let Ok(v) = env::var("GOTIFY_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() { v } else { String::new() }
    } else if let Ok(path) = env::var("GOTIFY_KEY_FILE") {
        match std::fs::read_to_string(&path) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                eprintln!("GOTIFY_KEY_FILE read error from {}: {}", path, e);
                return Ok(());
            }
        }
    } else {
        String::new()
    };

    if gotify_key.is_empty() {
        eprintln!("GOTIFY_KEY not set (also checked DOCKERMON_GOTIFY_KEY/SPEEDY_GOTIFY_KEY); skipping Gotify notification.");
        return Ok(());
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
