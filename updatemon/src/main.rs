use anyhow::Result;
use clap::Parser;
use common::{dotenv_init, http_client, send_gotify_updatemon, send_ntfy_updatemon, NtfyAction};
use reqwest::Client;
use tracing::error;

mod types;
mod checkers;
mod executor;
mod docker;

use types::Server;
use checkers::get_checker;
use common::RemoteExecutor;
use executor::UpdatemonExecutor;

/// Update monitoring tool - checks for OS and Docker updates across multiple servers
#[derive(Parser, Debug)]
#[command(name = "updatemon")]
#[command(about = "Monitor package and Docker image updates across servers")]
struct Args {
    /// Comma-separated list of servers (name:user@host or just user@host)
    #[arg(long)]
    servers: Option<String>,

    /// Include local system in the check (can be combined with --servers)
    #[arg(long)]
    local: bool,

    /// Check Docker images for updates
    #[arg(long, default_value_t = true)]
    docker: bool,

    /// SSH key path for remote connections
    #[arg(long)]
    ssh_key: Option<String>,

    /// Suppress stdout output (Gotify only)
    #[arg(long, default_value_t = false)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv_init();

    // Initialize tracing (also bridges log macros)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    // Initialize tracing-log bridge for legacy log macros
    tracing_log::LogTracer::init().ok();

    let args = Args::parse();
    let client = http_client();

    // Parse server list from args or env
    let mut servers = Vec::new();

    // Add remote servers if specified
    let server_str = args.servers
        .or_else(|| std::env::var("UPDATE_SERVERS").ok())
        .unwrap_or_default();

    if !server_str.is_empty() {
        servers.extend(parse_servers(&server_str)?);
    }

    // Add localhost if --local flag is set
    if args.local {
        servers.push(Server::local());
    }

    let ssh_key = args.ssh_key
        .or_else(|| std::env::var("UPDATE_SSH_KEY").ok());

    if servers.is_empty() {
        eprintln!("No servers configured. Use --local and/or --servers or UPDATE_SERVERS env var.");
        eprintln!("Examples:");
        eprintln!("  --local                                           (check local system only)");
        eprintln!("  --servers server1:ubuntu@192.168.1.10             (check remote server)");
        eprintln!("  --local --servers server1:ubuntu@192.168.1.10     (check both local and remote)");
        std::process::exit(1);
    }

    // Check each server for updates (in parallel using tokio tasks)
    let mut tasks = Vec::new();

