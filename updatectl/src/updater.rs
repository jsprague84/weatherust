use anyhow::Result;
use common::RemoteExecutor;
use crate::executor::UpdatectlExecutor;
use crate::types::PackageManager;
use crate::checkers::get_checker;

/// Update OS packages on a server
pub async fn update_os(executor: &RemoteExecutor, dry_run: bool) -> Result<String> {
    // Detect package manager
    let pm = executor.detect_package_manager().await?;

    if dry_run {
        // Just check what would be updated
        let checker = get_checker(&pm);
        let updates = executor.check_updates(&checker).await?;

        if updates.is_empty() {
            return Ok("No updates available".to_string());
        } else {
            return Ok(format!("{} packages would be updated", updates.len()));
        }
    }

    // Perform actual update based on package manager
    match pm {
        PackageManager::Apt => {
            // Update package lists
            executor.execute_command(
                "/usr/bin/sudo",
                &["apt-get", "update", "-qq"]
            ).await?;

            // Full upgrade (handles new dependencies and removals)
            // Uses full-upgrade instead of upgrade to match what updatemon detects
            executor.execute_command(
                "/usr/bin/sudo",
                &["DEBIAN_FRONTEND=noninteractive", "apt-get", "full-upgrade", "-y"]
            ).await?;
        }
        PackageManager::Dnf => {
            executor.execute_command(
                "/usr/bin/sudo",
                &["dnf", "upgrade", "-y"]
            ).await?;
        }
        PackageManager::Pacman => {
            executor.execute_command(
                "/usr/bin/sudo",
                &["pacman", "-Syu", "--noconfirm"]
            ).await?;
        }
    };

    // After update completes, verify by checking for remaining updates
    let checker = get_checker(&pm);
    let remaining = executor.check_updates(&checker).await?;

    // Report actual status based on verification
    if remaining.is_empty() {
        Ok("✅ Up to date".to_string())
    } else {
        Ok(format!("⚠️ {} updates still available (may require reboot or manual intervention)", remaining.len()))
    }
}

