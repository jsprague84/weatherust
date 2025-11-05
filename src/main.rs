use std::env;
use std::io::{self, Write};

use chrono::{FixedOffset, TimeZone};
use clap::Parser;
use common::{dotenv_init, send_gotify_weatherust, send_ntfy_weatherust};
use reqwest::Client;
use serde::Deserialize;

/// CLI flags for non-interactive runs (systemd, cron, n8n)
#[derive(Parser, Debug)]
#[command(name = "weatherust")]
#[command(about = "Weather -> Gotify (current + next 6 days)")]
struct Args {
    /// ZIP code (e.g., 52726). If present, skips prompt.
    #[arg(long)]
    zip: Option<String>,

    /// Free-form location (e.g., "Davenport,IA,US"). Used if --zip is not given.
    #[arg(long)]
    location: Option<String>,

    /// Units: "imperial" (°F) or "metric" (°C). If omitted, uses DEFAULT_UNITS env or falls back to "imperial".
    #[arg(long)]
    units: Option<String>,

    /// If set, don't print to stdout; only send Gotify
    #[arg(long, default_value_t = false)]
    quiet: bool,
}

#[derive(Debug, Deserialize)]
struct GeoResult {
    name: String,
    lat: f64,
    lon: f64,
    country: String,
    state: Option<String>,
}

// For ZIP geocoding (returns a single object)
#[derive(Debug, Deserialize)]
struct ZipGeoResult {
    name: String,
    lat: f64,
    lon: f64,
    country: String,
}

#[derive(Debug, Deserialize)]
struct OneCall {
    timezone: String,
    timezone_offset: i32, // seconds
    current: Current,
    daily: Vec<Daily>,
}

#[derive(Debug, Deserialize)]
struct Current {
    dt: i64, // unix seconds
    temp: f64,
    humidity: u8,
    weather: Vec<Weather>,
}

#[derive(Debug, Deserialize)]
struct Daily {
    dt: i64,
    temp: DailyTemp,
    weather: Vec<Weather>,
}

#[derive(Debug, Deserialize)]
struct DailyTemp {
    min: f64,
    max: f64,
}

#[derive(Debug, Deserialize)]
struct Weather {
    description: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv_init(); // load .env if present

    let args = Args::parse();

    let api_key = env::var("OWM_API_KEY").expect("Missing OWM_API_KEY in environment or .env file");

    // Units: CLI flag -> DEFAULT_UNITS env -> "imperial"
    let units = args
        .units
        .clone()
        .or_else(|| env::var("DEFAULT_UNITS").ok())
        .unwrap_or_else(|| "imperial".to_string())
        .to_lowercase();

    // Create one HTTP client for all requests
    let client = Client::new();

    // Resolve location to lat/lon and a pretty display name
    let (lat, lon, pretty_location) = resolve_location(&client, &api_key, &args).await?;

    // ---- One Call daily forecast + current ----
    // If your account lacks One Call 3.0, change the path to /data/2.5/onecall
    let onecall_url = format!(
        "https://api.openweathermap.org/data/3.0/onecall?lat={lat}&lon={lon}&exclude=minutely,hourly,alerts&units={units}&appid={api_key}"
    );
    let oc_resp = client.get(&onecall_url).send().await?.error_for_status()?;
    let data: OneCall = oc_resp.json().await?;

    // timezone-aware timestamp for "current"
    let offset =
        FixedOffset::east_opt(data.timezone_offset).expect("invalid timezone offset from API");
    let current_time = offset.timestamp_opt(data.current.dt, 0).unwrap();

    let current_desc = data
        .current
        .weather
        .get(0)
        .map(|w| w.description.as_str())
        .unwrap_or("no description");

    // Build human-readable output and a concise Gotify message
    let (unit_label, degree) = if units == "metric" {
        ("°C", "°C")
    } else {
        ("°F", "°F")
    };

    // Today is daily[0]
    let today_high = data.daily.get(0).map(|d| d.temp.max);
    let today_low = data.daily.get(0).map(|d| d.temp.min);

    // Detailed multi-line body
    let mut lines = Vec::new();
    lines.push(format!(
        "Location: {}\nTimezone: {}\nNow: {} | {} | Temp: {:.1} {} | Humidity: {}%",
        pretty_location,
        data.timezone,
        current_time,
        current_desc,
        data.current.temp,
        unit_label,
        data.current.humidity
    ));
    lines.push("\nNext 7 days (high/low):".to_string());

    for day in data.daily.iter().skip(1).take(7) {
        let dt = offset.timestamp_opt(day.dt, 0).unwrap();
        let label = dt.format("%a %d").to_string();
        let desc = day
            .weather
            .get(0)
            .map(|w| w.description.as_str())
            .unwrap_or("n/a");
        lines.push(format!(
            "  {label}: {:>5.1}{deg}/{:>5.1}{deg}  ({desc})",
            day.temp.max,
            day.temp.min,
            deg = degree
        ));
    }

    let human_output = lines.join("\n");

