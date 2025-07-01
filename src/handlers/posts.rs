use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
};
use html_escape::encode_text;
use serde::Deserialize;
use tracing::{debug, error};

use crate::models::response::ErrorResponse;
use crate::services::{DatabaseService, MarkdownService};

/// Query parameters for post listing
#[derive(Debug, Deserialize)]
pub struct PostQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub featured: Option<bool>,
}

/// App state for handlers
#[derive(Clone)]
pub struct AppState {
    pub database: DatabaseService,
    pub markdown: MarkdownService,
}

/// GET / - Home page showing recent and featured posts
pub async fn home_page(
    Query(_query): Query<PostQuery>,
    State(_state): State<AppState>
) -> Result<Html<String>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Loading home page");

    // For now, return a simple HTML response
    // In a real implementation, this would use a template engine like Tera
    let html = generate_home_html().await
        .map_err(|e| {
            error!("Failed to generate home page: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(format!("Failed to load home page: {}", e)))
            )
        })?;

    Ok(Html(html))
}

/// GET /posts/{year}/{slug} - Individual post page
pub async fn post_page(
    Path((year, slug)): Path<(String, String)>,
    State(state): State<AppState>
) -> Result<Html<String>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Loading post page for {}/{}", year, slug);

    // Get post by slug
    let post = state.database.get_post_by_slug(&slug).await
        .map_err(|e| {
            error!("Database error getting post {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Database error"))
            )
        })?;

    let post = match post {
        Some(post) => post,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found(format!("Post '{}' not found", slug)))
            ));
        }
    };

    // Check if the year in URL matches the post's year
    let post_year = post.created_at.format("%Y").to_string();
    if year != post_year {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(format!("Post '{}' not found in year {}", slug, year)))
        ));
    }

    // Only show published posts
    if !post.published {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(format!("Post '{}' not found", slug)))
        ));
    }

    // Generate HTML for the post
    let html = generate_post_html(&post).await
        .map_err(|e| {
            error!("Failed to generate post HTML: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to render post"))
            )
        })?;

    Ok(Html(html))
}

/// Generate home page HTML
async fn generate_home_html() -> anyhow::Result<String> {
    // In a real implementation, this would use Tera templates
    let html = r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tobelog - Personal Blog</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            line-height: 1.6;
            color: #333;
        }
        .header {
            border-bottom: 1px solid #eee;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }
        .post-card {
            border: 1px solid #eee;
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
            background: #fff;
        }
        .post-title {
            margin: 0 0 10px 0;
            color: #2563eb;
        }
        .post-meta {
            color: #666;
            font-size: 0.9em;
            margin-bottom: 10px;
        }
        .post-excerpt {
            margin: 10px 0;
        }
        .featured {
            border-left: 4px solid #f59e0b;
        }
        .nav {
            margin-bottom: 30px;
        }
        .nav a {
            color: #2563eb;
            text-decoration: none;
            margin-right: 20px;
        }
        .nav a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <header class="header">
        <h1>Tobelog</h1>
        <p>Personal Blog System built with Rust</p>
    </header>

    <nav class="nav">
        <a href="/">Home</a>
        <a href="/api/posts">API</a>
        <a href="/api/blog/stats">Stats</a>
    </nav>

    <main>
        <section>
            <h2>Recent Posts</h2>
            <div class="post-card">
                <h3 class="post-title">Welcome to Tobelog</h3>
                <div class="post-meta">Published on 2024-01-01 | Category: Tech</div>
                <div class="post-excerpt">
                    This is a sample post to demonstrate the blog system. 
                    The blog is built with Rust using Axum web framework and Dropbox for storage.
                </div>
                <a href="/posts/2024/welcome-to-tobelog">Read more →</a>
            </div>

            <div class="post-card featured">
                <h3 class="post-title">Building a Blog with Rust and Dropbox</h3>
                <div class="post-meta">Published on 2024-01-02 | Category: Tech | Featured</div>
                <div class="post-excerpt">
                    Learn how to build a personal blog system using Rust, Axum, and Dropbox API.
                    This post covers the architecture and implementation details.
                </div>
                <a href="/posts/2024/building-blog-rust-dropbox">Read more →</a>
            </div>
        </section>

        <aside>
            <h3>Quick Stats</h3>
            <p>Posts are loaded dynamically from the database. Visit <a href="/api/posts">/api/posts</a> to see available posts.</p>
        </aside>
    </main>

    <footer style="margin-top: 40px; padding-top: 20px; border-top: 1px solid #eee; text-align: center; color: #666;">
        <p>Powered by Tobelog - A Rust-based blog system</p>
    </footer>
</body>
</html>
"#;

    Ok(html.to_string())
}

