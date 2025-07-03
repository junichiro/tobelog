use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::Json,
    routing::{get, post, put, delete},
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
mod middleware;
mod models;
mod services;

use handlers::{posts, api, admin, version};
use services::{DropboxClient, BlogStorageService, DatabaseService, MarkdownService, TemplateService, LLMImportService, MediaService, VersionService};

#[derive(Clone)]
struct AppState {
    dropbox_client: Arc<DropboxClient>,
    blog_storage: Arc<BlogStorageService>,
    database: Arc<DatabaseService>,
    markdown: Arc<MarkdownService>,
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

    // Initialize LLM import service
    let llm_import = Arc::new(LLMImportService::new(
        (*markdown).clone(),
        (*database).clone(),
    ));
    info!("LLM import service initialized");

    // Initialize media service
    let media = Arc::new(MediaService::new(
        dropbox_client.clone(),
        blog_storage.clone(),
        (*database).clone(),
    ));
    info!("Media service initialized");

    // Initialize version service
    let version_service = Arc::new(VersionService::new(
        (*database).clone(),
        (*markdown).clone(),
    ));
    info!("Version service initialized");

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
        blog_storage: blog_storage.clone(),
        database: database.clone(),
        markdown: markdown.clone(),
        config: Arc::new(config.clone()),
    };

    // Create handler states
    let posts_state = posts::AppState {
        database: (*database).clone(),
        markdown: (*markdown).clone(),
        templates: (*templates).clone(),
    };

    let api_state = api::ApiState {
        database: (*database).clone(),
        llm_import: (*llm_import).clone(),
        markdown: (*markdown).clone(),
        blog_storage: blog_storage,
        media: (*media).clone(),
    };

    let admin_state = admin::AdminState {
        database: (*database).clone(),
        markdown: (*markdown).clone(),
        templates: (*templates).clone(),
        llm_import: (*llm_import).clone(),
    };

    let version_state = version::VersionState {
        version_service: (*version_service).clone(),
    };
    
    // Create separate routers for each state type
    let web_pages_router = Router::new()
        .route("/", get(posts::home_page))
        .route("/posts/:year/:slug", get(posts::post_page))
        .route("/category/:category", get(posts::category_page))
        .route("/tag/:tag", get(posts::tag_page))
        .with_state(posts_state.clone());

    let api_router = Router::new()
        // Read operations (no auth required)
        .route("/api/posts", get(api::list_posts_api))
        .route("/api/posts/:slug", get(api::get_post_api))
        .route("/api/blog/stats", get(api::blog_stats_api))
        .route("/api/categories", get(api::list_categories_api))
        .route("/api/tags", get(api::list_tags_api))
        .route("/api/search", get(api::search_posts_api))
        // CRUD operations (auth required)
        .route("/api/posts", post(api::create_post_api))
        .route("/api/posts/:slug", put(api::update_post_api))
        .route("/api/posts/:slug", delete(api::delete_post_api))
        // LLM import operations (auth required)
        .route("/api/import/llm-article", post(api::import_llm_article_api))
        .route("/api/import/batch", post(api::batch_import_api))
        .route("/api/posts/:slug/save", post(api::save_llm_article_api))
        // Media operations (auth required)
        .route("/api/media/upload", post(api::upload_media_api))
        .route("/api/media", get(api::list_media_api))
        .route("/api/media/:id", delete(api::delete_media_api))
        // Sync operations (auth required)
        .route("/api/sync/dropbox", post(api::sync_dropbox_api))
        .route("/api/import/markdown", post(api::import_markdown_api))
        .with_state(api_state.clone())
        .layer(from_fn_with_state(config.clone(), crate::middleware::auth_middleware));

    let admin_router = Router::new()
        .route("/admin", get(admin::dashboard))
        .route("/admin/posts", get(admin::posts_list))
        .route("/admin/new", get(admin::new_post_form))
        .route("/admin/edit/:slug", get(admin::edit_post_form))
        // LLM import admin routes
        .route("/admin/import", get(admin::admin_import_page).post(admin::admin_process_import))
        .route("/admin/posts/:slug/edit", get(admin::admin_edit_post_page))
        .with_state(admin_state);

    let version_router = Router::new()
        // Version management API endpoints (auth required)
        .route("/api/posts/:slug/versions", get(version::get_version_history))
        .route("/api/posts/:slug/versions/:version", get(version::get_post_version))
        .route("/api/posts/:slug/diff/:version_from/:version_to", get(version::compare_versions))
        .route("/api/posts/:slug/restore/:version", post(version::restore_version))
        .route("/api/posts/:slug/versions/cleanup", post(version::cleanup_old_versions))
        .with_state(version_state)
        .layer(from_fn_with_state(config.clone(), crate::middleware::auth_middleware));

    let legacy_router = Router::new()
        .route("/health", get(health_handler))
        .route("/api/dropbox/status", get(dropbox_status_handler))
        .route("/api/blog/posts", get(list_posts_handler))
        .route("/api/blog/posts/:slug", get(get_post_handler))
        .route("/api/blog/drafts", get(list_drafts_handler))
        .with_state(app_state);

    let media_router = Router::new()
        .route("/media/*path", get(api::serve_media_file))
        .with_state(api_state);

    let app = Router::new()
        .merge(web_pages_router)
        .merge(api_router)
        .merge(admin_router)
        .merge(version_router)
        .merge(legacy_router)
        .merge(media_router)
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