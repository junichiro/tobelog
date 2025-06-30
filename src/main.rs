use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, Level};
use tracing_subscriber;

mod config;
mod handlers;
mod models;
mod services;

use services::DropboxClient;

#[derive(Clone)]
struct AppState {
    dropbox_client: Arc<DropboxClient>,
    config: Arc<config::Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    dotenv::dotenv().ok();

    let config = config::Config::from_env()?;
    info!("Configuration loaded successfully");

    // Initialize Dropbox client
    let dropbox_client = DropboxClient::new(config.dropbox_access_token.clone());
    info!("Dropbox client initialized");

    // Test Dropbox connection on startup (with warning if it fails)
    match dropbox_client.test_connection().await {
        Ok(account_info) => {
            if let Some(name) = account_info.get("name") {
                if let Some(display_name) = name.get("display_name") {
                    info!("✅ Connected to Dropbox account: {}", display_name);
                }
            }
        }
        Err(e) => {
            warn!("⚠️  Dropbox connection test failed: {}. Server will start but Dropbox features may not work.", e);
        }
    }

    let app_state = AppState {
        dropbox_client: Arc::new(dropbox_client),
        config: Arc::new(config.clone()),
    };
    
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/api/dropbox/status", get(dropbox_status_handler))
        .with_state(app_state)
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive())); // TODO: Configure restrictive CORS policy for production

    let addr = format!("{}:{}", config.host, config.port);
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root_handler() -> Html<&'static str> {
    Html("<h1>Tobelog - Personal Blog System</h1><p>Coming soon...</p>")
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn dropbox_status_handler(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match state.dropbox_client.test_connection().await {
        Ok(account_info) => {
            let response = json!({
                "status": "connected",
                "account": {
                    "name": account_info.get("name").and_then(|n| n.get("display_name")),
                    "email": account_info.get("email"),
                    "account_id": account_info.get("account_id")
                },
                "message": "Dropbox API connection successful"
            });
            Ok(Json(response))
        }
        Err(e) => {
            let response = json!({
                "status": "error",
                "message": format!("Dropbox API connection failed: {}", e)
            });
            Ok(Json(response))
        }
    }
}