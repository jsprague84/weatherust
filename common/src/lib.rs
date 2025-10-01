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

    // Prefer GOTIFY_KEY; optionally support GOTIFY_KEY_FILE if set
    let gotify_key = match env::var("GOTIFY_KEY") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            if let Ok(path) = env::var("GOTIFY_KEY_FILE") {
                match std::fs::read_to_string(&path) {
                    Ok(s) => s.trim().to_string(),
                    Err(e) => {
                        eprintln!("GOTIFY_KEY_FILE read error from {}: {}", path, e);
                        return Ok(());
                    }
                }
            } else {
                eprintln!("GOTIFY_KEY not set; skipping Gotify notification.");
                return Ok(());
            }
        }
    };

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

