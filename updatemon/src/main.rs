use anyhow::Result;
use clap::Parser;
use common::{dotenv_init, http_client, send_gotify_updatemon};

mod types;
mod checkers;
mod executor;
mod docker;

use types::Server;
use checkers::get_checker;
use executor::RemoteExecutor;

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
    env_logger::init();

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

    for server in servers {
        let ssh_key_clone = ssh_key.clone();
        let docker_check = args.docker;
        let quiet = args.quiet;

        if !quiet {
            println!("Checking {}...", server.name);
        }

        // Spawn concurrent task for each server
        let task = tokio::spawn(async move {
            match check_server(&server, docker_check, ssh_key_clone.as_deref()).await {
                Ok(report) => report,
                Err(e) => {
                    eprintln!("Error checking {}: {}", server.name, e);
                    format!("❌ {} - Error: {}", server.name, e)
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
                eprintln!("Task join error: {}", e);
            }
        }
    }

    // Format and send notification
    let summary = format_summary(&all_reports);
    let details = all_reports.join("\n\n");

    if !args.quiet {
        println!("\n{}", details);
    }

    // Send to Gotify
    if let Err(e) = send_gotify_updatemon(&client, &summary, &details).await {
        eprintln!("Gotify send error: {e}");
    }

    Ok(())
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
