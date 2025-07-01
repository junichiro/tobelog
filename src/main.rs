use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{info, warn, Level};
use tracing_subscriber;

mod config;
mod handlers;
mod models;
mod services;

use handlers::{posts, api};
use services::{DropboxClient, BlogStorageService, DatabaseService, MarkdownService, TemplateService};

#[derive(Clone)]
struct AppState {
    dropbox_client: Arc<DropboxClient>,
    blog_storage: Arc<BlogStorageService>,
    // database: Arc<DatabaseService>,
    // markdown: Arc<MarkdownService>,
    // config: Arc<config::Config>,
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
    let dropbox_client = Arc::new(DropboxClient::new(config.dropbox_access_token.clone()));
    info!("Dropbox client initialized");

    // Initialize blog storage service
    let blog_storage = Arc::new(BlogStorageService::new(dropbox_client.clone()));
    info!("Blog storage service initialized");

    // Initialize database service
    let database = Arc::new(DatabaseService::new(&config.database_url).await?);
    info!("Database service initialized");

    // Initialize markdown service
    let markdown = Arc::new(MarkdownService::new());
    info!("Markdown service initialized");

    // Initialize template service
    let templates = Arc::new(TemplateService::new()?);
    info!("Template service initialized");

    // Test Dropbox connection on startup (with warning if it fails)
    match dropbox_client.test_connection().await {
        Ok(account_info) => {
            if let Some(name) = account_info.get("name") {
                if let Some(display_name) = name.get("display_name") {
                    info!("✅ Connected to Dropbox account: {}", display_name);
                }
            }
            
            // Initialize blog folder structure
            if let Err(e) = blog_storage.initialize_blog_structure().await {
                warn!("⚠️  Failed to initialize blog structure: {}", e);
            }
        }
        Err(e) => {
            warn!("⚠️  Dropbox connection test failed: {}. Server will start but Dropbox features may not work.", e);
        }
    }

    let app_state = AppState {
        dropbox_client,
        blog_storage,
        // database: database.clone(),
        // markdown: markdown.clone(),
        // config: Arc::new(config.clone()),
    };

    // Create handler states
    let posts_state = posts::AppState {
        database: (*database).clone(),
        markdown: (*markdown).clone(),
        templates: (*templates).clone(),
    };

    let api_state = api::ApiState {
        database: (*database).clone(),
    };
    
    // Create separate routers for each state type
    let web_pages_router = Router::new()
        .route("/", get(posts::home_page))
        .route("/posts/:year/:slug", get(posts::post_page))
        .with_state(posts_state.clone());

    let api_router = Router::new()
        .route("/api/posts", get(api::list_posts_api))
        .route("/api/posts/:slug", get(api::get_post_api))
        .route("/api/blog/stats", get(api::blog_stats_api))
        .route("/api/categories", get(api::list_categories_api))
        .route("/api/tags", get(api::list_tags_api))
        .route("/api/search", get(api::search_posts_api))
        .with_state(api_state);

    let legacy_router = Router::new()
        .route("/health", get(health_handler))
        .route("/api/dropbox/status", get(dropbox_status_handler))
        .route("/api/blog/posts", get(list_posts_handler))
        .route("/api/blog/posts/:slug", get(get_post_handler))
        .route("/api/blog/drafts", get(list_drafts_handler))
        .with_state(app_state);

    let app = Router::new()
        .merge(web_pages_router)
        .merge(api_router)
        .merge(legacy_router)
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        // Middleware
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive())); // TODO: Configure restrictive CORS policy for production

    let addr = format!("{}:{}", config.host, config.port);
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Remove the old root_handler since we're using the new handlers

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

async fn list_posts_handler(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match state.blog_storage.list_published_posts().await {
        Ok(posts) => {
            let response = json!({
                "posts": posts,
                "count": posts.len()
            });
            Ok(Json(response))
        }
        Err(e) => {
            let response = json!({
                "error": format!("Failed to list posts: {}", e)
            });
            Ok(Json(response))
        }
    }
}

async fn get_post_handler(Path(slug): Path<String>, State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match state.blog_storage.get_post_by_slug(&slug).await {
        Ok(Some(post)) => {
            Ok(Json(serde_json::to_value(post).unwrap()))
        }
        Ok(None) => {
            let response = json!({
                "error": format!("Post with slug '{}' not found", slug)
            });
            Ok(Json(response))
        }
        Err(e) => {
            let response = json!({
                "error": format!("Failed to get post: {}", e)
            });
            Ok(Json(response))
        }
    }
}

async fn list_drafts_handler(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match state.blog_storage.list_draft_posts().await {
        Ok(drafts) => {
            let response = json!({
                "drafts": drafts,
                "count": drafts.len()
            });
            Ok(Json(response))
        }
        Err(e) => {
            let response = json!({
                "error": format!("Failed to list drafts: {}", e)
            });
            Ok(Json(response))
        }
    }
}

// Unused handler - commented out to avoid dead code warning
// async fn blog_stats_handler(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
//     match state.blog_storage.get_blog_stats().await {
//         Ok(stats) => {
//             match serde_json::to_value(stats) {
//                 Ok(value) => Ok(Json(value)),
//                 Err(e) => {
//                     tracing::error!("Failed to serialize stats: {}", e);
//                     Ok(Json(json!({ "error": "Failed to serialize stats" })))
//                 }
//             }
//         }
//         Err(e) => {
//             let response = json!({
//                 "error": format!("Failed to get blog stats: {}", e)
//             });
//             Ok(Json(response))
//         }
//     }
// }