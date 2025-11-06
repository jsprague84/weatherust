use bollard::models::HealthStatusEnum;
use clap::{Parser, Subcommand};
use common::{dotenv_init, http_client, send_gotify_dockermon, send_ntfy_dockermon, NtfyAction};
use futures_util::StreamExt;
use std::collections::HashSet;
use std::env;
use tokio::time::{timeout, Duration};

mod cleanup;

#[derive(Parser, Debug)]
#[command(name = "dockermon")]
#[command(about = "Docker monitoring and cleanup tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check Docker container health and notify
    Health {
        /// Suppress stdout; only send notifications
        #[arg(long, default_value_t = false)]
        quiet: bool,

        /// CPU warn threshold in percent (overrides env CPU_WARN_PCT)
        #[arg(long)]
        cpu_warn_pct: Option<f64>,

        /// Memory warn threshold in percent (overrides env MEM_WARN_PCT)
        #[arg(long)]
        mem_warn_pct: Option<f64>,

        /// Always notify, even when everything is OK (overrides env HEALTH_NOTIFY_ALWAYS)
        #[arg(long, default_value_t = false)]
        notify_always: bool,

        /// Ignore containers by name/id/service (comma-separated or repeated)
        #[arg(long, value_name = "NAME", value_delimiter = ',')]
        ignore: Vec<String>,
    },
    /// Analyze Docker resources and report cleanup opportunities
    Cleanup {
        /// Suppress stdout; only send notifications
        #[arg(long, default_value_t = false)]
        quiet: bool,

        /// Execute safe cleanup (dangling images + unused networks)
        #[arg(long, default_value_t = false)]
        execute_safe: bool,

        /// Execute unused image cleanup (requires explicit flag)
        #[arg(long, default_value_t = false)]
        prune_unused_images: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv_init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Health {
            quiet,
            cpu_warn_pct,
            mem_warn_pct,
            notify_always,
            ignore,
        } => {
            run_health_check(quiet, cpu_warn_pct, mem_warn_pct, notify_always, ignore).await
        }
        Commands::Cleanup {
            quiet,
            execute_safe,
            prune_unused_images,
        } => run_cleanup(quiet, execute_safe, prune_unused_images).await,
    }
}

