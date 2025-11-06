use crate::cleanup::types::{VolumeInfo, VolumeStats};
use anyhow::Result;
use bollard::Docker;
use bollard::volume::ListVolumesOptions;
use std::collections::HashMap;
use std::path::Path;

/// Analyze Docker volumes (informational only, no deletion)
pub async fn analyze_volumes(docker: &Docker) -> Result<VolumeStats> {
    let volumes_response = docker.list_volumes(None::<ListVolumesOptions<String>>).await?;

    let volumes = volumes_response.volumes.unwrap_or_default();

    // Get all containers to see which volumes are in use
    let containers = docker
        .list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;

    // Build map of volumes to containers using them
    let mut volume_usage: HashMap<String, Vec<String>> = HashMap::new();
    for container in containers {
        let container_name = container
            .names
            .as_ref()
            .and_then(|v| v.first())
            .map(|s| s.trim_start_matches('/').to_string())
            .unwrap_or_default();

        if let Some(mounts) = container.mounts {
            for mount in mounts {
                if let Some(name) = mount.name {
                    volume_usage
                        .entry(name)
                        .or_insert_with(Vec::new)
                        .push(container_name.clone());
                }
            }
        }
    }

    let mut stats = VolumeStats::default();
    stats.count = volumes.len();

    for volume in volumes {
        let name = volume.name;
        let mount_point = volume.mountpoint;

        // Try to get volume size (best effort)
        let size_bytes = get_directory_size(&mount_point).unwrap_or(0);
        stats.total_size_bytes += size_bytes;

        let containers_using = volume_usage.get(&name).cloned().unwrap_or_default();

        stats.items.push(VolumeInfo {
            name: name.clone(),
            driver: volume.driver,
            mount_point,
            size_bytes,
            created_timestamp: volume
                .created_at
                .and_then(|c| chrono::DateTime::parse_from_rfc3339(&c).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
            containers_using,
        });
    }

    // Sort by size descending
    stats.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    // Keep only top 10 largest for reporting
    stats.items.truncate(10);

    Ok(stats)
}

/// Get total size of a directory (best effort, may fail on permission issues)
fn get_directory_size(path: &str) -> Result<u64> {
    let path = Path::new(path);

    if !path.exists() {
        return Ok(0);
    }

    let output = std::process::Command::new("du")
        .arg("-sb")
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Ok(0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let size_str = stdout.split_whitespace().next().unwrap_or("0");
    let size = size_str.parse::<u64>().unwrap_or(0);

    Ok(size)
}
