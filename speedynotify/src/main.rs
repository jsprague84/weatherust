use clap::Parser;
use common::{dotenv_init, http_client, send_gotify};
use serde::Deserialize;
use std::env;
use tokio::process::Command;

#[derive(Parser, Debug)]
#[command(name = "speedynotify")]
#[command(about = "Run Ookla speedtest and send Gotify summary")] 
struct Args {
    /// Minimum acceptable download speed in Mbps
    #[arg(long)]
    min_down: Option<f64>,

    /// Minimum acceptable upload speed in Mbps
    #[arg(long)]
    min_up: Option<f64>,

    /// Optional server id to target
    #[arg(long)]
    server_id: Option<u32>,

    /// Suppress stdout; only send Gotify
    #[arg(long, default_value_t = false)]
    quiet: bool,
}

#[derive(Debug, Deserialize)]
struct OoklaResult {
    ping: Ping,
    download: Transfer,
    upload: Transfer,
    isp: Option<String>,
    interface: Option<Interface>,
    server: Option<Server>,
}

#[derive(Debug, Deserialize)]
struct Ping { latency: f64 }

#[derive(Debug, Deserialize)]
struct Transfer { bandwidth: f64 }

#[derive(Debug, Deserialize)]
struct Interface { name: Option<String> }

#[derive(Debug, Deserialize)]
struct Server { id: Option<u32>, name: Option<String>, location: Option<String> }

// speedtest-cli (Python) JSON format
#[derive(Debug, Deserialize)]
struct PyResult {
    download: f64,
    upload: f64,
    ping: f64,
    client: Option<PyClient>,
    server: Option<PyServer>,
}

#[derive(Debug, Deserialize)]
struct PyClient { isp: Option<String> }

