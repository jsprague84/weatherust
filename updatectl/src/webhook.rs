use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use common::{send_gotify_updatectl, send_ntfy_updatectl};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::collections::HashMap;
use tower_http::trace::TraceLayer;

use crate::types::Server;
use common::RemoteExecutor;
use crate::updater::{update_os, update_docker};

#[derive(Clone)]
pub struct WebhookState {
    pub secret: String,
    pub servers: HashMap<String, Server>,
    pub ssh_key: Option<String>,
    pub client: Client,
}

#[derive(Debug, Deserialize)]
pub struct WebhookQuery {
    server: String,
    token: String,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CleanupQuery {
    server: String,
    token: String,
}

/// Start the webhook server
pub async fn serve_webhooks(
    port: u16,
    secret: String,
    servers: HashMap<String, Server>,
    ssh_key: Option<String>,
) -> Result<()> {
    let client = Client::new();
    let state = Arc::new(WebhookState {
        secret,
        servers,
        ssh_key,
        client,
    });

    let app = Router::new()
        .route("/webhook/update/os", post(handle_os_update))
        .route("/webhook/update/docker/all", post(handle_docker_all_update))
        .route("/webhook/update/docker/image", post(handle_docker_image_update))
        .route("/webhook/cleanup/safe", post(handle_cleanup_safe))
        .route("/webhook/cleanup/images/prune-unused", post(handle_cleanup_prune_unused))
        .route("/health", axum::routing::get(health_check))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    println!("Webhook server listening on http://{}", addr);
    println!("Available endpoints:");
    println!("  POST /webhook/update/os?server=<name>&token=<secret>");
    println!("  POST /webhook/update/docker/all?server=<name>&token=<secret>");
    println!("  POST /webhook/update/docker/image?server=<name>&image=<image>&token=<secret>");
    println!("  POST /webhook/cleanup/safe?server=<name>&token=<secret>");
    println!("  POST /webhook/cleanup/images/prune-unused?server=<name>&token=<secret>");
    println!("  GET  /health");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn handle_os_update(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<WebhookQuery>,
) -> impl IntoResponse {
    // Verify token
    if params.token != state.secret {
        log::warn!("Invalid webhook token for OS update");
        return (StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }

    // Get server
    let server = match state.servers.get(&params.server) {
        Some(s) => s.clone(),
        None => {
            log::error!("Unknown server: {}", params.server);
            return (StatusCode::BAD_REQUEST, format!("Unknown server: {}", params.server));
        }
    };

    log::info!("Webhook triggered: OS update for {}", server.name);

    // Execute update in background
    let ssh_key = state.ssh_key.clone();
    let client = state.client.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_os_update(&server, ssh_key.as_deref()).await {
            Ok(msg) => {
                log::info!("OS update completed: {}", msg);
                (
                    format!("{} - OS update complete", server.name),
                    format!("✅ {}", msg)
                )
            }
            Err(e) => {
                log::error!("OS update failed: {}", e);
                (
                    format!("{} - OS update failed", server.name),
                    format!("❌ Error: {}", e)
                )
            }
        };

        // Send notification (both Gotify and ntfy if configured)
        if let Err(e) = send_gotify_updatectl(&client, &title, &message).await {
            log::warn!("Failed to send Gotify notification: {}", e);
        }
        if let Err(e) = send_ntfy_updatectl(&client, &title, &message, None).await {
            log::warn!("Failed to send ntfy notification: {}", e);
        }
    });

    (StatusCode::ACCEPTED, format!("OS update started for {}", params.server))
}

async fn handle_docker_all_update(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<WebhookQuery>,
) -> impl IntoResponse {
    if params.token != state.secret {
        log::warn!("Invalid webhook token for Docker update");
        return (StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }

    let server = match state.servers.get(&params.server) {
        Some(s) => s.clone(),
        None => {
            log::error!("Unknown server: {}", params.server);
            return (StatusCode::BAD_REQUEST, format!("Unknown server: {}", params.server));
        }
    };

    log::info!("Webhook triggered: Docker all update for {}", server.name);

    let ssh_key = state.ssh_key.clone();
    let client = state.client.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_docker_update(&server, true, None, ssh_key.as_deref()).await {
            Ok(msg) => {
                log::info!("Docker update completed: {}", msg);
                (
                    format!("{} - Docker update complete", server.name),
                    format!("✅ {}", msg)
                )
            }
            Err(e) => {
                log::error!("Docker update failed: {}", e);
                (
                    format!("{} - Docker update failed", server.name),
                    format!("❌ Error: {}", e)
                )
            }
        };

        // Send notification (both Gotify and ntfy if configured)
        if let Err(e) = send_gotify_updatectl(&client, &title, &message).await {
            log::warn!("Failed to send Gotify notification: {}", e);
        }
        if let Err(e) = send_ntfy_updatectl(&client, &title, &message, None).await {
            log::warn!("Failed to send ntfy notification: {}", e);
        }
    });

