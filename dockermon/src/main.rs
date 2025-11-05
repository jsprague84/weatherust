use bollard::models::HealthStatusEnum;
use clap::Parser;
use common::{dotenv_init, http_client, send_gotify_dockermon, send_ntfy_dockermon};
use futures_util::StreamExt;
use std::collections::HashSet;
use std::env;
use tokio::time::{timeout, Duration};

#[derive(Parser, Debug)]
#[command(name = "dockermon")]
#[command(about = "Check Docker container health/usage and notify Gotify")]
struct Args {
    /// Suppress stdout; only send Gotify
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv_init();
    let args = Args::parse();

    let ignore_set = build_ignore_set(&args.ignore);

    // Allow a dockermon-specific Gotify token override
    if let Ok(tok) = std::env::var("DOCKERMON_GOTIFY_KEY") {
        if !tok.trim().is_empty() {
            std::env::set_var("GOTIFY_KEY", tok);
        }
    }

    // Resolve thresholds and flags from env with CLI overrides
    let cpu_warn = args.cpu_warn_pct.or_else(|| env_var_f64("CPU_WARN_PCT"));
    let mem_warn = args.mem_warn_pct.or_else(|| env_var_f64("MEM_WARN_PCT"));
    let notify_always = if args.notify_always {
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
    if !args.quiet {
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
