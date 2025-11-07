use anyhow::Result;
use serde::Deserialize;

use common::RemoteExecutor;

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
    // ID field exists but we don't need it
}

/// Check for Docker image updates
pub async fn check_docker_updates(executor: &RemoteExecutor) -> Result<Vec<DockerImage>> {
    // Get list of images (use full path for SSH compatibility)
    let output = executor
        .execute_command("/usr/bin/docker", &["images", "--format", "{{json .}}"])
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

                // Check if this image has updates available
                let has_update = match check_image_update(executor, &info.repository, &info.tag).await {
                    Ok(update_available) => update_available,
                    Err(e) => {
                        log::warn!("Could not check updates for {}:{} - {}", info.repository, info.tag, e);
                        false // Assume no update on error to avoid false positives
                    }
                };

                images.push(DockerImage {
                    name: info.repository.clone(),
                    current_tag: info.tag.clone(),
                    has_update,
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
async fn check_image_update(
    executor: &RemoteExecutor,
    image_name: &str,
    tag: &str,
) -> Result<bool> {
    // Get local image digest using docker inspect (more reliable than --digests)
    let local_output = executor
        .execute_command(
            "/usr/bin/docker",
            &[
                "inspect",
                &format!("{}:{}", image_name, tag),
                "--format={{index .RepoDigests 0}}",
            ],
        )
        .await?;

    let local_repo_digest = local_output.trim();
    if local_repo_digest.is_empty() || local_repo_digest == "<no value>" {
        log::debug!("No RepoDigest found for {}:{}", image_name, tag);
        return Ok(false); // Can't compare without local digest
    }

    // RepoDigest format: "image@sha256:abc123..."
    // Extract just the sha256:... part
    let local_digest = if let Some(hash) = local_repo_digest.split('@').nth(1) {
        hash
    } else {
        log::debug!("Could not parse RepoDigest for {}:{}: {}", image_name, tag, local_repo_digest);
        return Ok(false);
    };

    log::debug!("Local digest for {}:{} is {}", image_name, tag, local_digest);

    // Get remote digest using docker manifest inspect
    // This pulls the latest manifest from the registry without downloading the image
    let remote_output = executor
        .execute_command(
            "/usr/bin/docker",
            &["manifest", "inspect", &format!("{}:{}", image_name, tag)],
        )
        .await;

    match remote_output {
        Ok(output) => {
            // Parse manifest JSON to extract digest
            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&output) {
                // Try multiple paths to find the digest:
                // 1. config.digest (for image manifests)
                // 2. For manifest lists, we need to look at the manifests array
                let remote_digest = manifest
                    .get("config")
                    .and_then(|c| c.get("digest"))
                    .and_then(|d| d.as_str())
                    .or_else(|| {
                        // Check if this is a manifest list (multi-arch)
                        // In that case, we should check if ANY platform has a different digest
                        // For simplicity, we'll check the first manifest's digest
                        manifest
                            .get("manifests")
                            .and_then(|m| m.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|first| first.get("digest"))
                            .and_then(|d| d.as_str())
                    });

                if let Some(digest) = remote_digest {
                    log::debug!("Remote digest for {}:{} is {}", image_name, tag, digest);
                    log::debug!("Comparing: local='{}' vs remote='{}'", local_digest, digest);
                    // Update available if digests differ
                    Ok(digest != local_digest)
                } else {
                    log::debug!("Could not parse digest from manifest for {}:{}", image_name, tag);
                    log::debug!("Manifest structure: {}", serde_json::to_string_pretty(&manifest).unwrap_or_default());
                    Ok(false)
                }
            } else {
                log::debug!("Could not parse manifest JSON for {}:{}", image_name, tag);
                Ok(false)
            }
        }
        Err(e) => {
            log::debug!(
                "Could not fetch remote manifest for {}:{} - {}",
                image_name,
                tag,
                e
            );
            // If we can't check remote, assume no update to avoid false positives
            // This can happen with:
            // - Private registries without auth
            // - Rate limiting (Docker Hub)
            // - Network issues
            // - Invalid image names
            Ok(false)
        }
    }
}
