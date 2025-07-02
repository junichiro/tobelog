use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    Form,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::models::{
    response::{PostResponse, PostSummary},
    PostFilters, LLMArticleImportRequest
};
use crate::services::{DatabaseService, TemplateService, LLMImportService};

/// Admin app state
#[derive(Clone)]
pub struct AdminState {
    pub database: DatabaseService,
    pub template: TemplateService,
    pub llm_import: LLMImportService,
}

/// GET /admin - Admin dashboard
pub async fn admin_dashboard(
    State(state): State<AdminState>
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    debug!("Admin: Loading dashboard");

    // Get basic statistics
    let stats = state.database.get_post_stats().await
        .map_err(|e| {
            error!("Database error getting stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Internal server error".to_string())
            )
        })?;

    // Get recent posts
    let recent_filters = PostFilters {
        published: None,
        limit: Some(10),
        ..Default::default()
    };

    let recent_posts = state.database.list_posts(recent_filters).await
        .map_err(|e| {
            error!("Database error getting recent posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Internal server error".to_string())
            )
        })?;

    let recent_summaries: Vec<PostSummary> = recent_posts.into_iter()
        .map(PostSummary::from)
        .collect();

    let context = AdminDashboardContext {
        total_posts: stats.total_posts,
        published_posts: stats.published_posts,
        draft_posts: stats.draft_posts,
        recent_posts: recent_summaries,
    };

    let html = state.template.render("admin/dashboard.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

/// GET /admin/import - LLM article import page
pub async fn admin_import_page(
    State(state): State<AdminState>
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    debug!("Admin: Loading import page");

    let context = AdminImportContext {
        page_title: "LLM記事インポート".to_string(),
    };

    let html = state.template.render("admin/import.html", &context)
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

    let html = state.template.render("admin/import_result.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

/// GET /admin/posts - Post management page
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

    let html = state.template.render("admin/posts.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

/// GET /admin/posts/{slug}/edit - Edit post page
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

    let html = state.template.render("admin/edit_post.html", &context)
        .map_err(|e| {
            error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Template error".to_string())
            )
        })?;

    Ok(Html(html))
}

// Context structures for templates
#[derive(Serialize)]
struct AdminDashboardContext {
    total_posts: i64,
    published_posts: i64,
    draft_posts: i64,
    recent_posts: Vec<PostSummary>,
}

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

// Form data structures
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