    (StatusCode::ACCEPTED, format!("Docker update started for {}", params.server))
}

async fn handle_docker_image_update(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<WebhookQuery>,
) -> impl IntoResponse {
    if params.token != state.secret {
        log::warn!("Invalid webhook token for Docker image update");
        return (StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }

    let image = match params.image {
        Some(img) => img,
        None => {
            return (StatusCode::BAD_REQUEST, "Missing image parameter".to_string());
        }
    };

    let server = match state.servers.get(&params.server) {
        Some(s) => s.clone(),
        None => {
            log::error!("Unknown server: {}", params.server);
            return (StatusCode::BAD_REQUEST, format!("Unknown server: {}", params.server));
        }
    };

    log::info!("Webhook triggered: Docker image {} update for {}", image, server.name);

    let ssh_key = state.ssh_key.clone();
    let client = state.client.clone();
    let image_clone = image.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_docker_update(&server, false, Some(&image_clone), ssh_key.as_deref()).await {
            Ok(msg) => {
                log::info!("Docker image update completed: {}", msg);
                (
                    format!("{} - Docker image update complete", server.name),
                    format!("✅ {}", msg)
                )
            }
            Err(e) => {
                log::error!("Docker image update failed: {}", e);
                (
                    format!("{} - Docker image update failed", server.name),
                    format!("❌ Error: {}", e)
                )
            }
        };

        // Send notification (both Gotify and ntfy if configured)
        if let Err(e) = send_gotify_updatectl(&client, &title, &message).await {
            log::warn!("Failed to send Gotify notification: {}", e);
        }
        if let Err(e) = send_ntfy_updatectl(&client, &title, &message, None).await {
            log::warn!("Failed to send ntfy notification: {}", e);
        }
    });

    (StatusCode::ACCEPTED, format!("Docker image {} update started for {}", image, params.server))
}

async fn handle_cleanup_safe(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<CleanupQuery>,
) -> impl IntoResponse {
    if params.token != state.secret {
        log::warn!("Invalid webhook token for safe cleanup");
        return (StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }

    // Get server from registry
    let server = match state.servers.get(&params.server) {
        Some(s) => s.clone(),
        None => {
            log::error!("Unknown server: {}", params.server);
            return (StatusCode::BAD_REQUEST, format!("Unknown server: {}", params.server));
        }
    };

    log::info!("Webhook triggered: Safe cleanup for {}", server.name);

    let ssh_key = state.ssh_key.clone();
    let client = state.client.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_safe_cleanup_for_server(&server, ssh_key.as_deref()).await {
            Ok(msg) => {
                log::info!("Safe cleanup completed: {}", msg);
                (
                    format!("{} - Docker Cleanup: Complete", server.name),
                    format!("✅ {}", msg)
                )
            }
            Err(e) => {
                log::error!("Safe cleanup failed: {}", e);
                (
                    format!("{} - Docker Cleanup: Failed", server.name),
                    format!("❌ Error: {}", e)
                )
            }
        };

        // Send notification (both Gotify and ntfy if configured)
        if let Err(e) = send_gotify_updatectl(&client, &title, &message).await {
            log::warn!("Failed to send Gotify notification: {}", e);
        }
        if let Err(e) = send_ntfy_updatectl(&client, &title, &message, None).await {
            log::warn!("Failed to send ntfy notification: {}", e);
        }
    });

    (StatusCode::ACCEPTED, format!("Safe cleanup started for {}", params.server))
}

