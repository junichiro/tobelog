use anyhow::Result;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use tobelog::services::template::{TemplateService, HomePageContext, PostPageContext, PostSummary, PostData};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🎨 Testing Template Engine and Responsive Design...");

    // Test template service initialization
    test_template_service().await?;

    // Test template rendering
    test_template_rendering().await?;

    info!("✅ All template tests completed successfully!");
    Ok(())
}

async fn test_template_service() -> Result<()> {
    info!("🔧 Testing template service initialization...");

    let template_service = TemplateService::new()?;
    info!("✅ Template service initialized successfully");

    // Test that templates directory exists and templates are loaded
    let available_templates = template_service.tera().get_template_names().collect::<Vec<_>>();
    info!("📋 Available templates: {:?}", available_templates);

    // Verify expected templates exist
    let expected_templates = ["base.html", "index.html", "post.html"];
    for template in expected_templates {
        if !available_templates.iter().any(|t| t.ends_with(template)) {
            anyhow::bail!("Template not found: {}", template);
        }
        info!("✅ Template found: {}", template);
    }

    Ok(())
}

async fn test_template_rendering() -> Result<()> {
    info!("🎨 Testing template rendering...");

    let template_service = TemplateService::new()?;

    // Test home page template
    let sample_posts = vec![
        PostSummary {
            id: "test-1".to_string(),
            slug: "sample-post-1".to_string(),
            title: "Sample Post 1".to_string(),
            excerpt: Some("This is a sample excerpt for testing".to_string()),
            category: Some("tech".to_string()),
            tags: vec!["rust".to_string(), "blog".to_string()],
            author: Some("Test Author".to_string()),
            published: true,
            featured: true,
            created_at: chrono::Utc::now(),
            published_at: Some(chrono::Utc::now()),
        },
        PostSummary {
            id: "test-2".to_string(),
            slug: "sample-post-2".to_string(),
            title: "Sample Post 2".to_string(),
            excerpt: Some("Another sample excerpt".to_string()),
            category: Some("design".to_string()),
            tags: vec!["tailwind".to_string(), "css".to_string()],
            author: Some("Test Author".to_string()),
            published: true,
            featured: false,
            created_at: chrono::Utc::now(),
            published_at: Some(chrono::Utc::now()),
        },
    ];

    let home_context = HomePageContext {
        site_title: "Test Blog".to_string(),
        site_description: "A test blog for template verification".to_string(),
        posts: sample_posts,
        blog_stats: None,
    };

    let home_html = template_service.render("index.html", &home_context)?;
    info!("✅ Home page template rendered: {} characters", home_html.len());

    // Verify key elements are present
    anyhow::ensure!(home_html.contains("Test Blog"), "Site title not rendered correctly");
    info!("✅ Site title rendered correctly");
    anyhow::ensure!(home_html.contains("Sample Post 1"), "Post titles not rendered correctly");
    info!("✅ Post titles rendered correctly");
    anyhow::ensure!(home_html.contains("TailwindCSS"), "TailwindCSS not included");
    info!("✅ TailwindCSS included");
    anyhow::ensure!(home_html.contains("dark:"), "Dark mode classes not present");
    info!("✅ Dark mode classes present");

    // Test post page template
    let sample_post = PostData {
        id: "test-post".to_string(),
        slug: "test-post-slug".to_string(),
        title: "Test Post Title".to_string(),
        content: "# Test Content\n\nThis is test content.".to_string(),
        html_content: "<h1>Test Content</h1><p>This is test content.</p>".to_string(),
        excerpt: Some("Test excerpt".to_string()),
        category: Some("tech".to_string()),
        tags: vec!["rust".to_string(), "templates".to_string()],
        author: Some("Test Author".to_string()),
        published: true,
        featured: false,
        created_at: chrono::Utc::now(),
        published_at: Some(chrono::Utc::now()),
    };

    let post_context = PostPageContext {
        site_title: "Test Blog".to_string(),
        site_description: "A test blog".to_string(),
        post: sample_post,
    };

    let post_html = template_service.render("post.html", &post_context)?;
    info!("✅ Post page template rendered: {} characters", post_html.len());

    // Verify key elements are present
    anyhow::ensure!(post_html.contains("Test Post Title"), "Post title not rendered correctly");
    info!("✅ Post title rendered correctly");
    anyhow::ensure!(post_html.contains("<h1>Test Content</h1>"), "HTML content not rendered correctly");
    info!("✅ HTML content rendered correctly");
    anyhow::ensure!(post_html.contains("prose"), "Prose styling classes not present");
    info!("✅ Prose styling classes present");

    info!("🎨 Template rendering tests completed successfully");
    Ok(())
}