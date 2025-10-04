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
    // 1) GOTIFY_KEY (explicit; caller may set this)
    // 2) DOCKERMON_GOTIFY_KEY (tool-specific)
    // 3) SPEEDY_GOTIFY_KEY (tool-specific)
    // 4) GOTIFY_KEY_FILE (path to file containing only the token)
    let mut key_source = "";
    let gotify_key = if let Ok(v) = env::var("GOTIFY_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() { key_source = "GOTIFY_KEY"; v } else { String::new() }
    } else if let Ok(v) = env::var("DOCKERMON_GOTIFY_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() { key_source = "DOCKERMON_GOTIFY_KEY"; v } else { String::new() }
    } else if let Ok(v) = env::var("SPEEDY_GOTIFY_KEY") {
        let v = v.trim().to_string();
        if !v.is_empty() { key_source = "SPEEDY_GOTIFY_KEY"; v } else { String::new() }
    } else if let Ok(path) = env::var("GOTIFY_KEY_FILE") {
        match std::fs::read_to_string(&path) {
            Ok(s) => { key_source = "GOTIFY_KEY_FILE"; s.trim().to_string() },
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

    // Optional debug output (mask token)
    let debug = env::var("GOTIFY_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if debug {
        let masked: String = if gotify_key.len() > 6 {
            format!("{}***{}", &gotify_key[..3], &gotify_key[gotify_key.len()-3..])
        } else { "***".to_string() };
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
