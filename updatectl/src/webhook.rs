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
use crate::executor::RemoteExecutor;
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
    println!("  POST /webhook/cleanup/safe?token=<secret>");
    println!("  POST /webhook/cleanup/images/prune-unused?token=<secret>");
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

    log::info!("Webhook triggered: Safe cleanup (dangling images + unused networks)");

    let client = state.client.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_safe_cleanup().await {
            Ok(msg) => {
                log::info!("Safe cleanup completed: {}", msg);
                ("Docker Cleanup: Safe cleanup complete".to_string(), format!("✅ {}", msg))
            }
            Err(e) => {
                log::error!("Safe cleanup failed: {}", e);
                ("Docker Cleanup: Safe cleanup failed".to_string(), format!("❌ Error: {}", e))
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

    (StatusCode::ACCEPTED, "Safe cleanup started".to_string())
}

async fn handle_cleanup_prune_unused(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<CleanupQuery>,
) -> impl IntoResponse {
    if params.token != state.secret {
        log::warn!("Invalid webhook token for unused image cleanup");
        return (StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }

    log::info!("Webhook triggered: Prune unused images");

    let client = state.client.clone();
    tokio::spawn(async move {
        let (title, message) = match execute_prune_unused_images().await {
            Ok(msg) => {
                log::info!("Unused image cleanup completed: {}", msg);
                ("Docker Cleanup: Unused images pruned".to_string(), format!("✅ {}", msg))
            }
            Err(e) => {
                log::error!("Unused image cleanup failed: {}", e);
                ("Docker Cleanup: Unused image prune failed".to_string(), format!("❌ Error: {}", e))
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

    (StatusCode::ACCEPTED, "Unused image cleanup started".to_string())
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

async fn execute_safe_cleanup() -> Result<String> {
    use bollard::Docker;
    use bollard::image::PruneImagesOptions;
    use bollard::network::PruneNetworksOptions;
    use std::collections::HashMap;

    let docker = Docker::connect_with_unix_defaults()?;

    let mut results = Vec::new();
    let mut total_space_reclaimed: u64 = 0;

    // Prune dangling images
    let mut filters = HashMap::new();
    filters.insert("dangling", vec!["true"]);
    let image_prune_result = docker.prune_images(Some(PruneImagesOptions { filters })).await?;
    let image_count = image_prune_result.images_deleted.as_ref().map(|v| v.len()).unwrap_or(0);
    let image_space = image_prune_result.space_reclaimed.unwrap_or(0).max(0) as u64;
    total_space_reclaimed += image_space;
    results.push(format!("{} dangling images", image_count));

    // Prune unused networks
    let network_prune_result = docker.prune_networks(None::<PruneNetworksOptions<String>>).await?;
    let network_count = network_prune_result.networks_deleted.as_ref().map(|v| v.len()).unwrap_or(0);
    results.push(format!("{} unused networks", network_count));

    let space_str = if total_space_reclaimed >= 1024 * 1024 * 1024 {
        format!("{:.2}GB", total_space_reclaimed as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if total_space_reclaimed >= 1024 * 1024 {
        format!("{}MB", total_space_reclaimed / (1024 * 1024))
    } else {
        format!("{}KB", total_space_reclaimed / 1024)
    };

    Ok(format!("Removed {} | Reclaimed {}", results.join(" + "), space_str))
}

async fn execute_prune_unused_images() -> Result<String> {
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
