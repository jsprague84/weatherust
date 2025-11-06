use crate::cleanup::{CleanupReport, ImageStats, ImageInfo, NetworkStats, NetworkInfo, LogStats, VolumeStats};
use crate::executor::RemoteExecutor;
use anyhow::Result;
use serde_json::Value;

/// Analyze cleanup opportunities on a remote server via SSH using Docker CLI
pub async fn analyze_cleanup_remote(
    executor: &RemoteExecutor,
    server_name: &str,
) -> Result<CleanupReport> {
    let mut report = CleanupReport::new(server_name.to_string());

    // Analyze dangling images
    report.dangling_images = analyze_dangling_images_remote(executor).await?;

    // Analyze unused images
    report.unused_images = analyze_unused_images_remote(executor).await?;

    // Analyze unused networks
    report.unused_networks = analyze_unused_networks_remote(executor).await?;

    // Note: Large logs and volumes analysis requires more complex logic
    // For now, set to default (empty)
    report.large_logs = LogStats::default();
    report.volumes = VolumeStats::default();

    // Calculate total reclaimable
    report.calculate_reclaimable();

    Ok(report)
}

async fn analyze_dangling_images_remote(executor: &RemoteExecutor) -> Result<ImageStats> {
    // List dangling images using Docker CLI
    let output = executor.execute("docker image ls --filter dangling=true --format '{{json .}}'")?;

    let mut stats = ImageStats::default();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let image: Value = serde_json::from_str(line)?;
        let size_str = image["Size"].as_str().unwrap_or("0B");
        let size_bytes = parse_docker_size(size_str);

        stats.count += 1;
        stats.total_size_bytes += size_bytes;

        stats.items.push(ImageInfo {
            repository: "<none>".to_string(),
            tag: "<none>".to_string(),
            image_id: image["ID"].as_str().unwrap_or("").to_string(),
            size_bytes,
            created_timestamp: 0, // Docker CLI doesn't provide epoch timestamp easily
        });
    }

    // Sort by size descending
    stats.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(stats)
}

async fn analyze_unused_images_remote(executor: &RemoteExecutor) -> Result<ImageStats> {
    // This is complex - would need to list all images and containers
    // For now, return empty (can be enhanced later)
    Ok(ImageStats::default())
}

async fn analyze_unused_networks_remote(executor: &RemoteExecutor) -> Result<NetworkStats> {
    let output = executor.execute("docker network ls --format '{{json .}}'")?;

    let mut stats = NetworkStats::default();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let network: Value = serde_json::from_str(line)?;
        let name = network["Name"].as_str().unwrap_or("").to_string();

        // Skip default networks
        if name == "bridge" || name == "host" || name == "none" {
            continue;
        }

        // Check if network has containers (requires inspect)
        let inspect_cmd = format!("docker network inspect {} --format '{{{{json .Containers}}}}'", name);
        let containers_json = executor.execute(&inspect_cmd).unwrap_or_else(|_| "{}".to_string());

        // If containers is empty object, network is unused
        if containers_json.trim() == "{}" || containers_json.trim() == "null" {
            stats.count += 1;
            stats.items.push(NetworkInfo {
                id: network["ID"].as_str().unwrap_or("").to_string(),
                name: name.clone(),
                driver: network["Driver"].as_str().unwrap_or("bridge").to_string(),
                created_timestamp: 0,
            });
        }
    }

    Ok(stats)
}

/// Parse Docker CLI size format (e.g., "1.5GB", "250MB", "1.2kB")
fn parse_docker_size(size_str: &str) -> u64 {
    let size_str = size_str.trim().to_uppercase();

    // Extract number and unit
    let (num_str, multiplier) = if size_str.ends_with("GB") {
        (&size_str[..size_str.len()-2], 1024 * 1024 * 1024)
    } else if size_str.ends_with("MB") {
        (&size_str[..size_str.len()-2], 1024 * 1024)
    } else if size_str.ends_with("KB") {
        (&size_str[..size_str.len()-2], 1024)
    } else if size_str.ends_with('B') {
        (&size_str[..size_str.len()-1], 1)
    } else {
        (size_str.as_str(), 1)
    };

    // Parse the number (may be float like "1.5")
    if let Ok(num) = num_str.parse::<f64>() {
        (num * multiplier as f64) as u64
    } else {
        0
    }
}
