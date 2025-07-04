use anyhow::Result;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use tobelog::models::CreatePost;
use tobelog::services::{DatabaseService, MarkdownService};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🧪 Testing Web Handlers and Database Integration...");

    // Test database and markdown services
    test_services().await?;

    // Test basic post creation and retrieval
    test_post_creation().await?;

    info!("✅ All web handler tests completed successfully!");
    Ok(())
}

async fn test_services() -> Result<()> {
    info!("📊 Testing service initialization...");

    // Test database service
    let db_service = DatabaseService::new("sqlite::memory:").await?;
    info!("✅ Database service initialized");

    // Test markdown service
    let markdown_service = MarkdownService::new();
    info!("✅ Markdown service initialized");

    // Test cloning (needed for handlers)
    let _db_clone = db_service.clone();
    let _markdown_clone = markdown_service.clone();
    info!("✅ Service cloning works");

    Ok(())
}

async fn test_post_creation() -> Result<()> {
    info!("📝 Testing post creation and retrieval...");

    let db_service = DatabaseService::new("sqlite::memory:").await?;
    let markdown_service = MarkdownService::new();

    // Create a test post
    let markdown_content = r#"---
title: "Web Handler Test Post"
category: "testing"
tags: ["web", "handlers", "rust"]
published: true
author: "Test Author"
excerpt: "A test post for web handlers"
---

# Web Handler Test

This is a test post to verify that the web handlers work correctly.

## Features Tested

- Markdown processing
- Database integration
- Post CRUD operations
- Handler routing

The web handlers should be able to:
1. Display this post on the home page
2. Show individual post pages
3. Serve JSON API responses
4. Handle 404 errors gracefully"#;

    // Process markdown
    let parsed = markdown_service.parse_markdown(markdown_content)?;

    // Create post data
    let create_data = CreatePost {
        slug: "web-handler-test-post".to_string(),
        title: markdown_service.extract_title(&parsed.frontmatter, &parsed.content),
        content: parsed.content,
        html_content: parsed.html,
        excerpt: markdown_service.extract_excerpt(&parsed.frontmatter),
        category: markdown_service.extract_category(&parsed.frontmatter),
        tags: markdown_service.extract_tags(&parsed.frontmatter),
        published: markdown_service.extract_published(&parsed.frontmatter),
        featured: false,
        author: markdown_service.extract_author(&parsed.frontmatter),
        dropbox_path: "/BlogStorage/posts/2024/web-handler-test-post.md".to_string(),
    };

    // Create post in database
    let post = db_service.create_post(create_data).await?;
    info!("✅ Created test post: {}", post.title);

    // Test retrieval by slug
    let retrieved = db_service.get_post_by_slug("web-handler-test-post").await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();

    info!("✅ Retrieved post: {}", retrieved.title);
    info!("   - Category: {:?}", retrieved.category);
    info!("   - Tags: {:?}", retrieved.get_tags());
    info!("   - Published: {}", retrieved.published);
    info!("   - HTML length: {} bytes", retrieved.html_content.len());

    // Test post listing
    let filters = tobelog::models::PostFilters {
        published: Some(true),
        limit: Some(10),
        ..Default::default()
    };

    let posts = db_service.list_posts(filters).await?;
    info!("✅ Listed {} published posts", posts.len());

    // Test statistics
    let stats = db_service.get_post_stats().await?;
    info!("✅ Blog statistics:");
    info!("   - Total posts: {}", stats.total_posts);
    info!("   - Published posts: {}", stats.published_posts);
    info!("   - Categories: {}", stats.categories.len());

    Ok(())
}