/// Update Docker images on a server
pub async fn update_docker(
    executor: &RemoteExecutor,
    all: bool,
    images: Option<&str>,
    dry_run: bool,
) -> Result<String> {
    if !all && images.is_none() {
        return Ok("No images specified (use --all or --images)".to_string());
    }

    // Get list of images to update
    let image_list = if all {
        get_all_images(executor).await?
    } else if let Some(imgs) = images {
        imgs.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };

    if image_list.is_empty() {
        return Ok("No images found".to_string());
    }

    if dry_run {
        return Ok(format!("{} images would be updated", image_list.len()));
    }

    // Pull each image and restart containers using them
    let mut updated = 0;
    let mut failed = 0;
    let mut restarted = 0;
    let mut restart_failed = 0;
    let mut skipped_webhook = false;

    for image in &image_list {
        // Pull the image
        match executor.execute_command("/usr/bin/docker", &["pull", image]).await {
            Ok(_) => {
                log::info!("Updated image: {}", image);
                updated += 1;

                // Find containers using this image and restart them
                match get_containers_using_image(executor, image).await {
                    Ok(containers) => {
                        if !containers.is_empty() {
                            log::info!("Found {} containers using {}: {}", containers.len(), image, containers.join(", "));

                            // Get restart policy and exclusion list
                            let policy = get_restart_policy();
                            let excluded = get_restart_exclusions(executor.server_name());

                            // Filter containers based on policy and exclusions
                            let containers_to_restart: Vec<_> = containers.iter()
                                .filter(|c| should_restart_container(c, &policy, &excluded))
                                .collect();

                            let skipped_count = containers.len() - containers_to_restart.len();
                            if skipped_count > 0 {
                                log::info!("Skipping {} container(s) based on restart policy", skipped_count);
                                skipped_webhook = true;
                            }

                            for container in &containers_to_restart {
                                match executor.execute_command("/usr/bin/docker", &["restart", container]).await {
                                    Ok(_) => {
                                        log::info!("Restarted container: {}", container);
                                        restarted += 1;
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to restart container {}: {}", container, e);
                                        restart_failed += 1;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to find containers using {}: {}", image, e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to update {}: {}", image, e);
                failed += 1;
            }
        }
    }

    // Build result message
    let mut parts = vec![format!("Updated {} images", updated)];
    if failed > 0 {
        parts.push(format!("{} failed", failed));
    }
    if restarted > 0 {
        parts.push(format!("restarted {} containers", restarted));
    }
    if restart_failed > 0 {
        parts.push(format!("{} restart failures", restart_failed));
    }
    if skipped_webhook {
        let policy = get_restart_policy();
        if policy == "none" {
            parts.push("no containers restarted (policy: none)".to_string());
        } else {
            parts.push("some containers excluded from restart".to_string());
        }
    }

    Ok(parts.join(", "))
}

/// Get list of all Docker images on a server
async fn get_all_images(executor: &RemoteExecutor) -> Result<Vec<String>> {
    let output = executor.execute_command(
        "/usr/bin/docker",
        &["images", "--format", "{{.Repository}}:{{.Tag}}"]
    ).await?;

    let images: Vec<String> = output
        .lines()
        .filter(|line| !line.contains("<none>"))
        .map(|line| line.trim().to_string())
        .collect();

    Ok(images)
}

/// Get list of containers using a specific image
async fn get_containers_using_image(executor: &RemoteExecutor, image: &str) -> Result<Vec<String>> {
    let output = executor.execute_command(
        "/usr/bin/docker",
        &["ps", "--format", "{{.Names}}:{{.Image}}", "--filter", &format!("ancestor={}", image)]
    ).await?;

    let containers: Vec<String> = output
        .lines()
        .filter_map(|line| {
            // Format is "container_name:image"
            line.split(':').next().map(|s| s.trim().to_string())
        })
        .filter(|name| !name.is_empty())
        .collect();

    Ok(containers)
}

/// Get restart policy from environment
fn get_restart_policy() -> String {
    std::env::var("UPDATECTL_RESTART_POLICY")
        .unwrap_or_else(|_| "all-except-webhook".to_string())
}

/// Get container exclusion list for a specific server
fn get_restart_exclusions(server_name: &str) -> Vec<String> {
    let mut excluded = Vec::new();

    // 1. Add default exclusions (applies to all servers)
    if let Ok(defaults) = std::env::var("UPDATECTL_RESTART_EXCLUDE_DEFAULT") {
        let default_list: Vec<String> = defaults.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        log::debug!("Default exclusions: {:?}", default_list);
        excluded.extend(default_list);
    }

    // 2. Add server-specific exclusions from UPDATECTL_RESTART_EXCLUDE
    // Format: "server1:container1,server1:container2,server2:container3"
    if let Ok(specific) = std::env::var("UPDATECTL_RESTART_EXCLUDE") {
        for pair in specific.split(',') {
            let pair = pair.trim();
            if let Some((server, container)) = pair.split_once(':') {
                let server = server.trim();
                let container = container.trim();

                // Check if this exclusion applies to current server
                if server.eq_ignore_ascii_case(server_name) {
                    log::debug!("Server-specific exclusion for {}: {}", server_name, container);
                    excluded.push(container.to_string());
                }
            }
        }
    }

    excluded
}

/// Check if a container should be restarted based on policy and exclusions
fn should_restart_container(container_name: &str, policy: &str, excluded: &[String]) -> bool {
    // Check if container is in exclusion list
    if excluded.iter().any(|ex| container_name.contains(ex)) {
        log::info!("Container {} excluded by UPDATECTL_RESTART_EXCLUDE", container_name);
        return false;
    }

    // Apply policy
    match policy {
        "none" => {
            log::info!("Container {} skipped (policy: none)", container_name);
            false
        }
        "all-except-webhook" => {
            if container_name.contains("updatectl_webhook") {
                log::info!("Container {} skipped (webhook server)", container_name);
                false
            } else {
                true
            }
        }
        _ => {
            log::warn!("Unknown restart policy '{}', defaulting to all-except-webhook", policy);
            !container_name.contains("updatectl_webhook")
        }
    }
}

