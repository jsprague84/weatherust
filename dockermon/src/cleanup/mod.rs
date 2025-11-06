mod types;
mod images;
mod networks;
mod logs;
mod volumes;

pub use types::{CleanupReport, format_bytes};

use bollard::Docker;
use anyhow::Result;

/// Analyze Docker resources and generate cleanup report
pub async fn analyze_cleanup(docker: &Docker) -> Result<CleanupReport> {
    let mut report = CleanupReport::new("local".to_string());

    // Analyze images (dangling and unused)
    report.dangling_images = images::analyze_dangling_images(docker).await?;
    report.unused_images = images::analyze_unused_images(docker).await?;

    // Analyze networks
    report.unused_networks = networks::analyze_unused_networks(docker).await?;

    // Analyze container logs
    report.large_logs = logs::analyze_large_logs(docker).await?;

    // Analyze volumes (informational only)
    report.volumes = volumes::analyze_volumes(docker).await?;

    // Calculate total reclaimable space
    report.calculate_reclaimable();

    Ok(report)
}

/// Execute safe cleanup operations (dangling images + unused networks)
pub async fn execute_safe_cleanup(docker: &Docker) -> Result<CleanupResult> {
    let mut result = CleanupResult::default();

    // Prune dangling images
    match images::prune_dangling_images(docker).await {
        Ok(stats) => {
            result.dangling_images_removed = stats.count;
            result.space_reclaimed_bytes += stats.space_reclaimed;
        }
        Err(e) => result.errors.push(format!("Failed to prune dangling images: {}", e)),
    }

    // Prune unused networks
    match networks::prune_unused_networks(docker).await {
        Ok(count) => {
            result.networks_removed = count;
        }
        Err(e) => result.errors.push(format!("Failed to prune networks: {}", e)),
    }

    Ok(result)
}

/// Execute unused image cleanup (requires confirmation)
pub async fn execute_unused_image_cleanup(docker: &Docker) -> Result<CleanupResult> {
    let mut result = CleanupResult::default();

    match images::prune_unused_images(docker).await {
        Ok(stats) => {
            result.unused_images_removed = stats.count;
            result.space_reclaimed_bytes += stats.space_reclaimed;
        }
        Err(e) => result.errors.push(format!("Failed to prune unused images: {}", e)),
    }

    Ok(result)
}

/// Result of cleanup execution
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub dangling_images_removed: usize,
    pub unused_images_removed: usize,
    pub networks_removed: usize,
    pub space_reclaimed_bytes: u64,
    pub errors: Vec<String>,
}

impl CleanupResult {
    pub fn format_summary(&self) -> String {
        let mut parts = Vec::new();

        if self.dangling_images_removed > 0 {
            parts.push(format!("Removed {} dangling images", self.dangling_images_removed));
        }

        if self.unused_images_removed > 0 {
            parts.push(format!("Removed {} unused images", self.unused_images_removed));
        }

        if self.networks_removed > 0 {
            parts.push(format!("Removed {} unused networks", self.networks_removed));
        }

        if self.space_reclaimed_bytes > 0 {
            parts.push(format!("Reclaimed {}", format_bytes(self.space_reclaimed_bytes)));
        }

        if !self.errors.is_empty() {
            parts.push(format!("{} errors", self.errors.len()));
        }

        if parts.is_empty() {
            "No cleanup performed".to_string()
        } else {
            parts.join(", ")
        }
    }
}
