use serde::{Deserialize, Serialize};

/// Complete cleanup report for a single server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    pub server: String,
    pub dangling_images: ImageStats,
    pub unused_images: ImageStats,
    pub unused_networks: NetworkStats,
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
            large_logs: LogStats::default(),
            volumes: VolumeStats::default(),
            total_reclaimable_bytes: 0,
        }
    }

    /// Calculate total reclaimable space (dangling images + unused networks)
    /// Excludes unused images (need confirmation) and logs/volumes (manual action)
    pub fn calculate_reclaimable(&mut self) {
        self.total_reclaimable_bytes = self.dangling_images.total_size_bytes;
        // Networks don't have size, but are reclaimable
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