    for server in &servers {
        let ssh_key_clone = ssh_key.clone();
        let docker_check = args.docker;
        let quiet = args.quiet;
        let server_clone = server.clone();

        if !quiet {
            println!("Checking {}...", server.name);
        }

        // Spawn concurrent task for each server
        let task = tokio::spawn(async move {
            match check_server(&server_clone, docker_check, ssh_key_clone.as_deref()).await {
                Ok(report) => report,
                Err(e) => {
                    error!(server = %server_clone.name, error = %e, "Error checking server");
                    format!("❌ {} - Error: {}", server_clone.name, e)
                }
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let mut all_reports = Vec::new();
    for task in tasks {
        match task.await {
            Ok(report) => all_reports.push(report),
            Err(e) => {
                error!(error = %e, "Task join error");
            }
        }
    }

    // Format and send notification
    let summary = format_summary(&all_reports);
    let details = all_reports.join("\n\n");

    if !args.quiet {
        println!("\n{}", details);
    }

    // Send to Gotify (if configured) - full details
    if let Err(e) = send_gotify_updatemon(&client, &summary, &details).await {
        error!(error = %e, "Failed to send Gotify notification");
    }

    // Send to ntfy.sh (if configured) with action buttons - one notification per server
    send_ntfy_per_server(&client, &all_reports, &servers).await;

    Ok(())
}

/// Send individual ntfy notifications per server (only for servers with updates)
async fn send_ntfy_per_server(client: &Client, reports: &[String], servers: &[Server]) {
    for (report, server) in reports.iter().zip(servers.iter()) {
        let has_os_updates = report.contains("📦") && report.contains("OS:");
        let has_docker_updates = report.contains("🐳") && report.contains("Docker:");

        // Only send notification if server has updates
        if !has_os_updates && !has_docker_updates {
            continue;
        }

        // Generate title
        let mut update_types = Vec::new();
        if has_os_updates {
            update_types.push("OS");
        }
        if has_docker_updates {
            update_types.push("Docker");
        }
        let title = format!("{} - {} updates available", server.name, update_types.join(" + "));

        // Use the full report as message (it's already concise per-server)
        let message = report.clone();

        // Generate action buttons for this specific server
        let actions = generate_server_action_buttons(report, server);

        // Send notification
        if let Err(e) = send_ntfy_updatemon(client, &title, &message, Some(actions)).await {
            error!(server = %server.name, error = %e, "Failed to send ntfy notification");
        }
    }
}

/// Generate action buttons for a single server's ntfy notification
fn generate_server_action_buttons(report: &str, server: &Server) -> Vec<NtfyAction> {
    let webhook_url = std::env::var("UPDATECTL_WEBHOOK_URL")
        .unwrap_or_else(|_| "http://updatectl_webhook:8080".to_string());
    let webhook_secret = std::env::var("UPDATECTL_WEBHOOK_SECRET")
        .unwrap_or_default();

    if webhook_secret.is_empty() {
        // No webhook secret configured - can't generate secure buttons
        return Vec::new();
    }

    let has_os_updates = report.contains("📦") && report.contains("OS:");
    let has_docker_updates = report.contains("🐳") && report.contains("Docker:");

    let mut actions = Vec::new();
    let server_name_encoded = urlencoding::encode(&server.name);
    let token_encoded = urlencoding::encode(&webhook_secret);

    // Add OS update button if needed
    if has_os_updates {
        let url = format!(
            "{}/webhook/update/os?server={}&token={}",
            webhook_url, server_name_encoded, token_encoded
        );
        actions.push(
            NtfyAction::http_post("Update OS", &url)
        );
    }

    // Add Docker update button if needed
    if has_docker_updates {
        let url = format!(
            "{}/webhook/update/docker/all?server={}&token={}",
            webhook_url, server_name_encoded, token_encoded
        );
        actions.push(
            NtfyAction::http_post("Update Docker", &url)
        );
    }

    // We have room for up to 3 buttons per server
    // Could add a third button here for individual Docker image updates if needed

    actions
}

async fn check_server(server: &Server, check_docker: bool, ssh_key: Option<&str>) -> Result<String> {
    let executor = RemoteExecutor::new(server.clone(), ssh_key)?;

    let mut report_lines = Vec::new();
    report_lines.push(format!("🖥️  {} ({})", server.name, server.display_host()));

    // Detect package manager
    let pm = executor.detect_package_manager().await?;
    report_lines.push(format!("   Package Manager: {}", pm.display_name()));

    // Check OS updates
    let checker = get_checker(&pm);
    let updates = executor.check_updates(&checker).await?;

    if updates.is_empty() {
        report_lines.push("   OS: ✅ Up to date".to_string());
    } else {
        report_lines.push(format!("   OS: 📦 {} updates available", updates.len()));
        for update in updates.iter().take(5) {
            report_lines.push(format!("      - {}", update));
        }
        if updates.len() > 5 {
            report_lines.push(format!("      ... and {} more", updates.len() - 5));
        }
    }

    // Check Docker images if enabled
    if check_docker {
        match docker::check_docker_updates(&executor).await {
            Ok(images) => {
                if images.is_empty() {
                    report_lines.push("   Docker: No images found".to_string());
                } else {
                    let updates_available = images.iter().filter(|img| img.has_update).count();
                    if updates_available > 0 {
                        report_lines.push(format!("   Docker: 🐳 {} of {} images with updates", updates_available, images.len()));
                        // Show images with updates first
                        for image in images.iter().filter(|img| img.has_update).take(5) {
                            report_lines.push(format!("      - {}", image));
                        }
                        let remaining = updates_available.saturating_sub(5);
                        if remaining > 0 {
                            report_lines.push(format!("      ... and {} more with updates", remaining));
                        }
                    } else {
                        report_lines.push(format!("   Docker: ✅ {} images up to date", images.len()));
                    }
                }
            }
            Err(e) => {
                log::warn!("Error checking Docker images: {}", e);
                report_lines.push(format!("   Docker: ⚠️  Error: {}", e));
            }
        }
    }

    Ok(report_lines.join("\n"))
}

fn parse_servers(input: &str) -> Result<Vec<Server>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    input.split(',')
        .map(|s| Server::parse(s.trim()))
        .collect()
}

fn format_summary(reports: &[String]) -> String {
    let server_count = reports.len();
    let has_updates = reports.iter().any(|r| r.contains("📦"));

    if has_updates {
        format!("📦 Updates available ({} servers)", server_count)
    } else {
        format!("✅ All systems up to date ({} servers)", server_count)
    }
}