#[derive(Debug, Deserialize)]
struct PyServer { id: Option<String>, name: Option<String>, sponsor: Option<String> }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv_init();
    let args = Args::parse();

    // If a separate token is provided for speedynotify, prefer it locally
    if let Ok(tok) = std::env::var("SPEEDY_GOTIFY_KEY") {
        if !tok.trim().is_empty() {
            std::env::set_var("GOTIFY_KEY", tok);
        }
    }

    // Resolve thresholds with env fallbacks
    let min_down = args
        .min_down
        .or_else(|| env::var("SPEEDTEST_MIN_DOWN").ok()?.parse().ok());
    let min_up = args
        .min_up
        .or_else(|| env::var("SPEEDTEST_MIN_UP").ok()?.parse().ok());
    let server_id = args
        .server_id
        .or_else(|| env::var("SPEEDTEST_SERVER_ID").ok()?.parse().ok());

    // Try Ookla CLI first; fall back to python speedtest-cli if needed
    match run_and_parse_ookla(server_id).await {
        Ok((down_mbps, up_mbps, ping_ms, isp, iface, server)) => {
            emit_and_notify(args.quiet, down_mbps, up_mbps, ping_ms, isp, iface, server, min_down, min_up).await?;
        }
        Err(e) => {
            let err_s = format!("{}", e).to_lowercase();
            // If Ookla flags are not recognized, try without acceptance flags
            if err_s.contains("unknown option") || err_s.contains("unrecognized option") {
                if let Ok((down_mbps, up_mbps, ping_ms, isp, iface, server)) = run_and_parse_ookla_no_accept(server_id).await {
                    emit_and_notify(args.quiet, down_mbps, up_mbps, ping_ms, isp, iface, server, min_down, min_up).await?;
                    return Ok(());
                }
            }
            eprintln!("Ookla speedtest attempt failed: {}\nFalling back to python speedtest-cli if available...", e);
            match run_and_parse_python(server_id).await {
                Ok((down_mbps, up_mbps, ping_ms, isp, iface, server)) => {
                    emit_and_notify(args.quiet, down_mbps, up_mbps, ping_ms, isp, iface, server, min_down, min_up).await?;
                }
                Err(e2) => {
                    // Avoid launching GUI variants of 'speedtest' by default
                    let allow_text = std::env::var("SPEEDY_ALLOW_TEXT_FALLBACK")
                        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false);
                    if allow_text {
                        eprintln!(
                            "Python speedtest-cli unavailable: {}\nAttempting to parse plain text output from 'speedtest' (SPEEDY_ALLOW_TEXT_FALLBACK=1)...",
                            e2
                        );
                        let (down_mbps, up_mbps, ping_ms, isp, iface, server) = run_and_parse_text().await?;
                        emit_and_notify(args.quiet, down_mbps, up_mbps, ping_ms, isp, iface, server, min_down, min_up).await?;
                    } else {
                        eprintln!(
                            "No JSON-capable speedtest CLI found. Install 'speedtest-cli' (python) and retry.\nFedora: sudo dnf install -y speedtest-cli  (or: sudo dnf install -y python3-speedtest-cli)\nOr via pipx: pipx install speedtest-cli"
                        );
                        return Err("speedtest-cli not installed".into());
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_and_parse_ookla(server_id: Option<u32>) -> Result<(f64, f64, f64, String, String, String), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("speedtest");
    cmd.arg("--accept-license").arg("--accept-gdpr").arg("-f").arg("json");
    if let Some(id) = server_id { cmd.arg("-s").arg(id.to_string()); }
    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Ookla speedtest exited {}: {}", output.status, stderr).into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let res: OoklaResult = serde_json::from_str(&stdout)?;
    let down_mbps = (res.download.bandwidth * 8.0) / 1_000_000.0;
    let up_mbps = (res.upload.bandwidth * 8.0) / 1_000_000.0;
    let ping_ms = res.ping.latency;
    let iface = res.interface.and_then(|i| i.name).unwrap_or_default();
    let server = res
        .server
        .map(|s| format!(
            "{}{}{}",
            s.name.unwrap_or_default(),
            s.location.map(|l| format!(", {}", l)).unwrap_or_default(),
            s.id.map(|i| format!(" (#{})", i)).unwrap_or_default()
        ))
        .unwrap_or_default();
    let isp = res.isp.unwrap_or_default();
    Ok((down_mbps, up_mbps, ping_ms, isp, iface, server))
}

async fn run_and_parse_ookla_no_accept(server_id: Option<u32>) -> Result<(f64, f64, f64, String, String, String), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("speedtest");
    cmd.arg("-f").arg("json");
    if let Some(id) = server_id { cmd.arg("-s").arg(id.to_string()); }
    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Ookla speedtest (no-accept) exited {}: {}", output.status, stderr).into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let res: OoklaResult = serde_json::from_str(&stdout)?;
    let down_mbps = (res.download.bandwidth * 8.0) / 1_000_000.0;
    let up_mbps = (res.upload.bandwidth * 8.0) / 1_000_000.0;
    let ping_ms = res.ping.latency;
    let iface = res.interface.and_then(|i| i.name).unwrap_or_default();
    let server = res
        .server
        .map(|s| format!(
            "{}{}{}",
            s.name.unwrap_or_default(),
            s.location.map(|l| format!(", {}", l)).unwrap_or_default(),
            s.id.map(|i| format!(" (#{})", i)).unwrap_or_default()
        ))
        .unwrap_or_default();
    let isp = res.isp.unwrap_or_default();
    Ok((down_mbps, up_mbps, ping_ms, isp, iface, server))
}

async fn run_and_parse_python(server_id: Option<u32>) -> Result<(f64, f64, f64, String, String, String), Box<dyn std::error::Error>> {
    // Try python variants, preferring HTTPS (--secure) to avoid 403s
    let candidates: &[(&str, &[&str])] = &[
        ("speedtest-cli", &["--json", "--secure"][..]),
        ("speedtest-cli", &["--json"][..]),
        ("python3", &["-m", "speedtest", "--json", "--secure"][..]),
        ("python3", &["-m", "speedtest", "--json"][..]),
        ("python", &["-m", "speedtest", "--json", "--secure"][..]),
        ("python", &["-m", "speedtest", "--json"][..]),
        ("speedtest", &["--json"][..]),
    ];
    for (bin, base_args) in candidates {
        let mut args: Vec<String> = base_args.iter().map(|s| s.to_string()).collect();
        if let Some(id) = server_id {
            args.push("-s".into());
            args.push(id.to_string());
        }
        let out = Command::new(bin).args(&args).output().await;
        match out {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let res: PyResult = serde_json::from_str(&stdout)?;
                let down_mbps = res.download / 1_000_000.0;
                let up_mbps = res.upload / 1_000_000.0;
                let ping_ms = res.ping;
                let isp = res.client.and_then(|c| c.isp).unwrap_or_default();
                let iface = String::new();
                let server = res.server.map(|s| {
                    let name = s.name.unwrap_or_default();
                    let sponsor = s.sponsor.unwrap_or_default();
                    let id = s.id.unwrap_or_default();
                    if !id.is_empty() { format!("{} ({}) #{}", name, sponsor, id) } else { format!("{} ({})", name, sponsor) }
                }).unwrap_or_default();
                return Ok((down_mbps, up_mbps, ping_ms, isp, iface, server));
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("{} failed with {}\n{}", bin, output.status, stderr);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Err("No compatible speedtest CLI found".into())
}

async fn emit_and_notify(
    quiet: bool,
    down_mbps: f64,
    up_mbps: f64,
    ping_ms: f64,
    isp: String,
    iface: String,
    server: String,
    min_down: Option<f64>,
    min_up: Option<f64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    lines.push(format!("ISP: {} | IF: {} | Server: {}", isp, iface, server));
    lines.push(format!("Down: {:.2} Mbps | Up: {:.2} Mbps | Ping: {:.1} ms", down_mbps, up_mbps, ping_ms));
    let human = lines.join("\n");

    let mut degraded = false;
    if let Some(min) = min_down { if down_mbps < min { degraded = true; } }
    if let Some(min) = min_up { if up_mbps < min { degraded = true; } }

    if !quiet { println!("{}", human); }

    let client = http_client();
    let title = if degraded { "Speedtest: Degraded" } else { "Speedtest: OK" };
    if let Err(e) = send_gotify(&client, title, &human).await {
        eprintln!("Gotify send error: {e}");
    }
    Ok(())
}

async fn run_and_parse_text() -> Result<(f64, f64, f64, String, String, String), Box<dyn std::error::Error>> {
    let output = Command::new("speedtest").output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("'speedtest' failed: {}\n{}", output.status, stderr).into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut down_mbps: Option<f64> = None;
    let mut up_mbps: Option<f64> = None;
    let mut ping_ms: Option<f64> = None;

    for line in stdout.lines() {
        let l = line.trim();
        let lower = l.to_lowercase();
        if down_mbps.is_none() && lower.contains("download") {
            down_mbps = parse_speed_line(l);
        }
        if up_mbps.is_none() && lower.contains("upload") {
            up_mbps = parse_speed_line(l);
        }
        if ping_ms.is_none() && (lower.contains("ping") || lower.contains("latency")) {
            ping_ms = parse_first_number(l);
        }
    }

    let down = down_mbps.ok_or("could not parse download speed from text output")?;
    let up = up_mbps.ok_or("could not parse upload speed from text output")?;
    let ping = ping_ms.unwrap_or(0.0);
    Ok((down, up, ping, String::new(), String::new(), String::new()))
}

fn parse_speed_line(s: &str) -> Option<f64> {
    // Extract first float and unit, normalize to Mbps
    let num = parse_first_number(s)?;
    let sl = s.to_lowercase();
    if sl.contains("gbps") || sl.contains("gbit/s") { Some(num * 1000.0) }
    else if sl.contains("mbps") || sl.contains("mbit/s") { Some(num) }
    else if sl.contains("kbps") || sl.contains("kbit/s") { Some(num / 1000.0) }
    else if sl.contains("bps") { Some(num / 1_000_000.0) } // bits per second
    else { Some(num) }
}

fn parse_first_number(s: &str) -> Option<f64> {
    let mut start = None;
    let mut end = None;
    for (i, ch) in s.char_indices() {
        if start.is_none() {
            if ch.is_ascii_digit() { start = Some(i); }
        } else if !(ch.is_ascii_digit() || ch == '.' ) {
            end = Some(i); break;
        }
    }
    let sfx = match (start, end) { (Some(a), Some(b)) => &s[a..b], (Some(a), None) => &s[a..], _ => return None };
    sfx.parse::<f64>().ok()
}
