use anyhow::Result;
use std::sync::Arc;
use tobelog::{Config, DropboxClient};
use tobelog::services::{BlogStorageService, DatabaseService, MarkdownService};
use tobelog::models::CreatePost;
use tracing::{info, warn, error, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    dotenv::dotenv().ok();

    info!("🔄 Starting Dropbox to Database synchronization...");

    let config = Config::from_env()?;
    
    // Initialize services
    let dropbox_client = Arc::new(DropboxClient::new(config.dropbox_access_token.clone()));
    let blog_storage = Arc::new(BlogStorageService::new(dropbox_client));
    let database = Arc::new(DatabaseService::new(&config.database_url).await?);
    let markdown_service = MarkdownService::new();

    // Get all published posts from Dropbox
    info!("📥 Fetching published posts from Dropbox...");
    let dropbox_posts = blog_storage.list_published_posts().await?;
    info!("Found {} posts in Dropbox", dropbox_posts.len());

    // Get existing posts from database to avoid duplicates
    let db_posts = database.list_posts(Default::default()).await?;
    let existing_slugs: std::collections::HashSet<String> = db_posts
        .into_iter()
        .map(|post| post.slug)
        .collect();

    info!("Found {} existing posts in database", existing_slugs.len());

    let mut synced_count = 0;
    let mut skipped_count = 0;

    let total_dropbox_posts = dropbox_posts.len();
    
    for dropbox_post in dropbox_posts {
        let slug = &dropbox_post.metadata.slug;
        
        if existing_slugs.contains(slug) {
            info!("⏭️  Skipping '{}' - already exists in database", dropbox_post.metadata.title);
            skipped_count += 1;
            continue;
        }

        info!("📝 Syncing post: '{}'", dropbox_post.metadata.title);

        // Convert markdown to HTML
        let html_content = match markdown_service.markdown_to_html(&dropbox_post.content) {
            Ok(html) => html,
            Err(e) => {
                warn!("Failed to convert markdown to HTML for '{}': {}", dropbox_post.metadata.title, e);
                format!("<p>{}</p>", html_escape::encode_text(&dropbox_post.content))
            }
        };

        // Create database post
        let create_post = CreatePost {
            slug: dropbox_post.metadata.slug.clone(),
            title: dropbox_post.metadata.title.clone(),
            content: dropbox_post.content.clone(),
            html_content,
            excerpt: dropbox_post.metadata.excerpt.clone(),
            category: dropbox_post.metadata.category.clone(),
            tags: dropbox_post.metadata.tags.clone(),
            published: dropbox_post.metadata.published,
            featured: false, // Default to false
            author: dropbox_post.metadata.author.clone(),
            dropbox_path: dropbox_post.dropbox_path.clone(),
        };

        match database.create_post(create_post).await {
            Ok(post) => {
                info!("✅ Successfully synced '{}' (ID: {})", post.title, post.id);
                synced_count += 1;
            }
            Err(e) => {
                error!("❌ Failed to sync '{}': {}", dropbox_post.metadata.title, e);
            }
        }
    }

    info!("🎉 Synchronization completed!");
    info!("📊 Summary:");
    info!("  - Synced: {} posts", synced_count);
    info!("  - Skipped: {} posts", skipped_count);
    info!("  - Total in Dropbox: {} posts", total_dropbox_posts);

    if synced_count > 0 {
        info!("🌐 You can now view all posts at: http://localhost:3000/");
        info!("📖 API endpoint: http://localhost:3000/api/posts");
    }

    Ok(())
}