use anyhow::Result;
use crate::executor::RemoteExecutor;
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
    let result = match pm {
        PackageManager::Apt => {
            // Update package lists
            executor.execute_command(
                "/usr/bin/sudo",
                &["apt-get", "update", "-qq"]
            ).await?;

            // Full upgrade (handles new dependencies and removals)
            // Uses full-upgrade instead of upgrade to match what updatemon detects
            let output = executor.execute_command(
                "/usr/bin/sudo",
                &["DEBIAN_FRONTEND=noninteractive", "apt-get", "full-upgrade", "-y"]
            ).await?;

            parse_apt_output(&output)
        }
        PackageManager::Dnf => {
            let output = executor.execute_command(
                "/usr/bin/sudo",
                &["dnf", "upgrade", "-y"]
            ).await?;

            parse_dnf_output(&output)
        }
        PackageManager::Pacman => {
            let output = executor.execute_command(
                "/usr/bin/sudo",
                &["pacman", "-Syu", "--noconfirm"]
            ).await?;

            parse_pacman_output(&output)
        }
    };

    Ok(result)
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
                            for container in &containers {
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

/// Parse apt-get upgrade output to count updated packages
fn parse_apt_output(output: &str) -> String {
    // Look for line like "X upgraded, Y newly installed, Z to remove"
    for line in output.lines() {
        if line.contains("upgraded") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(count_str) = parts.first() {
                if let Ok(count) = count_str.parse::<i32>() {
                    if count > 0 {
                        return format!("✅ {} packages upgraded", count);
                    }
                }
            }
        }
    }

    // Fallback if we can't parse
    if output.contains("0 upgraded") {
        "✅ Already up to date".to_string()
    } else {
        "✅ Upgrade completed".to_string()
    }
}

/// Parse dnf upgrade output
fn parse_dnf_output(output: &str) -> String {
    // Look for "Complete!" or "Nothing to do"
    if output.contains("Nothing to do") {
        "✅ Already up to date".to_string()
    } else if output.contains("Complete!") {
        // Try to count upgraded packages
        let count = output.lines()
            .filter(|line| line.starts_with("Upgrading "))
            .count();

        if count > 0 {
            format!("✅ {} packages upgraded", count)
        } else {
            "✅ Upgrade completed".to_string()
        }
    } else {
        "✅ Upgrade completed".to_string()
    }
}

/// Parse pacman update output
fn parse_pacman_output(output: &str) -> String {
    if output.contains("there is nothing to do") {
        "✅ Already up to date".to_string()
    } else if output.contains("Total Installed Size") || output.contains("Total Download Size") {
        "✅ Upgrade completed".to_string()
    } else {
        "✅ Upgrade completed".to_string()
    }
}
