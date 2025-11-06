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
        .route("/health", axum::routing::get(health_check))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    println!("Webhook server listening on http://{}", addr);
    println!("Available endpoints:");
    println!("  POST /webhook/update/os?server=<name>&token=<secret>");
    println!("  POST /webhook/update/docker/all?server=<name>&token=<secret>");
    println!("  POST /webhook/update/docker/image?server=<name>&image=<image>&token=<secret>");
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
