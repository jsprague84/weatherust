use serde::{Deserialize, Serialize};

/// Complete cleanup report for a single server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    pub server: String,
    pub dangling_images: ImageStats,
    pub unused_images: ImageStats,
    pub unused_networks: NetworkStats,
    pub build_cache: BuildCacheStats,
    pub stopped_containers: ContainerStats,
    pub large_logs: LogStats,
    pub volumes: VolumeStats,
    pub total_reclaimable_bytes: u64,
}

impl CleanupReport {
    pub fn new(server: String) -> Self {
        Self {
            server,
            dangling_images: ImageStats::default(),
            unused_images: ImageStats::default(),
            unused_networks: NetworkStats::default(),
            build_cache: BuildCacheStats::default(),
            stopped_containers: ContainerStats::default(),
            large_logs: LogStats::default(),
            volumes: VolumeStats::default(),
            total_reclaimable_bytes: 0,
        }
    }

    /// Calculate total reclaimable space (safe to auto-cleanup)
    /// Includes: dangling images, build cache, stopped containers
    /// Excludes: unused images (need confirmation), unused networks (no size), logs/volumes (manual)
    pub fn calculate_reclaimable(&mut self) {
        self.total_reclaimable_bytes = self.dangling_images.total_size_bytes
            + self.build_cache.total_size_bytes
            + self.stopped_containers.total_size_bytes;
    }
}

/// Statistics about images
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageStats {
    pub count: usize,
    pub total_size_bytes: u64,
    pub items: Vec<ImageInfo>,
}

/// Information about a single Docker image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub repository: String,
    pub tag: String,
    pub image_id: String,
    pub size_bytes: u64,
    pub created_timestamp: i64,
}

impl ImageInfo {
    pub fn display_name(&self) -> String {
        if self.repository.is_empty() || self.repository == "<none>" {
            format!("<none> ({})", &self.image_id[..12])
        } else {
            format!("{}:{}", self.repository, self.tag)
        }
    }
}

/// Statistics about Docker networks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    pub count: usize,
    pub items: Vec<NetworkInfo>,
}

/// Information about a single Docker network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub created_timestamp: i64,
}

/// Statistics about container logs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogStats {
    pub total_size_bytes: u64,
    pub containers_over_threshold: usize,
    pub items: Vec<LogInfo>,
}

/// Information about a single container's logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogInfo {
    pub container_name: String,
    pub container_id: String,
    pub log_size_bytes: u64,
    pub has_rotation: bool,
}

/// Statistics about Docker volumes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeStats {
    pub count: usize,
    pub total_size_bytes: u64,
    pub items: Vec<VolumeInfo>,
}

/// Information about a single Docker volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mount_point: String,
    pub size_bytes: u64,
    pub created_timestamp: i64,
    pub containers_using: Vec<String>,
}

/// Statistics about Docker build cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildCacheStats {
    pub total_size_bytes: u64,
    pub reclaimable_bytes: u64,
    pub items: Vec<BuildCacheItem>,
}

/// Information about a build cache item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCacheItem {
    pub id: String,
    pub cache_type: String, // "regular", "internal", "frontend", "source"
    pub size_bytes: u64,
    pub created_timestamp: i64,
    pub last_used_timestamp: Option<i64>,
    pub in_use: bool,
    pub shared: bool,
}

/// Statistics about stopped containers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerStats {
    pub count: usize,
    pub total_size_bytes: u64,
    pub items: Vec<ContainerInfo>,
}

/// Information about a stopped container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub size_bytes: u64,
    pub created_timestamp: i64,
    pub stopped_timestamp: Option<i64>,
    pub exit_code: Option<i64>,
    pub status: String,
}

/// Format bytes as human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{}MB", bytes / MB)
    } else if bytes >= KB {
        format!("{}KB", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0B");
        assert_eq!(format_bytes(1023), "1023B");
        assert_eq!(format_bytes(1024), "1KB");
        assert_eq!(format_bytes(1024 * 1024), "1MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00GB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.50GB");
    }
}
