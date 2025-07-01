use anyhow::Result;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use tobelog::models::{CreatePost, PostFilters, UpdatePost};
use tobelog::services::{DatabaseService, MarkdownService};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🧪 Testing Markdown and Database functionality...");

    // Test markdown processing
    test_markdown_processing().await?;

    // Test database operations
    test_database_operations().await?;

    info!("✅ All tests completed successfully!");
    Ok(())
}

async fn test_markdown_processing() -> Result<()> {
    info!("📝 Testing Markdown processing...");

    let markdown_service = MarkdownService::new();

    let test_content = r#"---
title: "My First Blog Post"
category: "tech"
tags: ["rust", "blog", "markdown"]
published: true
author: "Test Author"
excerpt: "This is a test post to demonstrate markdown processing."
---

# Welcome to My Blog

This is my **first blog post** written in *Markdown*.

## Features

- Markdown to HTML conversion
- YAML frontmatter parsing
- Metadata extraction

```rust
fn main() {
    println!("Hello, world!");
}
```

> This is a blockquote with some important information.

### List Example

1. First item
2. Second item
3. Third item

That's all for now!"#;

    let parsed = markdown_service.parse_markdown(test_content)?;
    
    info!("Parsed frontmatter fields: {}", parsed.frontmatter.len());
    info!("Generated HTML length: {} bytes", parsed.html.len());
    
    // Test field extraction
    let title = markdown_service.extract_title(&parsed.frontmatter, &parsed.content);
    let tags = markdown_service.extract_tags(&parsed.frontmatter);
    let category = markdown_service.extract_category(&parsed.frontmatter);
    let published = markdown_service.extract_published(&parsed.frontmatter);
    
    info!("Extracted title: {}", title);
    info!("Extracted tags: {:?}", tags);
    info!("Extracted category: {:?}", category);
    info!("Published status: {}", published);

    // Verify HTML contains expected elements
    assert!(parsed.html.contains("<h1>Welcome to My Blog</h1>"));
    assert!(parsed.html.contains("<strong>first blog post</strong>"));
    assert!(parsed.html.contains("rust"));
    assert!(parsed.html.contains("<blockquote>"));
    
    info!("Generated HTML:\n{}", parsed.html);

    info!("✅ Markdown processing tests passed!");
    Ok(())
}

async fn test_database_operations() -> Result<()> {
    info!("🗃️ Testing Database operations...");

    // Use in-memory SQLite for testing
    let db_service = DatabaseService::new("sqlite::memory:").await?;

    // Create test post
    let create_data = CreatePost {
        slug: "test-post-1".to_string(),
        title: "Test Post 1".to_string(),
        content: "# Test Content\n\nThis is test content.".to_string(),
        html_content: "<h1>Test Content</h1><p>This is test content.</p>".to_string(),
        excerpt: Some("This is test content.".to_string()),
        category: Some("tech".to_string()),
        tags: vec!["rust".to_string(), "test".to_string()],
        published: true,
        featured: false,
        author: Some("Test Author".to_string()),
        dropbox_path: "/BlogStorage/posts/2024/test-post-1.md".to_string(),
    };

    let post = db_service.create_post(create_data).await?;
    info!("Created post with ID: {}", post.id);

    // Test retrieval by slug
    let retrieved_post = db_service.get_post_by_slug("test-post-1").await?;
    assert!(retrieved_post.is_some());
    let retrieved_post = retrieved_post.unwrap();
    assert_eq!(retrieved_post.title, "Test Post 1");
    assert_eq!(retrieved_post.get_tags(), vec!["rust", "test"]);

    // Test retrieval by ID
    let retrieved_by_id = db_service.get_post_by_id(post.id).await?;
    assert!(retrieved_by_id.is_some());

    // Create another post for filtering tests
    let create_data_2 = CreatePost {
        slug: "test-post-2".to_string(),
        title: "Test Post 2".to_string(),
        content: "# Another Test\n\nDraft content.".to_string(),
        html_content: "<h1>Another Test</h1><p>Draft content.</p>".to_string(),
        excerpt: None,
        category: Some("blog".to_string()),
        tags: vec!["draft".to_string()],
        published: false,
        featured: true,
        author: Some("Another Author".to_string()),
        dropbox_path: "/BlogStorage/drafts/test-post-2.md".to_string(),
    };

    let post_2 = db_service.create_post(create_data_2).await?;
    info!("Created second post with ID: {}", post_2.id);

    // Test filtering
    let published_filter = PostFilters {
        published: Some(true),
        ..Default::default()
    };
    let published_posts = db_service.list_posts(published_filter).await?;
    info!("Published posts found: {}", published_posts.len());
    for post in &published_posts {
        info!("Published post: {} - published: {}", post.title, post.published);
    }
    assert_eq!(published_posts.len(), 1);
    assert_eq!(published_posts[0].slug, "test-post-1");

    let draft_filter = PostFilters {
        published: Some(false),
        ..Default::default()
    };
    let draft_posts = db_service.list_posts(draft_filter).await?;
    assert_eq!(draft_posts.len(), 1);
    assert_eq!(draft_posts[0].slug, "test-post-2");

    let category_filter = PostFilters {
        category: Some("tech".to_string()),
        ..Default::default()
    };
    let tech_posts = db_service.list_posts(category_filter).await?;
    info!("Tech posts found: {}", tech_posts.len());
    for post in &tech_posts {
        info!("Tech post: {} - category: {:?}", post.title, post.category);
    }
    assert_eq!(tech_posts.len(), 1);

    // Test statistics
    let stats = db_service.get_post_stats().await?;
    info!("Post statistics:");
    info!("  Total posts: {}", stats.total_posts);
    info!("  Published posts: {}", stats.published_posts);
    info!("  Draft posts: {}", stats.draft_posts);
    info!("  Featured posts: {}", stats.featured_posts);
    info!("  Categories: {}", stats.categories.len());

    assert_eq!(stats.total_posts, 2);
    assert_eq!(stats.published_posts, 1);
    assert_eq!(stats.draft_posts, 1);
    assert_eq!(stats.featured_posts, 1);

    // Test update
    let update_data = UpdatePost {
        title: Some("Updated Test Post 1".to_string()),
        published: Some(false),
        ..Default::default()
    };
    let updated_post = db_service.update_post(post.id, update_data).await?;
    assert!(updated_post.is_some());
    let updated_post = updated_post.unwrap();
    assert_eq!(updated_post.title, "Updated Test Post 1");
    assert!(!updated_post.published);
    assert_eq!(updated_post.version, 2);

    // Test deletion
    let deleted = db_service.delete_post(post_2.id).await?;
    assert!(deleted);

    let deleted_post = db_service.get_post_by_id(post_2.id).await?;
    assert!(deleted_post.is_none());

    info!("✅ Database operation tests passed!");
    Ok(())
}

