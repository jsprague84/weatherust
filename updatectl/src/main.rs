use anyhow::Result;
use clap::{Parser, Subcommand};
use common::{dotenv_init, http_client, send_gotify_updatectl, send_ntfy_updatectl};

mod types;
mod executor;
mod updater;
mod checkers;
mod webhook;

use types::Server;
use updater::{update_os, update_docker};

/// Update control tool - apply OS and Docker updates across multiple servers
#[derive(Parser, Debug)]
#[command(name = "updatectl")]
#[command(about = "Apply OS and Docker image updates across servers")]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// Comma-separated server names or connection strings
    /// Names are looked up from UPDATE_SERVERS (run 'list servers' to see available)
    /// Examples: --servers "Cloud VM1" or --servers "myserver:user@host"
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

    /// List available servers or show examples
    List {
        #[command(subcommand)]
        what: ListCommands,
    },

    /// Start webhook server for remote-triggered updates
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum ListCommands {
    /// List configured servers from UPDATE_SERVERS
    Servers,
    /// Show usage examples
    Examples,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv_init();
    env_logger::init();

    let args = Args::parse();
    let client = http_client();

    // Build server registry from UPDATE_SERVERS env var for name lookups
    let server_registry = build_server_registry()?;

    // Handle list commands early (no server connection needed)
    if let Commands::List { what } = &args.command {
        match what {
            ListCommands::Servers => {
                print_servers(&server_registry);
                return Ok(());
            }
            ListCommands::Examples => {
                print_examples();
                return Ok(());
            }
        }
    }

    // Handle serve command (webhook server mode)
    if let Commands::Serve { port } = &args.command {
        let secret = std::env::var("UPDATECTL_WEBHOOK_SECRET")
            .expect("UPDATECTL_WEBHOOK_SECRET must be set for webhook server");

        if secret.len() < 32 {
            eprintln!("Warning: UPDATECTL_WEBHOOK_SECRET should be at least 32 characters");
        }

        let ssh_key = args.ssh_key
            .or_else(|| std::env::var("UPDATE_SSH_KEY").ok());

        println!("Starting webhook server...");
        println!("Configured servers: {}", server_registry.len());
        for (name, server) in &server_registry {
            println!("  {} -> {}", name, server.display_host());
        }
        println!();

        return webhook::serve_webhooks(*port, secret, server_registry, ssh_key).await;
    }

    // Parse server list from args or env
    let mut servers = Vec::new();

    if let Some(server_names) = &args.servers {
        // User specified servers - resolve names from registry
        servers.extend(resolve_servers(server_names, &server_registry)?);
    } else if !args.local {
        // No --servers and no --local - use all servers from UPDATE_SERVERS
        servers.extend(server_registry.values().cloned());
    }
    // If only --local is set, servers stays empty (will add localhost below)

    if args.local {
        servers.push(Server::local());
    }

    let ssh_key = args.ssh_key
        .or_else(|| std::env::var("UPDATE_SSH_KEY").ok());

    if servers.is_empty() {
        eprintln!("No servers specified.");
        eprintln!();
        eprintln!("Available options:");
        eprintln!("  updatectl list servers              Show configured servers");
        eprintln!("  updatectl list examples             Show usage examples");
        eprintln!("  updatectl os --local                Update localhost");
        eprintln!("  updatectl os --servers \"name\"       Update named server");
        eprintln!();
        eprintln!("Configure servers in UPDATE_SERVERS environment variable.");
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
            Commands::List { .. } => {
                // Already handled early - this shouldn't be reached
                unreachable!("List commands should be handled before confirmation prompt")
            }
            Commands::Serve { .. } => {
                // Already handled early - this shouldn't be reached
                unreachable!("Serve command should be handled before confirmation prompt")
            }
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

    // Send to Gotify (if configured)
    if let Err(e) = send_gotify_updatectl(&client, &summary, &details).await {
        eprintln!("Gotify send error: {e}");
    }

    // Send to ntfy.sh (if configured)
    if let Err(e) = send_ntfy_updatectl(&client, &summary, &details, None).await {
        eprintln!("ntfy send error: {e}");
    }

    Ok(())
}

async fn execute_update(
    server: &Server,
    command: &Commands,
    dry_run: bool,
    ssh_key: Option<&str>,
) -> Result<String> {
    use common::RemoteExecutor;
    use crate::executor::UpdatectlExecutor;

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
        Commands::List { .. } => {
            // Already handled early - this shouldn't be reached
            unreachable!("List commands should be handled before server execution")
        }
        Commands::Serve { .. } => {
            // Already handled early - this shouldn't be reached
            unreachable!("Serve command should be handled before server execution")
        }
    }

    Ok(report_lines.join("\n"))
}