    // Concise single-line summary for Gotify title/message
    let summary = match (today_high, today_low) {
        (Some(h), Some(l)) => format!(
            "Now: {:.1}{} ({}) | Today H/L: {:.1}{}/ {:.1}{}",
            data.current.temp, degree, current_desc, h, degree, l, degree
        ),
        _ => format!("Now: {:.1}{} ({})", data.current.temp, degree, current_desc),
    };

    // Print unless --quiet
    if !args.quiet {
        println!("{}", human_output);
    }

    // Send to Gotify (if configured)
    if let Err(e) = send_gotify_weatherust(&client, &summary, &human_output).await {
        eprintln!("Gotify send error: {e}");
    }

    // Send to ntfy.sh (if configured)
    if let Err(e) = send_ntfy_weatherust(&client, &summary, &human_output, None).await {
        eprintln!("ntfy send error: {e}");
    }

    Ok(())
}

// ----------------- helpers -----------------

async fn resolve_location(
    client: &Client,
    api_key: &str,
    args: &Args,
) -> Result<(f64, f64, String), Box<dyn std::error::Error>> {
    // Highest priority: explicit CLI flags
    if let Some(zip) = args.zip.as_deref() {
        return geocode_zip(client, api_key, zip).await;
    }

    if let Some(loc) = args.location.as_deref() {
        return geocode_location(client, api_key, loc).await;
    }

    // Next: environment-provided defaults
    if let Ok(zip) = env::var("DEFAULT_ZIP") {
        if !zip.trim().is_empty() {
            return geocode_zip(client, api_key, zip.trim()).await;
        }
    }
    if let Ok(loc) = env::var("DEFAULT_LOCATION") {
        if !loc.trim().is_empty() {
            return geocode_location(client, api_key, loc.trim()).await;
        }
    }

    // Interactive fallback if no flags provided
    print!("Enter location (e.g., \"Davenport,IA,US\" or ZIP like \"52801\"): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        return Err("no input provided".into());
    }

    if looks_like_zip(input) {
        geocode_zip(client, api_key, input).await
    } else {
        geocode_location(client, api_key, input).await
    }
}

async fn geocode_zip(
    client: &Client,
    api_key: &str,
    zip_in: &str,
) -> Result<(f64, f64, String), Box<dyn std::error::Error>> {
    let (zip, cc) = split_zip_and_cc(zip_in);
    let url = format!("https://api.openweathermap.org/geo/1.0/zip?zip={zip},{cc}&appid={api_key}");
    let resp = client.get(&url).send().await?.error_for_status()?;
    let z: ZipGeoResult = resp.json().await?;
    Ok((z.lat, z.lon, format!("{}, {}", z.name, z.country)))
}

async fn geocode_location(
    client: &Client,
    api_key: &str,
    input: &str,
) -> Result<(f64, f64, String), Box<dyn std::error::Error>> {
    let q = normalize_city_query(input);
    let url =
        format!("https://api.openweathermap.org/geo/1.0/direct?q={q}&limit=1&appid={api_key}");
    let resp = client.get(&url).send().await?.error_for_status()?;
    let mut v: Vec<GeoResult> = resp.json().await?;
    if v.is_empty() {
        return Err(format!(
            "Could not find coordinates for \"{input}\".\nHint: try \"City,STATE,US\" (e.g., Davenport,IA,US) or use a ZIP code."
        ).into());
    }
    let loc = v.remove(0);
    let pretty = format!(
        "{}{}{}",
        loc.name,
        loc.state
            .as_ref()
            .map(|s| format!(", {}", s))
            .unwrap_or_default(),
        format!(", {}", loc.country)
    );
    Ok((loc.lat, loc.lon, pretty))
}

fn looks_like_zip(s: &str) -> bool {
    // US ZIP 5 or 5-4, or "ZIP,CC"
    let core = s.split(',').next().unwrap_or("");
    let digits5 = core.len() == 5 && core.chars().all(|c| c.is_ascii_digit());
    let zip4 = core.len() == 10
        && core[0..5].chars().all(|c| c.is_ascii_digit())
        && &core[5..6] == "-"
        && core[6..10].chars().all(|c| c.is_ascii_digit());
    digits5 || zip4
}

fn split_zip_and_cc(s: &str) -> (String, String) {
    let parts = s.split(',').map(|p| p.trim()).collect::<Vec<_>>();
    let zip = parts.get(0).copied().unwrap_or("").to_string();
    let cc = parts
        .get(1)
        .map(|v| (*v).to_string())
        .unwrap_or_else(|| "US".to_string());
    (zip, cc)
}

fn normalize_city_query(input: &str) -> String {
    // Accept: "City", "City,ST", "City,ST,CC", "City,CC"
    // If there are exactly 2 parts and second is 2 letters, assume it's a US state -> append ",US"
    let parts = input.split(',').map(|p| p.trim()).collect::<Vec<_>>();
    match parts.len() {
        1 => parts[0].to_string(),
        2 => {
            let second = parts[1];
            if second.len() == 2 && second.chars().all(|c| c.is_ascii_alphabetic()) {
                format!("{},{}", parts[0], format!("{},US", second))
            } else {
                // assume it's country code already
                format!("{},{}", parts[0], second)
            }
        }
        _ => parts.join(","),
    }
}

// moved to common::send_gotify
