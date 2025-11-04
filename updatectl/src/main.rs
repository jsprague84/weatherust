use anyhow::Result;
use clap::{Parser, Subcommand};
use common::{dotenv_init, http_client, send_gotify_updatectl};

mod types;
mod executor;
mod updater;
mod checkers;

use types::Server;
use updater::{update_os, update_docker};

/// Update control tool - apply OS and Docker updates across multiple servers
#[derive(Parser, Debug)]
#[command(name = "updatectl")]
#[command(about = "Apply OS and Docker image updates across servers")]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// Comma-separated list of servers (name:user@host or just user@host)
    /// If not specified, uses UPDATE_SERVERS env var
    #[arg(long, global = true)]
    servers: Option<String>,

    /// Include local system in the update (can be combined with --servers)
    #[arg(long, global = true)]
    local: bool,

    /// SSH key path for remote connections
    #[arg(long, global = true)]
    ssh_key: Option<String>,

    /// Skip confirmation prompts (use with caution!)
    #[arg(long, short = 'y', global = true)]
    yes: bool,

    /// Dry-run mode - show what would be updated without making changes
    #[arg(long, global = true)]
    dry_run: bool,

    /// Suppress stdout output (Gotify only)
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Update OS packages only
    Os,

    /// Update Docker images only
    Docker {
        /// Update all Docker images
        #[arg(long)]
        all: bool,

        /// Update specific image(s) - comma-separated
        #[arg(long)]
        images: Option<String>,
    },

    /// Update both OS packages and Docker images
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv_init();
    env_logger::init();

    let args = Args::parse();
    let client = http_client();

    // Parse server list from args or env
    let mut servers = Vec::new();

    let server_str = args.servers
        .or_else(|| std::env::var("UPDATE_SERVERS").ok())
        .unwrap_or_default();

    if !server_str.is_empty() {
        servers.extend(parse_servers(&server_str)?);
    }

    if args.local {
        servers.push(Server::local());
    }

    let ssh_key = args.ssh_key
        .or_else(|| std::env::var("UPDATE_SSH_KEY").ok());

    if servers.is_empty() {
        eprintln!("No servers configured. Use --local and/or --servers or UPDATE_SERVERS env var.");
        eprintln!("Examples:");
        eprintln!("  updatectl os --local");
        eprintln!("  updatectl all --servers server1:ubuntu@192.168.1.10");
        eprintln!("  updatectl docker --all --local --servers cloud-vm1:ubuntu@cloud.example.com");
        std::process::exit(1);
    }

    // Confirmation prompt (unless --yes or --dry-run)
    if !args.yes && !args.dry_run {
        println!("This will update the following servers:");
        for server in &servers {
            println!("  - {} ({})", server.name, server.display_host());
        }
        println!();
        match &args.command {
            Commands::Os => println!("Operation: OS package updates"),
            Commands::Docker { all, images } => {
                if *all {
                    println!("Operation: Update ALL Docker images");
                } else if let Some(imgs) = images {
                    println!("Operation: Update Docker images: {}", imgs);
                } else {
                    println!("Operation: Update Docker images (specify --all or --images)");
                }
            }
            Commands::All => println!("Operation: OS packages + Docker images"),
        }
        println!();
        print!("Continue? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    if args.dry_run {
        println!("DRY-RUN MODE - No changes will be made\n");
    }

    // Execute updates on each server (in parallel)
    let mut tasks = Vec::new();

    for server in servers {
        let ssh_key_clone = ssh_key.clone();
        let quiet = args.quiet;
        let dry_run = args.dry_run;
        let command = args.command.clone();

        if !quiet {
            println!("Updating {}...", server.name);
        }

        let task = tokio::spawn(async move {
            match execute_update(&server, &command, dry_run, ssh_key_clone.as_deref()).await {
                Ok(report) => report,
                Err(e) => {
                    eprintln!("Error updating {}: {}", server.name, e);
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
    let summary = format_summary(&all_reports, args.dry_run);
    let details = all_reports.join("\n\n");

    if !args.quiet {
        println!("\n{}", details);
    }

    // Send to Gotify
    if let Err(e) = send_gotify_updatectl(&client, &summary, &details).await {
        eprintln!("Gotify send error: {e}");
    }

    Ok(())
}

async fn execute_update(
    server: &Server,
    command: &Commands,
    dry_run: bool,
    ssh_key: Option<&str>,
) -> Result<String> {
    use executor::RemoteExecutor;

    let executor = RemoteExecutor::new(server.clone(), ssh_key)?;
    let mut report_lines = Vec::new();

    let prefix = if dry_run { "[DRY-RUN] " } else { "" };
    report_lines.push(format!("{}🖥️  {} ({})", prefix, server.name, server.display_host()));

    match command {
        Commands::Os => {
            let result = update_os(&executor, dry_run).await?;
            report_lines.push(format!("   OS Updates: {}", result));
        }
        Commands::Docker { all, images } => {
            let result = update_docker(&executor, *all, images.as_deref(), dry_run).await?;
            report_lines.push(format!("   Docker Updates: {}", result));
        }
        Commands::All => {
            let os_result = update_os(&executor, dry_run).await?;
            report_lines.push(format!("   OS Updates: {}", os_result));

            let docker_result = update_docker(&executor, true, None, dry_run).await?;
            report_lines.push(format!("   Docker Updates: {}", docker_result));
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

fn format_summary(reports: &[String], dry_run: bool) -> String {
    let server_count = reports.len();
    let prefix = if dry_run { "[DRY-RUN] " } else { "" };

    if reports.iter().any(|r| r.contains("Error")) {
        format!("{}⚠️  Updates completed with errors ({} servers)", prefix, server_count)
    } else {
        format!("{}✅ Updates completed successfully ({} servers)", prefix, server_count)
    }
}
