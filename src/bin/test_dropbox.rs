use anyhow::Result;
use tobelog::services::DropboxClient;
use tobelog::config::Config;
use tracing::{info, error, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    dotenv::dotenv().ok();

    info!("🧪 Testing Dropbox API connection...");

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            eprintln!("Make sure DROPBOX_ACCESS_TOKEN is set in your .env file");
            eprintln!("Run ./scripts/setup_dropbox.sh for setup instructions");
            std::process::exit(1);
        }
    };

    let dropbox_client = DropboxClient::new(config.dropbox_access_token);

    // Test connection
    info!("🔗 Testing connection to Dropbox API...");
    match dropbox_client.test_connection().await {
        Ok(account_info) => {
            info!("✅ Connection successful!");
            if let Some(name) = account_info.get("name") {
                if let Some(display_name) = name.get("display_name") {
                    info!("👤 Connected to account: {}", display_name);
                }
            }
            if let Some(email) = account_info.get("email") {
                info!("📧 Account email: {}", email);
            }
        }
        Err(e) => {
            error!("❌ Connection failed: {}", e);
            eprintln!("Check your DROPBOX_ACCESS_TOKEN and network connection");
            std::process::exit(1);
        }
    }

    // Test folder listing (try to list root folder)
    info!("📁 Testing folder listing...");
    match dropbox_client.list_folder("").await {
        Ok(result) => {
            info!("✅ Folder listing successful!");
            info!("📂 Found {} items in root folder", result.entries.len());
            
            for entry in result.entries.iter().take(5) {
                info!("  - {}", entry.name);
            }
            
            if result.entries.len() > 5 {
                info!("  ... and {} more items", result.entries.len() - 5);
            }
        }
        Err(e) => {
            error!("⚠️  Folder listing failed: {}", e);
            info!("This might be expected if your app folder is empty");
        }
    }

    // Test blog storage folder structure
    info!("🏗️  Checking blog storage folder structure...");
    let blog_folders = vec![
        "/BlogStorage",
        "/BlogStorage/posts",
        "/BlogStorage/media",
        "/BlogStorage/drafts",
        "/BlogStorage/templates",
        "/BlogStorage/config",
    ];

    for folder in &blog_folders {
        match dropbox_client.list_folder(folder).await {
            Ok(_) => {
                info!("✅ Folder exists: {}", folder);
            }
            Err(_) => {
                info!("📁 Creating folder: {}", folder);
                match dropbox_client.create_folder(folder).await {
                    Ok(_) => {
                        info!("✅ Created folder: {}", folder);
                    }
                    Err(e) => {
                        error!("❌ Failed to create folder {}: {}", folder, e);
                    }
                }
            }
        }
    }

    info!("🎉 Dropbox API test completed successfully!");
    info!("🚀 Your Dropbox integration is ready for tobelog!");

    Ok(())
}