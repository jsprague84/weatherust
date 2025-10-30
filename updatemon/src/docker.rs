use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::executor::RemoteExecutor;

/// Represents a Docker image with update status
#[derive(Debug, Clone)]
pub struct DockerImage {
    pub name: String,
    pub current_tag: String,
    pub has_update: bool,
}

impl std::fmt::Display for DockerImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.has_update {
            write!(f, "{}:{} (update available)", self.name, self.current_tag)
        } else {
            write!(f, "{}:{}", self.name, self.current_tag)
        }
    }
}

#[derive(Debug, Deserialize)]
struct ImageInfo {
    #[serde(rename = "Repository")]
    repository: String,
    #[serde(rename = "Tag")]
    tag: String,
    #[serde(rename = "ID")]
    id: String,
}

/// Check for Docker image updates
pub async fn check_docker_updates(executor: &RemoteExecutor) -> Result<Vec<DockerImage>> {
    // Get list of images
    let output = executor
        .execute_command("docker", &["images", "--format", "{{json .}}"])
        .await?;

    if output.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut images = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON line
        match serde_json::from_str::<ImageInfo>(line) {
            Ok(info) => {
                // Skip <none> tags
                if info.tag == "<none>" || info.repository == "<none>" {
                    continue;
                }

                // For now, we'll mark has_update as false
                // Full implementation would query registry API
                // This is a placeholder for the MVP
                images.push(DockerImage {
                    name: info.repository.clone(),
                    current_tag: info.tag.clone(),
                    has_update: false, // TODO: Actually check registry
                });
            }
            Err(e) => {
                log::warn!("Failed to parse docker image JSON: {} - line: {}", e, line);
            }
        }
    }

    // Deduplicate by name:tag
    images.sort_by(|a, b| {
        format!("{}:{}", a.name, a.current_tag)
            .cmp(&format!("{}:{}", b.name, b.current_tag))
    });
    images.dedup_by(|a, b| {
        a.name == b.name && a.current_tag == b.current_tag
    });

    Ok(images)
}

/// Check if a specific Docker image has updates available
/// This queries the registry to compare digests
pub async fn check_image_update(
    executor: &RemoteExecutor,
    image_name: &str,
    tag: &str,
) -> Result<bool> {
    // Get local image digest
    let local_output = executor
        .execute_command(
            "docker",
            &[
                "images",
                "--digests",
                "--format",
                "{{.Digest}}",
                &format!("{}:{}", image_name, tag),
            ],
        )
        .await?;

    let local_digest = local_output.trim();
    if local_digest.is_empty() {
        return Err(anyhow!("Could not find local image {}:{}", image_name, tag));
    }

    // Get remote digest using docker manifest inspect
    // Note: This requires experimental CLI features in older Docker versions
    let remote_output = executor
        .execute_command(
            "docker",
            &["manifest", "inspect", &format!("{}:{}", image_name, tag)],
        )
        .await;

    match remote_output {
        Ok(output) => {
            // Parse the manifest to get digest
            // The manifest command returns JSON with config.digest or similar
            // For simplicity, we check if output contains the local digest
            Ok(!output.contains(local_digest))
        }
        Err(e) => {
            log::warn!(
                "Could not check remote digest for {}:{} - {}",
                image_name,
                tag,
                e
            );
            // If we can't check remote, assume no update to avoid false positives
            Ok(false)
        }
    }
}