async fn test_integration() -> Result<()> {
    info!("🔄 Testing Markdown + Database integration...");

    let markdown_service = MarkdownService::new();
    let db_service = DatabaseService::new("sqlite::memory:").await?;

    let markdown_content = r#"---
title: "Integration Test Post"
category: "testing"
tags: ["integration", "rust", "sqlite"]
published: true
author: "Integration Tester"
---

# Integration Test

This post demonstrates the integration between **Markdown processing** and *database storage*.

## Code Example

```rust
fn integrate() -> Result<()> {
    // Process markdown
    let parsed = markdown_service.parse(content)?;
    
    // Store in database
    let post = db_service.create_post(parsed)?;
    
    Ok(())
}
```

That's how it works!"#;

    // Process markdown
    let parsed = markdown_service.parse_markdown(markdown_content)?;
    
    // Extract metadata and create post
    let create_data = CreatePost {
        slug: "integration-test".to_string(),
        title: markdown_service.extract_title(&parsed.frontmatter, &parsed.content),
        content: parsed.content,
        html_content: parsed.html,
        excerpt: markdown_service.extract_excerpt(&parsed.frontmatter),
        category: markdown_service.extract_category(&parsed.frontmatter),
        tags: markdown_service.extract_tags(&parsed.frontmatter),
        published: markdown_service.extract_published(&parsed.frontmatter),
        featured: false,
        author: markdown_service.extract_author(&parsed.frontmatter),
        dropbox_path: "/BlogStorage/posts/integration-test.md".to_string(),
    };

    let post = db_service.create_post(create_data).await?;
    
    info!("Created integrated post:");
    info!("  ID: {}", post.id);
    info!("  Title: {}", post.title);
    info!("  Category: {:?}", post.category);
    info!("  Tags: {:?}", post.get_tags());
    info!("  Published: {}", post.published);
    info!("  HTML length: {} bytes", post.html_content.len());

    // Verify the integration worked
    assert_eq!(post.title, "Integration Test Post");
    assert_eq!(post.category, Some("testing".to_string()));
    assert_eq!(post.get_tags(), vec!["integration", "rust", "sqlite"]);
    assert!(post.published);
    assert!(post.html_content.contains("<h1>Integration Test</h1>"));
    assert!(post.html_content.contains("<code>rust"));

    info!("✅ Integration tests passed!");
    Ok(())
}