async fn run_health_check(
    quiet: bool,
    cpu_warn_pct: Option<f64>,
    mem_warn_pct: Option<f64>,
    notify_always: bool,
    ignore: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ignore_set = build_ignore_set(&ignore);

    // Allow a dockermon-specific Gotify token override
    if let Ok(tok) = std::env::var("DOCKERMON_GOTIFY_KEY") {
        if !tok.trim().is_empty() {
            std::env::set_var("GOTIFY_KEY", tok);
        }
    }

    // Resolve thresholds and flags from env with CLI overrides
    let cpu_warn = cpu_warn_pct.or_else(|| env_var_f64("CPU_WARN_PCT"));
    let mem_warn = mem_warn_pct.or_else(|| env_var_f64("MEM_WARN_PCT"));
    let notify_always = if notify_always {
        true
    } else {
        env::var("HEALTH_NOTIFY_ALWAYS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    };

    // Connect to Docker via Unix socket
    let docker = bollard::Docker::connect_with_unix_defaults()?;

    // List containers
    let containers = docker
        .list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;

    // Inspect and sample stats for each container (best-effort)
    let mut issues: Vec<String> = Vec::new();
    let mut ok_count = 0usize;

    for c in containers {
        let id = c.id.unwrap_or_default();
        let name = c
            .names
            .as_ref()
            .and_then(|v| v.get(0))
            .map(|s| s.trim_start_matches('/').to_string())
            .unwrap_or_else(|| id.chars().take(12).collect());
        let short_id: String = id.chars().take(12).collect();
        let service_label = c
            .labels
            .as_ref()
            .and_then(|labels| labels.get("com.docker.compose.service"))
            .map(|s| s.to_string());

        if should_ignore(&ignore_set, &name, &id, &short_id, service_label.as_deref()) {
            continue;
        }

        // Inspect for state/health
        let inspect = docker.inspect_container(&id, None).await?;
        let (running, health_status) = match inspect.state {
            Some(state) => {
                let running = state.running.unwrap_or(false);
                let hs = match state.health.and_then(|h| h.status) {
                    Some(HealthStatusEnum::HEALTHY) => "healthy",
                    Some(HealthStatusEnum::UNHEALTHY) => "unhealthy",
                    Some(HealthStatusEnum::STARTING) => "starting",
                    Some(HealthStatusEnum::NONE) => "none",
                    Some(_) | None => "none",
                };
                (running, hs.to_string())
            }
            None => (false, "none".to_string()),
        };

        // Sample a single stats frame with a short timeout
        let (cpu_pct, mem_pct) = match sample_stats_once(&docker, &id).await {
            Ok(v) => v,
            Err(_) => (None, None),
        };

        // Determine if this container is problematic
        let mut bad = false;
        let mut reasons: Vec<String> = Vec::new();

        if !running {
            bad = true;
            reasons.push("not running".to_string());
        }
        if !health_status.eq_ignore_ascii_case("healthy")
            && !health_status.eq_ignore_ascii_case("none")
        {
            bad = true;
            reasons.push(format!("health: {}", health_status));
        }
        if let (Some(th), Some(val)) = (cpu_warn, cpu_pct) {
            if val > th {
                bad = true;
                reasons.push(format!("cpu: {:.1}% > {:.0}%", val, th));
            }
        }
        if let (Some(th), Some(val)) = (mem_warn, mem_pct) {
            if val > th {
                bad = true;
                reasons.push(format!("mem: {:.1}% > {:.0}%", val, th));
            }
        }

        if bad {
            let mut parts = vec![format!("{} ({})", name, short_id)];
            if let Some(v) = cpu_pct {
                parts.push(format!("CPU {:.1}%", v));
            }
            if let Some(v) = mem_pct {
                parts.push(format!("MEM {:.1}%", v));
            }
            parts.push(format!(
                "state: {}",
                if running { "running" } else { "exited" }
            ));
            if !health_status.is_empty() && health_status != "none" {
                parts.push(format!("health: {}", health_status));
            }
            issues.push(parts.join(" | "));
        } else {
            ok_count += 1;
        }
    }

    // Build output
    let mut lines = Vec::new();
    let had_issues = !issues.is_empty();
    let title;
    if !had_issues {
        title = "Docker Health: OK";
        lines.push(format!("All containers OK ({} checked)", ok_count));
    } else {
        title = "Docker Health: Issues";
        lines.push(format!("{} issue(s) detected", issues.len()));
        lines.extend(issues.iter().cloned());
    }

    let body = lines.join("\n");
    if !quiet {
        println!("{}\n{}", title, body);
    }

    if notify_always || had_issues {
        let client = http_client();
        // Send to Gotify (if configured)
        if let Err(e) = send_gotify_dockermon(&client, title, &body).await {
            eprintln!("Gotify send error: {e}");
        }
        // Send to ntfy.sh (if configured)
        if let Err(e) = send_ntfy_dockermon(&client, title, &body, None).await {
            eprintln!("ntfy send error: {e}");
        }
    }

    Ok(())
}

async fn run_cleanup(
    quiet: bool,
    execute_safe: bool,
    prune_unused_images: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Allow a dockermon-specific Gotify token override
    if let Ok(tok) = std::env::var("DOCKERMON_GOTIFY_KEY") {
        if !tok.trim().is_empty() {
            std::env::set_var("GOTIFY_KEY", tok);
        }
    }

    // Connect to Docker via Unix socket
    let docker = bollard::Docker::connect_with_unix_defaults()?;

    // Analyze cleanup opportunities
    let report = cleanup::analyze_cleanup(&docker).await?;

    // Execute cleanup if requested
    let mut execution_summary = Vec::new();

    if execute_safe {
        let result = cleanup::execute_safe_cleanup(&docker).await?;
        execution_summary.push(format!(
            "Safe cleanup: {} dangling images ({}) + {} unused networks removed",
            result.dangling_images_removed,
            cleanup::format_bytes(result.space_reclaimed_bytes),
            result.networks_removed
        ));
    }

    if prune_unused_images {
        let result = cleanup::execute_unused_image_cleanup(&docker).await?;
        execution_summary.push(format!(
            "Unused images: {} removed ({})",
            result.unused_images_removed,
            cleanup::format_bytes(result.space_reclaimed_bytes)
        ));
    }

    // Format report
    let title = if execution_summary.is_empty() {
        "Docker Cleanup: Analysis"
    } else {
        "Docker Cleanup: Complete"
    };

    let mut lines = Vec::new();
    let cleanup_was_executed = !execution_summary.is_empty();

    // Add execution summary if any cleanup was performed
    if cleanup_was_executed {
        lines.push("=== Cleanup Actions ===".to_string());
        lines.extend(execution_summary);
        lines.push("".to_string());
    }

    // Add analysis report
    lines.push("=== Analysis Report ===".to_string());
    lines.push(format!(
        "Total reclaimable: {}",
        cleanup::format_bytes(report.total_reclaimable_bytes)
    ));
    lines.push("".to_string());

    // Dangling images
    if report.dangling_images.count > 0 {
        lines.push(format!(
            "Dangling Images: {} ({})",
            report.dangling_images.count,
            cleanup::format_bytes(report.dangling_images.total_size_bytes)
        ));
        for item in report.dangling_images.items.iter().take(5) {
            lines.push(format!("  • {} ({})", item.image_id, cleanup::format_bytes(item.size_bytes)));
        }
        lines.push("".to_string());
    }

    // Unused images
    if report.unused_images.count > 0 {
        lines.push(format!(
            "Unused Images: {} ({})",
            report.unused_images.count,
            cleanup::format_bytes(report.unused_images.total_size_bytes)
        ));
        for item in report.unused_images.items.iter().take(5) {
            lines.push(format!(
                "  • {}:{} ({})",
                item.repository,
                item.tag,
                cleanup::format_bytes(item.size_bytes)
            ));
        }
        lines.push("".to_string());
    }

    // Unused networks
    if report.unused_networks.count > 0 {
        lines.push(format!("Unused Networks: {}", report.unused_networks.count));
        for item in report.unused_networks.items.iter().take(5) {
            lines.push(format!("  • {} ({})", item.name, item.driver));
        }
        lines.push("".to_string());
    }

    // Large logs
    if report.large_logs.containers_over_threshold > 0 {
        lines.push(format!(
            "Large Logs: {} containers (total {})",
            report.large_logs.containers_over_threshold,
            cleanup::format_bytes(report.large_logs.total_size_bytes)
        ));
        for item in report.large_logs.items.iter().take(5) {
            let rotation_status = if item.has_rotation { "rotated" } else { "NO ROTATION" };
            lines.push(format!(
                "  • {} ({}, {})",
                item.container_name,
                cleanup::format_bytes(item.log_size_bytes),
                rotation_status
            ));
        }
        lines.push("".to_string());
    }

    // Volumes (info only)
    if report.volumes.count > 0 {
        lines.push(format!(
            "Volumes: {} (total {})",
            report.volumes.count,
            cleanup::format_bytes(report.volumes.total_size_bytes)
        ));
        for item in report.volumes.items.iter().take(5) {
            let usage = if item.containers_using.is_empty() {
                "UNUSED".to_string()
            } else {
                format!("used by {}", item.containers_using.join(", "))
            };
            lines.push(format!(
                "  • {} ({}, {})",
                item.name,
                cleanup::format_bytes(item.size_bytes),
                usage
            ));
        }
        lines.push("".to_string());
    }

    let body = lines.join("\n");
    if !quiet {
        println!("{}\n{}", title, body);
    }

    // Prepare ntfy actions (only if no cleanup was executed)
    let ntfy_actions = if !cleanup_was_executed && report.total_reclaimable_bytes > 0 {
        // Get webhook base URL from env
        let webhook_url = env::var("UPDATECTL_WEBHOOK_URL")
            .unwrap_or_else(|_| "http://localhost:8080/webhook".to_string());
        let webhook_secret = env::var("UPDATECTL_WEBHOOK_SECRET")
            .unwrap_or_else(|_| "your_secret_token".to_string());

        let mut actions = Vec::new();

        // Add safe cleanup button if there are dangling images or unused networks
        if report.dangling_images.count > 0 || report.unused_networks.count > 0 {
            actions.push(
                NtfyAction::http_post(
                    "Safe Cleanup",
                    &format!("{}/cleanup/safe", webhook_url)
                )
                .with_headers(serde_json::json!({
                    "Authorization": format!("Bearer {}", webhook_secret)
                }))
            );
        }

        // Add unused images cleanup button if there are unused images (dangerous operation)
        if report.unused_images.count > 0 {
            actions.push(
                NtfyAction::http_post(
                    "Prune Unused Images",
                    &format!("{}/cleanup/images/prune-unused", webhook_url)
                )
                .with_headers(serde_json::json!({
                    "Authorization": format!("Bearer {}", webhook_secret)
                }))
            );
        }

        // Limit to 3 actions (ntfy.sh self-hosted limit)
        actions.truncate(3);
        Some(actions)
    } else {
        None
    };

    // Always notify for cleanup operations
    let client = http_client();

    // Send to Gotify (if configured) - full report
    if let Err(e) = send_gotify_dockermon(&client, title, &body).await {
        eprintln!("Gotify send error: {e}");
    }

    // Send to ntfy.sh (if configured) - with action buttons
    if let Err(e) = send_ntfy_dockermon(&client, title, &body, ntfy_actions).await {
        eprintln!("ntfy send error: {e}");
    }

    Ok(())
}

fn env_var_f64(key: &str) -> Option<f64> {
    env::var(key).ok().and_then(|v| v.parse::<f64>().ok())
}

fn build_ignore_set(cli_patterns: &[String]) -> HashSet<String> {
    let mut set = HashSet::new();
    for item in cli_patterns {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_lowercase());
        }
    }
    if let Ok(raw) = env::var("DOCKERMON_IGNORE") {
        for entry in raw.split(|c: char| c == ',' || c == '\n' || c.is_whitespace()) {
            let trimmed = entry.trim();
            if !trimmed.is_empty() {
                set.insert(trimmed.to_lowercase());
            }
        }
    }
    set
}

