use crate::cleanup::types::{LogInfo, LogStats};
use anyhow::Result;
use bollard::Docker;
use std::path::Path;

/// Analyze container log sizes
pub async fn analyze_large_logs(docker: &Docker) -> Result<LogStats> {
    let containers = docker
        .list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;

    let mut stats = LogStats::default();

    // Get threshold from env (default 100MB)
    let threshold_bytes = parse_size_threshold(
        &std::env::var("DOCKERMON_CLEANUP_LOG_SIZE_CONTAINER").unwrap_or_else(|_| "100M".to_string()),
    )
    .unwrap_or(100 * 1024 * 1024);

    for container in containers {
        let id = container.id.clone().unwrap_or_default();
        let name = container
            .names
            .as_ref()
            .and_then(|v| v.first())
            .map(|s| s.trim_start_matches('/').to_string())
            .unwrap_or_else(|| id.chars().take(12).collect());

        // Get container details to find log path
        let inspect = match docker.inspect_container(&id, None).await {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Check if log rotation is configured
        let has_rotation = inspect
            .host_config
            .as_ref()
            .and_then(|hc| hc.log_config.as_ref())
            .and_then(|lc| lc.config.as_ref())
            .map(|config| config.contains_key("max-size") || config.contains_key("max-file"))
            .unwrap_or(false);

        // Get log path
        let log_path = inspect.log_path.unwrap_or_default();
        if log_path.is_empty() {
            continue;
        }

        // Get log file size
        let log_size = match get_file_size(&log_path) {
            Ok(size) => size,
            Err(_) => continue,
        };

        stats.total_size_bytes += log_size;

        // Only include in report if over threshold
        if log_size >= threshold_bytes {
            stats.containers_over_threshold += 1;
            stats.items.push(LogInfo {
                container_name: name,
                container_id: id,
                log_size_bytes: log_size,
                has_rotation,
            });
        }
    }

    // Sort by size descending
    stats.items.sort_by(|a, b| b.log_size_bytes.cmp(&a.log_size_bytes));

    Ok(stats)
}

/// Get file size in bytes
fn get_file_size(path: &str) -> Result<u64> {
    let metadata = std::fs::metadata(Path::new(path))?;
    Ok(metadata.len())
}

/// Parse size string like "100M", "1G", "500K" to bytes
fn parse_size_threshold(s: &str) -> Result<u64> {
    let s = s.trim().to_uppercase();
    let (num_str, suffix) = if s.ends_with('G') {
        (&s[..s.len() - 1], 1024 * 1024 * 1024)
    } else if s.ends_with('M') {
        (&s[..s.len() - 1], 1024 * 1024)
    } else if s.ends_with('K') {
        (&s[..s.len() - 1], 1024)
    } else {
        (s.as_str(), 1)
    };

    let num: u64 = num_str.parse()?;
    Ok(num * suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_threshold() {
        assert_eq!(parse_size_threshold("100").unwrap(), 100);
        assert_eq!(parse_size_threshold("100K").unwrap(), 100 * 1024);
        assert_eq!(parse_size_threshold("100M").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size_threshold("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size_threshold("100m").unwrap(), 100 * 1024 * 1024);
    }
}
