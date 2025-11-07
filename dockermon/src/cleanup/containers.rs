use crate::cleanup::types::{ContainerStats, ContainerInfo};
use anyhow::Result;
use bollard::Docker;
use bollard::container::{ListContainersOptions, PruneContainersOptions};

/// Analyze stopped containers
pub async fn analyze_stopped_containers(docker: &Docker) -> Result<ContainerStats> {
    // Get all containers (including stopped)
    let list_opts = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };

    let containers = docker.list_containers(Some(list_opts)).await?;

    let mut stats = ContainerStats::default();

    // Get age threshold from env (default 30 days for stopped containers)
    let stopped_age_threshold_days = std::env::var("DOCKERMON_CLEANUP_STOPPED_AGE_DAYS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    for container in containers {
        // Only count stopped containers
        let state = container.state.as_deref().unwrap_or("");
        if state == "running" {
            continue;
        }

        let created = container.created.unwrap_or(0);
        let age_days = (now - created) / 86400;

        // Only flag containers stopped for longer than threshold
        if age_days < stopped_age_threshold_days {
            continue;
        }

        let id = container.id.clone().unwrap_or_default();
        let size = container.size_rw.unwrap_or(0).max(0) as u64;

        stats.count += 1;
        stats.total_size_bytes += size;

        let name = container
            .names
            .and_then(|names| names.first().map(|n| n.trim_start_matches('/').to_string()))
            .unwrap_or_else(|| id[..12].to_string());

        stats.items.push(ContainerInfo {
            id: id.clone(),
            name,
            image: container.image.unwrap_or_else(|| "unknown".to_string()),
            size_bytes: size,
            created_timestamp: created,
            stopped_timestamp: None, // Would need inspect to get exact stop time
            exit_code: None, // Would need inspect
            status: container.status.unwrap_or_else(|| state.to_string()),
        });
    }

    // Sort by size descending
    stats.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(stats)
}

/// Prune stopped containers
pub async fn prune_stopped_containers(docker: &Docker) -> Result<PruneStats> {
    let result = docker
        .prune_containers(None::<PruneContainersOptions<String>>)
        .await?;

    let containers_deleted = result.containers_deleted.map(|v| v.len()).unwrap_or(0);
    let space_reclaimed = result.space_reclaimed.unwrap_or(0).max(0) as u64;

    Ok(PruneStats {
        count: containers_deleted,
        space_reclaimed,
    })
}

#[derive(Debug)]
pub struct PruneStats {
    pub count: usize,
    pub space_reclaimed: u64,
}
