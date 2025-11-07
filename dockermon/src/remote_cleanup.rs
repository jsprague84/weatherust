use crate::cleanup::{
    CleanupReport, ImageStats, ImageInfo, NetworkStats, NetworkInfo,
    BuildCacheStats, BuildCacheItem, ContainerStats, ContainerInfo,
    LogStats, VolumeStats
};
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

    // Analyze build cache
    report.build_cache = analyze_build_cache_remote(executor).await?;

    // Analyze stopped containers
    report.stopped_containers = analyze_stopped_containers_remote(executor).await?;

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
    let output = executor.execute_command(
        "/usr/bin/docker",
        &["image", "ls", "--filter", "dangling=true", "--format", "{{json .}}"]
    ).await?;

    let mut stats = ImageStats::default();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON, with better error context
        let image: Value = serde_json::from_str(trimmed)
            .map_err(|e| anyhow::anyhow!("Failed to parse Docker JSON output: '{}' - Error: {}", trimmed, e))?;
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
    let output = executor.execute_command(
        "/usr/bin/docker",
        &["network", "ls", "--format", "{{json .}}"]
    ).await?;

    let mut stats = NetworkStats::default();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let network: Value = serde_json::from_str(trimmed)
            .map_err(|e| anyhow::anyhow!("Failed to parse Docker network JSON: '{}' - Error: {}", trimmed, e))?;
        let name = network["Name"].as_str().unwrap_or("").to_string();

        // Skip default networks
        if name == "bridge" || name == "host" || name == "none" {
            continue;
        }

        // Check if network has containers (requires inspect)
        let containers_json = executor.execute_command(
            "/usr/bin/docker",
            &["network", "inspect", &name, "--format", "{{json .Containers}}"]
        ).await.unwrap_or_else(|_| "{}".to_string());

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

async fn analyze_build_cache_remote(executor: &RemoteExecutor) -> Result<BuildCacheStats> {
    // Use docker system df to get build cache info
    let output = executor.execute_command(
        "/usr/bin/docker",
        &["system", "df", "-v", "--format", "{{json .}}"]
    ).await?;

    let mut stats = BuildCacheStats::default();

    // Docker system df -v output includes BuildCache section
    // Parse the JSON to extract build cache information
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(system_df) = serde_json::from_str::<Value>(trimmed) {
            if let Some(build_cache) = system_df.get("BuildCache").and_then(|v| v.as_array()) {
                for cache_item in build_cache {
                    let size_str = cache_item.get("Size").and_then(|v| v.as_str()).unwrap_or("0B");
                    let size_bytes = parse_docker_size(size_str);
                    let in_use = cache_item.get("InUse").and_then(|v| v.as_bool()).unwrap_or(false);

                    stats.total_size_bytes += size_bytes;
                    if !in_use {
                        stats.reclaimable_bytes += size_bytes;
                    }

                    stats.items.push(BuildCacheItem {
                        id: cache_item.get("ID").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        cache_type: cache_item.get("Type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        size_bytes,
                        created_timestamp: 0,  // Not easily available via CLI
                        last_used_timestamp: None,
                        in_use,
                        shared: cache_item.get("Shared").and_then(|v| v.as_bool()).unwrap_or(false),
                    });
                }
            }
        }
    }

    Ok(stats)
}

async fn analyze_stopped_containers_remote(executor: &RemoteExecutor) -> Result<ContainerStats> {
    // List all containers (including stopped)
    let output = executor.execute_command(
        "/usr/bin/docker",
        &["ps", "-a", "--format", "{{json .}}"]
    ).await?;

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

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let container: Value = serde_json::from_str(trimmed)
            .map_err(|e| anyhow::anyhow!("Failed to parse Docker container JSON: '{}' - Error: {}", trimmed, e))?;

        let state = container.get("State").and_then(|v| v.as_str()).unwrap_or("");

        // Only count stopped containers
        if state == "running" {
            continue;
        }

        // Parse created timestamp from "CreatedAt" field (e.g., "2024-01-15 10:30:45 +0000 UTC")
        let created_str = container.get("CreatedAt").and_then(|v| v.as_str()).unwrap_or("");
        let created = parse_docker_timestamp(created_str);
        let age_days = (now - created) / 86400;

        // Only flag containers stopped for longer than threshold
        if age_days < stopped_age_threshold_days {
            continue;
        }

        let id = container.get("ID").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let size_str = container.get("Size").and_then(|v| v.as_str()).unwrap_or("0B");
        let size_bytes = parse_docker_size(size_str);

        stats.count += 1;
        stats.total_size_bytes += size_bytes;

        let name = container.get("Names").and_then(|v| v.as_str()).unwrap_or(&id[..12]).to_string();

        stats.items.push(ContainerInfo {
            id: id.clone(),
            name,
            image: container.get("Image").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            size_bytes,
            created_timestamp: created,
            stopped_timestamp: None,
            exit_code: None,
            status: container.get("Status").and_then(|v| v.as_str()).unwrap_or(state).to_string(),
        });
    }

    // Sort by size descending
    stats.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(stats)
}

/// Parse Docker timestamp format (e.g., "2024-01-15 10:30:45 +0000 UTC")
fn parse_docker_timestamp(timestamp_str: &str) -> i64 {
    // Try to parse various Docker timestamp formats
    // This is a simplified parser - may not handle all edge cases
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&timestamp_str[..19], "%Y-%m-%d %H:%M:%S") {
        dt.and_utc().timestamp()
    } else {
        0
    }
}
