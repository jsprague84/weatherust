use crate::cleanup::types::{NetworkInfo, NetworkStats};
use anyhow::Result;
use bollard::Docker;
use bollard::network::{ListNetworksOptions, PruneNetworksOptions};

/// Analyze unused Docker networks
pub async fn analyze_unused_networks(docker: &Docker) -> Result<NetworkStats> {
    // Get all networks
    let networks = docker.list_networks(None::<ListNetworksOptions<String>>).await?;

    let mut stats = NetworkStats::default();

    for network in networks {
        // Skip default networks (bridge, host, none)
        let name = network.name.clone().unwrap_or_default();
        if name == "bridge" || name == "host" || name == "none" {
            continue;
        }

        // Check if network is in use
        let containers = network.containers.unwrap_or_default();
        if !containers.is_empty() {
            continue; // Network is in use, skip
        }

        stats.count += 1;
        stats.items.push(NetworkInfo {
            id: network.id.clone().unwrap_or_default(),
            name,
            driver: network.driver.unwrap_or_else(|| "unknown".to_string()),
            created_timestamp: network
                .created
                .and_then(|c| chrono::DateTime::parse_from_rfc3339(&c).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
        });
    }

    Ok(stats)
}

/// Prune unused networks
pub async fn prune_unused_networks(docker: &Docker) -> Result<usize> {
    let result = docker
        .prune_networks(None::<PruneNetworksOptions<String>>)
        .await?;

    let count = result.networks_deleted.map(|v| v.len()).unwrap_or(0);

    Ok(count)
}
