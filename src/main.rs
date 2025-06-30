use axum::{
    response::Html,
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{info, Level};
use tracing_subscriber;

mod config;
mod handlers;
mod models;
mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    dotenv::dotenv().ok();

    let config = config::Config::from_env()?;
    
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
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