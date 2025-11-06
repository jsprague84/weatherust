use crate::cleanup::types::{ImageInfo, ImageStats};
use anyhow::Result;
use bollard::Docker;
use bollard::image::{ListImagesOptions, PruneImagesOptions};
use std::collections::HashMap;

/// Analyze dangling images (<none>:<none> tags)
pub async fn analyze_dangling_images(docker: &Docker) -> Result<ImageStats> {
    let mut filters = HashMap::new();
    filters.insert("dangling", vec!["true"]);

    let options = Some(ListImagesOptions {
        filters,
        ..Default::default()
    });

    let images = docker.list_images(options).await?;

    let mut stats = ImageStats::default();
    stats.count = images.len();

    for image in images {
        let size = image.size.unwrap_or(0) as u64;
        stats.total_size_bytes += size;

        stats.items.push(ImageInfo {
            repository: "<none>".to_string(),
            tag: "<none>".to_string(),
            image_id: image.id.clone(),
            size_bytes: size,
            created_timestamp: image.created,
        });
    }

    Ok(stats)
}

/// Analyze unused images (images with no running or stopped containers using them)
pub async fn analyze_unused_images(docker: &Docker) -> Result<ImageStats> {
    // Get all images
    let all_images = docker.list_images(None::<ListImagesOptions<String>>).await?;

    // Get all containers (including stopped)
    let containers = docker
        .list_containers(Some(bollard::container::ListContainersOptions {
            all: true,
            ..Default::default()
        }))
        .await?;

    // Build set of images in use
    let mut images_in_use = std::collections::HashSet::new();
    for container in containers {
        if let Some(image) = container.image {
            images_in_use.insert(image);
        }
        if let Some(image_id) = container.image_id {
            images_in_use.insert(image_id);
        }
    }

    let mut stats = ImageStats::default();
    let image_age_threshold_days = std::env::var("DOCKERMON_CLEANUP_IMAGE_AGE_DAYS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(90);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    for image in all_images {
        // Skip dangling images (they're in the other category)
        let repo_tags = image.repo_tags.unwrap_or_default();
        if repo_tags.is_empty() || repo_tags.iter().any(|t| t.contains("<none>")) {
            continue;
        }

        // Check if image is in use
        let image_id = image.id.clone();
        let is_in_use = images_in_use.contains(&image_id)
            || repo_tags.iter().any(|tag| images_in_use.contains(tag));

        if is_in_use {
            continue;
        }

        // Check age threshold
        let age_days = (now - image.created) / 86400;
        if age_days < image_age_threshold_days {
            continue; // Too recent, skip
        }

        let size = image.size.unwrap_or(0) as u64;
        stats.total_size_bytes += size;
        stats.count += 1;

        let (repo, tag) = if let Some(first_tag) = repo_tags.first() {
            if let Some((r, t)) = first_tag.split_once(':') {
                (r.to_string(), t.to_string())
            } else {
                (first_tag.clone(), "latest".to_string())
            }
        } else {
            ("<none>".to_string(), "<none>".to_string())
        };

        stats.items.push(ImageInfo {
            repository: repo,
            tag,
            image_id: image_id.clone(),
            size_bytes: size,
            created_timestamp: image.created,
        });
    }

    Ok(stats)
}

/// Prune dangling images
pub async fn prune_dangling_images(docker: &Docker) -> Result<PruneStats> {
    let mut filters = HashMap::new();
    filters.insert("dangling", vec!["true"]);

    let options = Some(PruneImagesOptions { filters });

    let result = docker.prune_images(options).await?;

    let space_reclaimed = result.space_reclaimed.unwrap_or(0);
    let count = result.images_deleted.map(|v| v.len()).unwrap_or(0);

    Ok(PruneStats {
        count,
        space_reclaimed,
    })
}

/// Prune unused images (requires confirmation)
pub async fn prune_unused_images(docker: &Docker) -> Result<PruneStats> {
    // Prune all unused images (not just dangling)
    let result = docker.prune_images(None::<PruneImagesOptions<String>>).await?;

    let space_reclaimed = result.space_reclaimed.unwrap_or(0);
    let count = result.images_deleted.map(|v| v.len()).unwrap_or(0);

    Ok(PruneStats {
        count,
        space_reclaimed,
    })
}

#[derive(Debug)]
pub struct PruneStats {
    pub count: usize,
    pub space_reclaimed: u64,
}