/// Build a registry of server name -> Server from UPDATE_SERVERS env var
fn build_server_registry() -> Result<std::collections::HashMap<String, Server>> {
    use std::collections::HashMap;

    let server_str = std::env::var("UPDATE_SERVERS").unwrap_or_default();
    let mut registry = HashMap::new();

    if !server_str.is_empty() {
        for server_def in server_str.split(',') {
            let server = Server::parse(server_def.trim())?;
            registry.insert(server.name.clone(), server);
        }
    }

    Ok(registry)
}

/// Resolve comma-separated server names/specs using the registry
/// Supports:
/// - Server names: "Cloud VM1" -> looks up in registry
/// - Full specs: "myserver:user@host" -> parses directly
/// - Mixed: "Cloud VM1,newserver:admin@1.2.3.4"
fn resolve_servers(
    input: &str,
    registry: &std::collections::HashMap<String, Server>,
) -> Result<Vec<Server>> {
    let mut servers = Vec::new();

    for name in input.split(',') {
        let name = name.trim();

        // First try registry lookup by name
        if let Some(server) = registry.get(name) {
            servers.push(server.clone());
        } else if name.contains('@') || name.contains(':') {
            // Looks like a connection string - parse it directly
            servers.push(Server::parse(name)?);
        } else {
            return Err(anyhow::anyhow!(
                "Unknown server '{}'. Run 'updatectl list servers' to see available servers.",
                name
            ));
        }
    }

    Ok(servers)
}

/// Print configured servers
fn print_servers(registry: &std::collections::HashMap<String, Server>) {
    if registry.is_empty() {
        println!("No servers configured in UPDATE_SERVERS.");
        println!();
        println!("Set UPDATE_SERVERS in your .env file:");
        println!("  UPDATE_SERVERS=server1:user@host1,server2:user@host2");
        return;
    }

    println!("Configured servers ({}):", registry.len());
    println!();

    let mut servers: Vec<_> = registry.values().collect();
    servers.sort_by(|a, b| a.name.cmp(&b.name));

    for server in &servers {
        println!("  {} → {}", server.name, server.display_host());
    }

    println!();
    println!("Usage:");
    println!("  updatectl os --servers \"{}\"", servers[0].name);
    if servers.len() > 1 {
        println!("  updatectl all --servers \"{},{}\"", servers[0].name, servers[1].name);
    }
}

/// Print usage examples
fn print_examples() {
    println!("Common usage examples:");
    println!();
    println!("List available servers:");
    println!("  updatectl list servers");
    println!();
    println!("Preview changes (dry-run):");
    println!("  updatectl all --dry-run --local");
    println!("  updatectl os --dry-run --servers \"Cloud VM1\"");
    println!();
    println!("Update OS packages:");
    println!("  updatectl os --yes --local");
    println!("  updatectl os --yes --servers \"Cloud VM1,Cloud VM2\"");
    println!();
    println!("Update Docker images:");
    println!("  updatectl docker --all --yes --local");
    println!("  updatectl docker --all --yes --servers \"Cloud VM1\"");
    println!("  updatectl docker --images nginx:latest,redis:latest --yes --local");
    println!();
    println!("Update everything:");
    println!("  updatectl all --yes --local");
    println!("  updatectl all --yes --servers \"Cloud VM1\"");
    println!();
    println!("Server targeting:");
    println!("  --local                    Update localhost only");
    println!("  --servers \"name1,name2\"    Update specific servers by name");
    println!("  (no flags)                 Update all servers from UPDATE_SERVERS");
    println!("  --local --servers \"name\"   Update both localhost AND named servers");
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
