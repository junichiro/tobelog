use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tobelog::services::blog_storage::{BlogPost, BlogPostMetadata};
use tobelog::{BlogStorageService, Config, DropboxClient};
use tracing::{error, info, warn, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    dotenv::dotenv().ok();

    info!("🧪 Testing Blog Storage Service...");

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            eprintln!("Make sure DROPBOX_ACCESS_TOKEN is set in your .env file");
            eprintln!("Run ./scripts/setup_dropbox.sh for setup instructions");
            std::process::exit(1);
        }
    };

    let dropbox_client = Arc::new(DropboxClient::new(config.dropbox_access_token));
    let blog_storage = BlogStorageService::new(dropbox_client.clone());

    // Test connection first
    info!("🔗 Testing Dropbox connection...");
    match dropbox_client.test_connection().await {
        Ok(account_info) => {
            info!("✅ Connected to Dropbox");
            if let Some(name) = account_info.get("name") {
                if let Some(display_name) = name.get("display_name") {
                    info!("👤 Account: {}", display_name);
                }
            }
        }
        Err(e) => {
            error!("❌ Connection failed: {}", e);
            std::process::exit(1);
        }
    }

    // Initialize blog structure
    info!("🏗️  Initializing blog folder structure...");
    match blog_storage.initialize_blog_structure().await {
        Ok(_) => {
            info!("✅ Blog structure initialized");
        }
        Err(e) => {
            warn!("⚠️  Failed to initialize blog structure: {}", e);
        }
    }

    // Test creating a sample blog post
    info!("📝 Creating a test blog post...");
    let test_post = BlogPost {
        metadata: BlogPostMetadata {
            title: "Test Blog Post".to_string(),
            slug: "test-blog-post".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            category: Some("testing".to_string()),
            tags: vec!["test".to_string(), "blog".to_string(), "rust".to_string()],
            published: false, // Start as draft
            author: Some("Tobelog Test".to_string()),
            excerpt: Some(
                "This is a test blog post to verify the blog storage service.".to_string(),
            ),
        },
        content: r#"# Test Blog Post

This is a test blog post created by the blog storage service test.

## Features Tested

- Blog post creation
- Markdown content handling
- YAML frontmatter parsing
- Dropbox integration

## Code Example

```rust
fn hello_world() {
    println!("Hello, world!");
}
```

This post demonstrates that the blog storage service is working correctly!"#
            .to_string(),
        dropbox_path: String::new(), // Will be set when saved
        file_metadata: None,
    };

    // Save as draft
    match blog_storage.save_post(&test_post, true).await {
        Ok(_) => {
            info!("✅ Test post saved as draft");
        }
        Err(e) => {
            error!("❌ Failed to save test post: {}", e);
        }
    }

    // List drafts
    info!("📄 Listing draft posts...");
    match blog_storage.list_draft_posts().await {
        Ok(drafts) => {
            info!("✅ Found {} draft posts", drafts.len());
            for draft in &drafts {
                info!("  📝 {}: {}", draft.metadata.slug, draft.metadata.title);
            }
        }
        Err(e) => {
            error!("❌ Failed to list drafts: {}", e);
        }
    }

    // Test getting post by slug
    info!("🔍 Getting test post by slug...");
    match blog_storage.get_post_by_slug("test-blog-post").await {
        Ok(Some(post)) => {
            info!("✅ Found post: {}", post.metadata.title);
            info!(
                "  📅 Created: {}",
                post.metadata.created_at.format("%Y-%m-%d %H:%M")
            );
            info!("  🏷️  Tags: {}", post.metadata.tags.join(", "));
            info!("  📊 Published: {}", post.metadata.published);
        }
        Ok(None) => {
            warn!("⚠️  Post not found");
        }
        Err(e) => {
            error!("❌ Failed to get post: {}", e);
        }
    }

    // Test publishing the post
    info!("📤 Publishing test post...");
    match blog_storage.publish_post("test-blog-post").await {
        Ok(true) => {
            info!("✅ Post published successfully");
        }
        Ok(false) => {
            warn!("⚠️  Failed to publish post");
        }
        Err(e) => {
            error!("❌ Error publishing post: {}", e);
        }
    }

    // List published posts
    info!("📰 Listing published posts...");
    match blog_storage.list_published_posts().await {
        Ok(posts) => {
            info!("✅ Found {} published posts", posts.len());
            for post in &posts {
                info!("  📰 {}: {}", post.metadata.slug, post.metadata.title);
            }
        }
        Err(e) => {
            error!("❌ Failed to list published posts: {}", e);
        }
    }

    // Get blog statistics
    info!("📊 Getting blog statistics...");
    match blog_storage.get_blog_stats().await {
        Ok(stats) => {
            info!("✅ Blog statistics:");
            if let Some(published) = stats.get("published_posts") {
                info!("  📰 Published posts: {}", published);
            }
            if let Some(drafts) = stats.get("draft_posts") {
                info!("  📝 Draft posts: {}", drafts);
            }
            if let Some(categories) = stats.get("categories") {
                info!(
                    "  🗂️  Categories: {}",
                    serde_json::to_string_pretty(categories)?
                );
            }
            if let Some(tags) = stats.get("tags") {
                info!("  🏷️  Tags: {}", serde_json::to_string_pretty(tags)?);
            }
        }
        Err(e) => {
            error!("❌ Failed to get blog stats: {}", e);
        }
    }

    // Clean up - delete the test post
    info!("🧹 Cleaning up test post...");
    match blog_storage.delete_post("test-blog-post").await {
        Ok(true) => {
            info!("✅ Test post deleted successfully");
        }
        Ok(false) => {
            warn!("⚠️  Test post not found for deletion");
        }
        Err(e) => {
            error!("❌ Failed to delete test post: {}", e);
        }
    }

    info!("🎉 Blog storage service test completed!");
    info!("🚀 Your blog storage integration is ready!");

    Ok(())
}
