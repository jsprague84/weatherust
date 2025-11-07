use super::CleanupResult;
use anyhow::Result;
use bollard::Docker;

/// Cleanup profile - determines aggressiveness of cleanup operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupProfile {
    Conservative,
    Moderate,
    Aggressive,
}

impl CleanupProfile {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "conservative" => Some(CleanupProfile::Conservative),
            "moderate" => Some(CleanupProfile::Moderate),
            "aggressive" => Some(CleanupProfile::Aggressive),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            CleanupProfile::Conservative => "Safe operations only: dangling images, unused networks, build cache, stopped containers >30 days",
            CleanupProfile::Moderate => "Conservative + unused images >90 days, stopped containers >7 days",
            CleanupProfile::Aggressive => "Moderate + all unused images >30 days, all stopped containers",
        }
    }

    /// Get the age threshold for stopped containers (in days)
    pub fn stopped_container_age_days(&self) -> i64 {
        match self {
            CleanupProfile::Conservative => 30,
            CleanupProfile::Moderate => 7,
            CleanupProfile::Aggressive => 0, // All stopped containers
        }
    }

    /// Get the age threshold for unused images (in days)
    pub fn unused_image_age_days(&self) -> i64 {
        match self {
            CleanupProfile::Conservative => i64::MAX, // Don't remove unused images in conservative
            CleanupProfile::Moderate => 90,
            CleanupProfile::Aggressive => 30,
        }
    }

    /// Should we prune unused images?
    pub fn prune_unused_images(&self) -> bool {
        match self {
            CleanupProfile::Conservative => false,
            CleanupProfile::Moderate => true,
            CleanupProfile::Aggressive => true,
        }
    }
}

/// Execute cleanup based on profile
pub async fn execute_cleanup_with_profile(
    docker: &Docker,
    profile: CleanupProfile,
) -> Result<CleanupResult> {
    // Temporarily set age thresholds based on profile
    let original_container_age = std::env::var("DOCKERMON_CLEANUP_STOPPED_AGE_DAYS").ok();
    let original_image_age = std::env::var("DOCKERMON_CLEANUP_IMAGE_AGE_DAYS").ok();

    std::env::set_var(
        "DOCKERMON_CLEANUP_STOPPED_AGE_DAYS",
        profile.stopped_container_age_days().to_string(),
    );
    std::env::set_var(
        "DOCKERMON_CLEANUP_IMAGE_AGE_DAYS",
        profile.unused_image_age_days().to_string(),
    );

    // Execute cleanup
    let mut result = super::execute_safe_cleanup(docker).await?;

    // Add unused image cleanup for moderate/aggressive profiles
    if profile.prune_unused_images() {
        match super::execute_unused_image_cleanup(docker).await {
            Ok(unused_result) => {
                result.unused_images_removed = unused_result.unused_images_removed;
                result.space_reclaimed_bytes += unused_result.space_reclaimed_bytes;
            }
            Err(e) => result.errors.push(format!("Failed to prune unused images: {}", e)),
        }
    }

    // Restore original environment variables
    if let Some(age) = original_container_age {
        std::env::set_var("DOCKERMON_CLEANUP_STOPPED_AGE_DAYS", age);
    } else {
        std::env::remove_var("DOCKERMON_CLEANUP_STOPPED_AGE_DAYS");
    }

    if let Some(age) = original_image_age {
        std::env::set_var("DOCKERMON_CLEANUP_IMAGE_AGE_DAYS", age);
    } else {
        std::env::remove_var("DOCKERMON_CLEANUP_IMAGE_AGE_DAYS");
    }

    Ok(result)
}