fn should_ignore(
    ignore: &HashSet<String>,
    name: &str,
    id: &str,
    short_id: &str,
    service: Option<&str>,
) -> bool {
    if ignore.is_empty() {
        return false;
    }

    let identifiers = [
        Some(name),
        Some(id),
        if short_id.is_empty() {
            None
        } else {
            Some(short_id)
        },
        service,
    ];

    identifiers.iter().flatten().any(|value| {
        let v = value.trim();
        !v.is_empty() && ignore.contains(&v.to_lowercase())
    })
}

async fn sample_stats_once(
    docker: &bollard::Docker,
    id: &str,
) -> Result<(Option<f64>, Option<f64>), Box<dyn std::error::Error>> {
    use bollard::container::StatsOptions;
    let mut stream = docker.stats(
        id,
        Some(StatsOptions {
            stream: false,
            one_shot: true,
        }),
    );
    let next_opt = timeout(Duration::from_secs(2), stream.next()).await?;
    let stats = match next_opt {
        Some(res) => res?,
        None => return Ok((None, None)),
    };

    // CPU% calculation per Docker docs (may be None if precpu/system not available)
    let cpu_stats = &stats.cpu_stats;
    let total = cpu_stats.cpu_usage.total_usage as f64; // u64 -> f64
    let system_opt = cpu_stats.system_cpu_usage; // Option<u64>
    let pre_total = stats.precpu_stats.cpu_usage.total_usage as f64; // u64 -> f64
    let pre_system_opt = stats.precpu_stats.system_cpu_usage; // Option<u64>
    let cpu_pct: Option<f64> = match (system_opt, pre_system_opt) {
        (Some(system), Some(pre_system))
            if total > pre_total && (system as f64) > pre_system as f64 =>
        {
            let cpu_delta = total - pre_total;
            let system_delta = system as f64 - pre_system as f64;
            if system_delta > 0.0 {
                let online_cpus = cpu_stats
                    .online_cpus
                    .or_else(|| {
                        cpu_stats
                            .cpu_usage
                            .percpu_usage
                            .as_ref()
                            .map(|v| v.len() as u64)
                    })
                    .unwrap_or(1) as f64;
                Some((cpu_delta / system_delta) * online_cpus * 100.0)
            } else {
                None
            }
        }
        _ => None,
    };

    // Memory%
    let mem_pct: Option<f64> = match (stats.memory_stats.usage, stats.memory_stats.limit) {
        (Some(usage), Some(limit)) if limit > 0 => Some((usage as f64 / limit as f64) * 100.0),
        _ => None,
    };

    Ok((cpu_pct, mem_pct))
}
