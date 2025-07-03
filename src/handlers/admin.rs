use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    Form,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::models::{PostFilters, LLMArticleImportRequest, response::{PostResponse, PostSummary}};
use crate::services::{DatabaseService, MarkdownService, TemplateService, LLMImportService};

/// Application state for admin handlers
#[derive(Clone)]
pub struct AdminState {
    pub database: DatabaseService,
    pub markdown: MarkdownService,
    pub templates: TemplateService,
    pub llm_import: LLMImportService,
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
pub async fn dashboard(State(state): State<AdminState>) -> Result<Html<String>, StatusCode> {
    debug!("Rendering admin dashboard");

    // Get statistics
    let stats = state.database.get_post_stats().await.map_err(|e| {
        error!("Failed to get post stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get recent posts
    let recent_filters = PostFilters {
        limit: Some(10),
        ..Default::default()
    };

    let recent_posts = state
        .database
        .list_posts(recent_filters)
        .await
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

    let draft_posts = state
        .database
        .list_posts(draft_filters)
        .await
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

    let html = state
        .templates
        .render("admin/dashboard.html", &context)
        .map_err(|e| {
            error!("Failed to render dashboard template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// GET /admin/posts - List all posts for management
pub async fn posts_list(State(state): State<AdminState>) -> Result<Html<String>, StatusCode> {
    debug!("Rendering admin posts list");

    let filters = PostFilters {
        limit: Some(100), // Show more posts in admin view
        ..Default::default()
    };

    let posts = state.database.list_posts(filters).await.map_err(|e| {
        error!("Failed to list posts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let context = PostListContext {
        page_title: "Manage Posts".to_string(),
        posts,
    };

    let html = state
        .templates
        .render("admin/post_list.html", &context)
        .map_err(|e| {
            error!("Failed to render post list template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// GET /admin/new - New post creation form
pub async fn new_post_form(State(state): State<AdminState>) -> Result<Html<String>, StatusCode> {
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

    let html = state
        .templates
        .render("admin/post_form.html", &context)
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

    let post = state.database.get_post_by_slug(&slug).await.map_err(|e| {
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

    let html = state
        .templates
        .render("admin/post_form.html", &context)
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

    let html = state.markdown.markdown_to_html(&content).map_err(|e| {
        error!("Failed to convert markdown to HTML: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Html(html))
}

/// GET /admin/llm-import - LLM article import form
pub async fn llm_import_form(State(state): State<AdminState>) -> Result<Html<String>, StatusCode> {
    debug!("Rendering LLM import form");

    // Create context for the template
    #[derive(Serialize)]
    struct LLMImportContext {
        page_title: String,
    }

    let context = LLMImportContext {
        page_title: "LLM Article Import".to_string(),
    };

    let html = state
        .templates
        .render("admin/llm_import.html", &context)
        .map_err(|e| {
            error!("Failed to render LLM import template: {}", e);
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
pub async fn admin_import_page(
    State(state): State<AdminState>
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    debug!("Admin: Loading import page");

    let context = AdminImportContext {
        page_title: "LLM記事インポート".to_string(),
    };

    let html = state.templates.render("admin/import.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

/// POST /admin/import - Process LLM article import
pub async fn admin_process_import(
    State(state): State<AdminState>,
    Form(form_data): Form<ImportFormData>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    debug!("Admin: Processing import for source: {}", form_data.source);

    if form_data.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("コンテンツが空です".to_string())
        ));
    }

    // Create import request
    let import_request = LLMArticleImportRequest {
        content: form_data.content.clone(),
        suggested_title: if form_data.title.trim().is_empty() { 
            None 
        } else { 
            Some(form_data.title.clone()) 
        },
        category_hint: if form_data.category.trim().is_empty() { 
            None 
        } else { 
            Some(form_data.category.clone()) 
        },
        tags_hint: if form_data.tags.trim().is_empty() {
            None
        } else {
            Some(form_data.tags.split(',').map(|s| s.trim().to_string()).collect())
        },
        source: form_data.source.clone(),
        published: Some(form_data.published),
        featured: Some(form_data.featured),
    };

    // Process the import
    let import_response = state.llm_import.process_single_article(import_request).await
        .map_err(|e| {
            error!("LLM import error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("インポートエラー: {}", e))
            )
        })?;

    // Save to database if requested
    if form_data.published {
        if let Err(e) = state.llm_import.save_imported_article(
            import_response.clone(),
            true
        ).await {
            error!("Failed to save imported article: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("保存エラー: {}", e))
            ));
        }
    }

    let context = ImportResultContext {
        success: true,
        slug: import_response.slug,
        title: import_response.suggested_metadata.title,
        preview_url: import_response.preview_url,
        formatted_content: import_response.formatted_content,
        suggested_category: import_response.suggested_metadata.category.unwrap_or_default(),
        suggested_tags: import_response.suggested_metadata.tags.join(", "),
        saved_to_db: form_data.published,
    };

    let html = state.templates.render("admin/import_result.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

/// GET /admin/posts/{slug}/edit - Edit post page with LLM support
pub async fn admin_edit_post_page(
    Path(slug): Path<String>,
    State(state): State<AdminState>
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    debug!("Admin: Loading edit page for post: {}", slug);

    let post = state.database.get_post_by_slug(&slug).await
        .map_err(|e| {
            error!("Database error getting post {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database error".to_string())
            )
        })?;

    let post = match post {
        Some(post) => post,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Html(format!("記事 '{}' が見つかりません", slug))
            ));
        }
    };

    let context = AdminEditPostContext {
        post: PostResponse::from(post),
    };

    let html = state.templates.render("admin/edit_post.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

/// GET /admin/posts - Post management page with enhanced features
pub async fn admin_posts_page(
    Query(query): Query<AdminPostsQuery>,
    State(state): State<AdminState>
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    debug!("Admin: Loading posts management page");

    let page = query.page.unwrap_or(1);
    let per_page = 20;
    let offset = (page.saturating_sub(1)) * per_page;

    let filters = PostFilters {
        published: query.published,
        category: query.category.clone(),
        search: query.search.clone(),
        limit: Some(per_page as i64),
        offset: Some(offset as i64),
        ..Default::default()
    };

    let posts = state.database.list_posts(filters.clone()).await
        .map_err(|e| {
            error!("Database error listing posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database error".to_string())
            )
        })?;

    let count_filters = PostFilters {
        published: query.published,
        category: query.category.clone(),
        search: query.search.clone(),
        ..Default::default()
    };

    let total_count = state.database.count_posts(count_filters).await
        .map_err(|e| {
            error!("Database error counting posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database error".to_string())
            )
        })?;

    let total = total_count as usize;
    let total_pages = total.div_ceil(per_page);

    let post_summaries: Vec<PostSummary> = posts.into_iter()
        .map(PostSummary::from)
        .collect();

    let context = AdminPostsContext {
        posts: post_summaries,
        current_page: page,
        total_pages,
        total_posts: total,
        search_query: query.search.unwrap_or_default(),
        filter_published: query.published,
        filter_category: query.category.unwrap_or_default(),
    };

    let html = state.templates.render("admin/posts.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

// Context structures for LLM templates
#[derive(Serialize)]
struct AdminImportContext {
    page_title: String,
}

#[derive(Serialize)]
struct ImportResultContext {
    success: bool,
    slug: String,
    title: String,
    preview_url: String,
    formatted_content: String,
    suggested_category: String,
    suggested_tags: String,
    saved_to_db: bool,
}

#[derive(Serialize)]
struct AdminPostsContext {
    posts: Vec<PostSummary>,
    current_page: usize,
    total_pages: usize,
    total_posts: usize,
    search_query: String,
    filter_published: Option<bool>,
    filter_category: String,
}

#[derive(Serialize)]
struct AdminEditPostContext {
    post: PostResponse,
}

// Form data structures for LLM import
#[derive(Debug, Deserialize)]
pub struct ImportFormData {
    pub content: String,
    pub title: String,
    pub category: String,
    pub tags: String,
    pub source: String,
    pub published: bool,
    pub featured: bool,
}

#[derive(Debug, Deserialize)]
pub struct AdminPostsQuery {
    pub page: Option<usize>,
    pub published: Option<bool>,
    pub category: Option<String>,
    pub search: Option<String>,
}