async fn handle_cleanup_prune_unused(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<CleanupQuery>,
) -> impl IntoResponse {
    if params.token != state.secret {
        log::warn!("Invalid webhook token for unused image cleanup");
        return (StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }

    // Get server from registry
    let server = match state.servers.get(&params.server) {
        Some(s) => s.clone(),
        None => {
            log::error!("Unknown server: {}", params.server);
            return (StatusCode::BAD_REQUEST, format!("Unknown server: {}", params.server));
        }
    };

    log::info!("Webhook triggered: Prune unused images for {}", server.name);

    let ssh_key = state.ssh_key.clone();
    let client = state.client.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_prune_unused_images_for_server(&server, ssh_key.as_deref()).await {
            Ok(msg) => {
                log::info!("Unused image cleanup completed: {}", msg);
                (
                    format!("{} - Docker Cleanup: Unused images pruned", server.name),
                    format!("✅ {}", msg)
                )
            }
            Err(e) => {
                log::error!("Unused image cleanup failed: {}", e);
                (
                    format!("{} - Docker Cleanup: Unused image prune failed", server.name),
                    format!("❌ Error: {}", e)
                )
            }
        };

        // Send notification (both Gotify and ntfy if configured)
        if let Err(e) = send_gotify_updatectl(&client, &title, &message).await {
            log::warn!("Failed to send Gotify notification: {}", e);
        }
        if let Err(e) = send_ntfy_updatectl(&client, &title, &message, None).await {
            log::warn!("Failed to send ntfy notification: {}", e);
        }
    });

    (StatusCode::ACCEPTED, format!("Unused image cleanup started for {}", params.server))
}

async fn execute_os_update(server: &Server, ssh_key: Option<&str>) -> Result<String> {
    let executor = RemoteExecutor::new(server.clone(), ssh_key)?;
    let result = update_os(&executor, false).await?;
    Ok(format!("OS: {}", result))
}

async fn execute_docker_update(
    server: &Server,
    all: bool,
    images: Option<&str>,
    ssh_key: Option<&str>,
) -> Result<String> {
    let executor = RemoteExecutor::new(server.clone(), ssh_key)?;
    let result = update_docker(&executor, all, images, false).await?;
    Ok(format!("Docker: {}", result))
}

async fn execute_safe_cleanup_for_server(server: &Server, ssh_key: Option<&str>) -> Result<String> {
    use crate::cleanup::profiles::CleanupProfile;

    if server.is_local() {
        // Local cleanup using Bollard
        let docker = bollard::Docker::connect_with_unix_defaults()?;
        let result = crate::cleanup::profiles::execute_cleanup_with_profile(
            &docker,
            CleanupProfile::Conservative
        ).await?;

        let mut parts = Vec::new();
        if result.dangling_images_removed > 0 {
            parts.push(format!("{} dangling images", result.dangling_images_removed));
        }
        if result.networks_removed > 0 {
            parts.push(format!("{} networks", result.networks_removed));
        }
        if result.stopped_containers_removed > 0 {
            parts.push(format!("{} containers", result.stopped_containers_removed));
        }

        Ok(format!("Removed {} | Reclaimed {}",
            parts.join(" + "),
            crate::cleanup::format_bytes(result.space_reclaimed_bytes)))
    } else {
        // Remote cleanup via SSH
        let executor = RemoteExecutor::new(server.clone(), ssh_key)?;
        let result = crate::remote_cleanup::execute_cleanup_with_profile_remote(
            &executor,
            CleanupProfile::Conservative
        ).await?;

        let mut parts = Vec::new();
        if result.dangling_images_removed > 0 {
            parts.push(format!("{} dangling images", result.dangling_images_removed));
        }
        if result.networks_removed > 0 {
            parts.push(format!("{} networks", result.networks_removed));
        }
        if result.stopped_containers_removed > 0 {
            parts.push(format!("{} containers", result.stopped_containers_removed));
        }

        Ok(format!("Removed {} | Reclaimed {}",
            parts.join(" + "),
            crate::cleanup::format_bytes(result.space_reclaimed_bytes)))
    }
}

