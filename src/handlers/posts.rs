use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
};
use serde::Deserialize;
use tracing::{debug, error};

use crate::models::response::ErrorResponse;
use crate::services::{DatabaseService, MarkdownService, TemplateService};
use crate::services::template::{HomePageContext, PostPageContext, CategoryPageContext, TagPageContext, PostSummary, PostData, BlogStats};

/// Query parameters for post listing
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // These fields will be used for pagination/filtering in the future
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
    #[allow(dead_code)] // Will be used for markdown processing in the future
    pub markdown: MarkdownService,
    pub templates: TemplateService,
}

/// GET / - Home page showing recent and featured posts
pub async fn home_page(
    Query(query): Query<PostQuery>,
    State(state): State<AppState>
) -> Result<Html<String>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Loading home page with query: {:?}", query);

    // Get recent posts
    let filters = crate::models::PostFilters {
        published: Some(true),
        limit: Some(10),
        ..Default::default()
    };
    
    let posts = state.database.list_posts(filters).await
        .map_err(|e| {
            error!("Database error loading posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load posts"))
            )
        })?;

    // Get blog stats
    let blog_stats = state.database.get_post_stats().await
        .map_err(|e| {
            error!("Database error loading stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load blog stats"))
            )
        })?;

    // Convert to template data
    let post_summaries: Vec<PostSummary> = posts.into_iter().map(PostSummary::from).collect();
    let template_stats = BlogStats::from(blog_stats);

    let context = HomePageContext {
        site_title: "Tobelog".to_string(),
        site_description: "Personal Blog System built with Rust".to_string(),
        posts: post_summaries,
        blog_stats: Some(template_stats),
    };

    // Render template
    let html = state.templates.render("index.html", &context)
        .map_err(|e| {
            error!("Template rendering error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to render page"))
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

    // Convert to template data
    let post_data = PostData::from(post);

    let context = PostPageContext {
        site_title: "Tobelog".to_string(),
        site_description: "Personal Blog System built with Rust".to_string(),
        post: post_data,
    };

    // Render template
    let html = state.templates.render("post.html", &context)
        .map_err(|e| {
            error!("Template rendering error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to render post"))
            )
        })?;

    Ok(Html(html))
}

/// GET /category/{category} - Category page showing posts in a specific category
pub async fn category_page(
    Path(category): Path<String>,
    Query(query): Query<PostQuery>,
    State(state): State<AppState>
) -> Result<Html<String>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Loading category page for category: {}", category);

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);
    let offset = (page.saturating_sub(1)) * per_page;

    // Get posts in this category
    let filters = crate::models::PostFilters {
        published: Some(true),
        category: Some(category.clone()),
        limit: Some(per_page as i64),
        offset: Some(offset as i64),
        ..Default::default()
    };
    
    let posts = state.database.list_posts(filters.clone()).await
        .map_err(|e| {
            error!("Database error loading posts for category {}: {}", category, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load posts"))
            )
        })?;

    // Get total count for pagination
    let count_filters = crate::models::PostFilters {
        published: Some(true),
        category: Some(category.clone()),
        ..Default::default()
    };
    
    let total_count = state.database.count_posts(count_filters).await
        .map_err(|e| {
            error!("Database error counting posts for category {}: {}", category, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to count posts"))
            )
        })?;

    let total_posts = total_count as usize;
    let total_pages = total_posts.div_ceil(per_page);

    // Convert to template data
    let post_summaries: Vec<PostSummary> = posts.into_iter().map(PostSummary::from).collect();

    let context = CategoryPageContext {
        site_title: "Tobelog".to_string(),
        site_description: "Personal Blog System built with Rust".to_string(),
        category_name: category.clone(),
        posts: post_summaries,
        total_posts,
        page,
        total_pages,
    };

    // Render template
    let html = state.templates.render("category.html", &context)
        .map_err(|e| {
            error!("Template rendering error for category {}: {}", category, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to render page"))
            )
        })?;

    Ok(Html(html))
}

/// GET /tag/{tag} - Tag page showing posts with a specific tag
pub async fn tag_page(
    Path(tag): Path<String>,
    Query(query): Query<PostQuery>,
    State(state): State<AppState>
) -> Result<Html<String>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Loading tag page for tag: {}", tag);

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);
    let offset = (page.saturating_sub(1)) * per_page;

    // Get posts with this tag
    let filters = crate::models::PostFilters {
        published: Some(true),
        tag: Some(tag.clone()),
        limit: Some(per_page as i64),
        offset: Some(offset as i64),
        ..Default::default()
    };
    
    let posts = state.database.list_posts(filters.clone()).await
        .map_err(|e| {
            error!("Database error loading posts for tag {}: {}", tag, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load posts"))
            )
        })?;

    // Get total count for pagination
    let count_filters = crate::models::PostFilters {
        published: Some(true),
        tag: Some(tag.clone()),
        ..Default::default()
    };
    
    let total_count = state.database.count_posts(count_filters).await
        .map_err(|e| {
            error!("Database error counting posts for tag {}: {}", tag, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to count posts"))
            )
        })?;

    let total_posts = total_count as usize;
    let total_pages = total_posts.div_ceil(per_page);

    // Convert to template data
    let post_summaries: Vec<PostSummary> = posts.into_iter().map(PostSummary::from).collect();

    let context = TagPageContext {
        site_title: "Tobelog".to_string(),
        site_description: "Personal Blog System built with Rust".to_string(),
        tag_name: tag.clone(),
        posts: post_summaries,
        total_posts,
        page,
        total_pages,
    };

    // Render template
    let html = state.templates.render("tag.html", &context)
        .map_err(|e| {
            error!("Template rendering error for tag {}: {}", tag, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to render page"))
            )
        })?;

    Ok(Html(html))
}