/// Generate individual post page HTML
async fn generate_post_html(post: &crate::models::Post) -> anyhow::Result<String> {
    let tags_html = if post.get_tags().is_empty() {
        String::new()
    } else {
        let escaped_tags: Vec<String> = post.get_tags()
            .iter()
            .map(|tag| encode_text(tag).to_string())
            .collect();
        format!(
            "<div class=\"tags\">Tags: {}</div>",
            escaped_tags.join(", ")
        )
    };

    let category_html = post.category.as_ref()
        .map(|cat| format!("<div class=\"category\">Category: {}</div>", encode_text(cat)))
        .unwrap_or_default();

    let author_html = post.author.as_ref()
        .map(|author| format!(" by {}", encode_text(author)))
        .unwrap_or_default();

    let published_date = post.published_at
        .unwrap_or(post.created_at)
        .format("%B %d, %Y")
        .to_string();

    let html = format!(r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Tobelog</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            line-height: 1.6;
            color: #333;
        }}
        .header {{
            border-bottom: 1px solid #eee;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }}
        .post-header {{
            margin-bottom: 30px;
        }}
        .post-title {{
            margin: 0 0 10px 0;
            color: #1a202c;
            font-size: 2.5em;
            font-weight: 700;
        }}
        .post-meta {{
            color: #666;
            font-size: 0.9em;
            margin-bottom: 20px;
        }}
        .post-content {{
            font-size: 1.1em;
            line-height: 1.8;
        }}
        .post-content h1, .post-content h2, .post-content h3 {{
            color: #2d3748;
            margin-top: 2em;
            margin-bottom: 1em;
        }}
        .post-content code {{
            background: #f7fafc;
            padding: 2px 4px;
            border-radius: 4px;
            font-size: 0.9em;
        }}
        .post-content pre {{
            background: #f7fafc;
            padding: 1em;
            border-radius: 8px;
            overflow-x: auto;
        }}
        .post-content blockquote {{
            border-left: 4px solid #e2e8f0;
            padding-left: 1em;
            margin: 1.5em 0;
            color: #4a5568;
        }}
        .tags, .category {{
            margin: 10px 0;
            color: #666;
            font-size: 0.9em;
        }}
        .nav {{
            margin-bottom: 30px;
        }}
        .nav a {{
            color: #2563eb;
            text-decoration: none;
            margin-right: 20px;
        }}
        .nav a:hover {{
            text-decoration: underline;
        }}
        .back-link {{
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #eee;
        }}
    </style>
</head>
<body>
    <header class="header">
        <h1><a href="/" style="color: inherit; text-decoration: none;">Tobelog</a></h1>
        <p>Personal Blog System</p>
    </header>

    <nav class="nav">
        <a href="/">← Back to Home</a>
        <a href="/api/posts">API</a>
    </nav>

    <article>
        <header class="post-header">
            <h1 class="post-title">{title}</h1>
            <div class="post-meta">
                Published on {published_date}{author}
            </div>
            {category}
            {tags}
        </header>

        <div class="post-content">
            {content}
        </div>
    </article>

    <div class="back-link">
        <a href="/">← Back to all posts</a>
    </div>

    <footer style="margin-top: 40px; padding-top: 20px; border-top: 1px solid #eee; text-align: center; color: #666;">
        <p>Powered by Tobelog</p>
    </footer>
</body>
</html>
"#,
        title = encode_text(&post.title),
        published_date = published_date,
        author = author_html,
        category = category_html,
        tags = tags_html,
        content = post.html_content
    );

    Ok(html)
}