async fn execute_prune_unused_images_for_server(server: &Server, ssh_key: Option<&str>) -> Result<String> {
    if server.is_local() {
        execute_prune_unused_images_local().await
    } else {
        execute_prune_unused_images_remote(server, ssh_key).await
    }
}

async fn execute_prune_unused_images_local() -> Result<String> {
    use bollard::Docker;
    use bollard::image::PruneImagesOptions;

    let docker = Docker::connect_with_unix_defaults()?;

    // Prune all unused images (not just dangling)
    let prune_result = docker.prune_images(None::<PruneImagesOptions<String>>).await?;
    let count = prune_result.images_deleted.as_ref().map(|v| v.len()).unwrap_or(0);
    let space = prune_result.space_reclaimed.unwrap_or(0).max(0) as u64;

    let space_str = if space >= 1024 * 1024 * 1024 {
        format!("{:.2}GB", space as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if space >= 1024 * 1024 {
        format!("{}MB", space / (1024 * 1024))
    } else {
        format!("{}KB", space / 1024)
    };

    Ok(format!("Removed {} unused images | Reclaimed {}", count, space_str))
}

async fn execute_prune_unused_images_remote(server: &Server, ssh_key: Option<&str>) -> Result<String> {
    let executor = RemoteExecutor::new(server.clone(), ssh_key)?;

    // Prune all unused images (not just dangling) - this is more aggressive
    let prune_output = executor.execute_command(
        "/usr/bin/docker",
        &["image", "prune", "-a", "-f"]
    ).await?;

    // Parse output to count removed images and space reclaimed
    let count = prune_output.lines()
        .filter(|line| line.starts_with("deleted:") || line.starts_with("untagged:"))
        .count();
    let space = parse_reclaimed_space(&prune_output);

    let space_str = if space >= 1024 * 1024 * 1024 {
        format!("{:.2}GB", space as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if space >= 1024 * 1024 {
        format!("{}MB", space / (1024 * 1024))
    } else {
        format!("{}KB", space / 1024)
    };

    Ok(format!("Removed {} unused images | Reclaimed {}", count, space_str))
}

/// Parse Docker's "Total reclaimed space: X.XXkB/MB/GB" output
fn parse_reclaimed_space(output: &str) -> u64 {
    for line in output.lines() {
        if line.contains("Total reclaimed space:") || line.contains("reclaimed:") {
            // Extract the size part (e.g., "1.23GB" or "456MB")
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(size_str) = parts.last() {
                return parse_docker_size_str(size_str);
            }
        }
    }
    0
}

/// Parse Docker size string (e.g., "1.5GB", "250MB", "1.2kB")
fn parse_docker_size_str(size_str: &str) -> u64 {
    let size_str = size_str.trim().to_uppercase();

    // Extract number and unit
    let (num_str, multiplier) = if size_str.ends_with("GB") {
        (&size_str[..size_str.len()-2], 1024 * 1024 * 1024)
    } else if size_str.ends_with("MB") {
        (&size_str[..size_str.len()-2], 1024 * 1024)
    } else if size_str.ends_with("KB") {
        (&size_str[..size_str.len()-2], 1024)
    } else if size_str.ends_with('B') {
        (&size_str[..size_str.len()-1], 1)
    } else {
        (size_str.as_str(), 1)
    };

    // Parse the number (may be float like "1.5")
    if let Ok(num) = num_str.parse::<f64>() {
        (num * multiplier as f64) as u64
    } else {
        0
    }
}
