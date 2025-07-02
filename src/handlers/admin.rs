use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::models::PostFilters;
use crate::services::{DatabaseService, MarkdownService, TemplateService};

/// Application state for admin handlers
#[derive(Clone)]
pub struct AdminState {
    pub database: DatabaseService,
    pub markdown: MarkdownService,
    pub templates: TemplateService,
}

/// Form data for post creation/editing
#[derive(Debug, Deserialize)]
pub struct PostFormData {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<String>, // Comma-separated tags
    pub published: Option<bool>,
    pub featured: Option<bool>,
}

/// Dashboard statistics
#[derive(Debug, Serialize)]
struct DashboardStats {
    total_posts: i64,
    published_posts: i64,
    draft_posts: i64,
    featured_posts: i64,
}

/// Dashboard context for template rendering
#[derive(Debug, Serialize)]
struct DashboardContext {
    page_title: String,
    stats: DashboardStats,
    recent_posts: Vec<crate::models::Post>,
    draft_posts: Vec<crate::models::Post>,
    categories: Vec<crate::models::CategoryStat>,
    tags: Vec<crate::models::TagStat>,
}

/// Post list context for template rendering
#[derive(Debug, Serialize)]
struct PostListContext {
    page_title: String,
    posts: Vec<crate::models::Post>,
}

/// Post form context for template rendering
#[derive(Debug, Serialize)]
struct PostFormContext {
    page_title: String,
    is_new: bool,
    post: PostFormPost,
}

/// Post data for form rendering
#[derive(Debug, Serialize)]
struct PostFormPost {
    id: Option<uuid::Uuid>,
    slug: Option<String>,
    title: String,
    content: String,
    category: String,
    tags: Vec<String>,
    published: bool,
    featured: bool,
}

/// GET /admin - Admin dashboard
pub async fn dashboard(
    State(state): State<AdminState>,
) -> Result<Html<String>, StatusCode> {
    debug!("Rendering admin dashboard");

    // Get statistics
    let stats = state.database.get_post_stats().await
        .map_err(|e| {
            error!("Failed to get post stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get recent posts
    let recent_filters = PostFilters {
        limit: Some(10),
        ..Default::default()
    };
    
    let recent_posts = state.database.list_posts(recent_filters).await
        .map_err(|e| {
            error!("Failed to get recent posts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get draft posts
    let draft_filters = PostFilters {
        published: Some(false),
        limit: Some(10),
        ..Default::default()
    };
    
    let draft_posts = state.database.list_posts(draft_filters).await
        .map_err(|e| {
            error!("Failed to get draft posts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let dashboard_stats = DashboardStats {
        total_posts: stats.total_posts,
        published_posts: stats.published_posts,
        draft_posts: stats.draft_posts,
        featured_posts: stats.featured_posts,
    };

    let context = DashboardContext {
        page_title: "Admin Dashboard".to_string(),
        stats: dashboard_stats,
        recent_posts,
        draft_posts,
        categories: stats.categories,
        tags: stats.tags,
    };

    let html = state.templates.render("admin/dashboard.html", &context)
        .map_err(|e| {
            error!("Failed to render dashboard template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// GET /admin/posts - List all posts for management
pub async fn posts_list(
    State(state): State<AdminState>,
) -> Result<Html<String>, StatusCode> {
    debug!("Rendering admin posts list");

    let filters = PostFilters {
        limit: Some(100), // Show more posts in admin view
        ..Default::default()
    };

    let posts = state.database.list_posts(filters).await
        .map_err(|e| {
            error!("Failed to list posts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let context = PostListContext {
        page_title: "Manage Posts".to_string(),
        posts,
    };

    let html = state.templates.render("admin/post_list.html", &context)
        .map_err(|e| {
            error!("Failed to render post list template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// GET /admin/new - New post creation form
pub async fn new_post_form(
    State(state): State<AdminState>,
) -> Result<Html<String>, StatusCode> {
    debug!("Rendering new post form");

    let context = PostFormContext {
        page_title: "Create New Post".to_string(),
        is_new: true,
        post: PostFormPost {
            id: None,
            slug: None,
            title: String::new(),
            content: String::new(),
            category: String::new(),
            tags: Vec::new(),
            published: false,
            featured: false,
        },
    };

    let html = state.templates.render("admin/post_form.html", &context)
        .map_err(|e| {
            error!("Failed to render post form template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// GET /admin/edit/{slug} - Edit post form
pub async fn edit_post_form(
    Path(slug): Path<String>,
    State(state): State<AdminState>,
) -> Result<Html<String>, StatusCode> {
    debug!("Rendering edit form for post: {}", slug);

    let post = state.database.get_post_by_slug(&slug).await
        .map_err(|e| {
            error!("Failed to get post {}: {}", slug, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let post = match post {
        Some(p) => p,
        None => {
            error!("Post not found: {}", slug);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Parse tags from JSON string to array
    let tags: Vec<String> = serde_json::from_str(&post.tags).unwrap_or_default();

    let context = PostFormContext {
        page_title: format!("Edit: {}", post.title),
        is_new: false,
        post: PostFormPost {
            id: Some(post.id),
            slug: Some(post.slug.clone()),
            title: post.title.clone(),
            content: post.content.clone(),
            category: post.category.unwrap_or_default(),
            tags,
            published: post.published,
            featured: post.featured,
        },
    };

    let html = state.templates.render("admin/post_form.html", &context)
        .map_err(|e| {
            error!("Failed to render post form template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// GET /admin/preview - Preview post (used by JavaScript for live preview)
pub async fn preview_post(
    State(state): State<AdminState>,
    content: String,
) -> Result<Html<String>, StatusCode> {
    debug!("Generating preview for markdown content");

    let html = state.markdown.markdown_to_html(&content)
        .map_err(|e| {
            error!("Failed to convert markdown to HTML: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// Helper function to parse comma-separated tags
fn parse_tags(tags_string: Option<String>) -> Vec<String> {
    tags_string
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}