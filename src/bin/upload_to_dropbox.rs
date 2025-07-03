use anyhow::Result;
use std::env;
use std::sync::Arc;
use tobelog::{Config, DropboxClient};
use tokio::fs;
use tracing::{error, info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    dotenv::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run --bin upload_to_dropbox <markdown_file>");
        eprintln!("Example: cargo run --bin upload_to_dropbox test_manual_post.md");
        std::process::exit(1);
    }

    let file_path = &args[1];
    info!("📤 Uploading '{}' to Dropbox...", file_path);

    let config = Config::from_env()?;
    let dropbox_client = Arc::new(DropboxClient::new(config.dropbox_access_token));

    // Read the markdown file
    let content = fs::read_to_string(file_path).await?;
    info!("📖 Read {} bytes from '{}'", content.len(), file_path);

    // Extract filename
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // Upload to Dropbox posts folder
    let dropbox_path = format!("/BlogStorage/posts/{}", filename);

    match dropbox_client.upload_file(&dropbox_path, &content).await {
        Ok(metadata) => {
            info!("✅ Successfully uploaded to Dropbox!");
            info!("📍 Dropbox path: {}", metadata.path_display);
            info!("📏 File size: {} bytes", metadata.size.unwrap_or(0));
            info!("🔄 Now run sync to add to database:");
            info!("   cargo run --bin sync_dropbox_to_db");
        }
        Err(e) => {
            error!("❌ Failed to upload: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
