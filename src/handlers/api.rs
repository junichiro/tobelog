use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{debug, error};

use crate::models::{
    response::{PostListResponse, PostResponse, PostSummary, ErrorResponse, 
              BlogStatsResponse, CategoryInfo, TagInfo},
    PostFilters
};
use crate::services::DatabaseService;

/// Query parameters for post listing API
#[derive(Debug, Deserialize)]
pub struct ApiPostQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub featured: Option<bool>,
    pub published: Option<bool>,
}

/// App state for API handlers
#[derive(Clone)]
pub struct ApiState {
    pub database: DatabaseService,
}

/// GET /api/posts - List posts with pagination and filtering
pub async fn list_posts_api(
    Query(query): Query<ApiPostQuery>,
    State(state): State<ApiState>
) -> Result<Json<PostListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Listing posts with query: {:?}", query);

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10).min(100); // Limit to 100 per page
    let offset = (page.saturating_sub(1)) * per_page;

    // Build filters
    let filters = PostFilters {
        published: query.published,
        category: query.category.clone(),
        tag: query.tag.clone(),
        featured: query.featured,
        limit: Some(per_page as i64),
        offset: Some(offset as i64),
        ..Default::default()
    };

    // Get posts from database
    let posts = state.database.list_posts(filters.clone()).await
        .map_err(|e| {
            error!("Database error listing posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load posts"))
            )
        })?;

    // Get total count for pagination using efficient count method
    let count_filters = PostFilters {
        published: query.published,
        category: query.category.clone(),
        tag: query.tag.clone(),
        featured: query.featured,
        ..Default::default()
    };
    
    let total_count = state.database.count_posts(count_filters).await
        .map_err(|e| {
            error!("Database error counting posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to count posts"))
            )
        })?;

    let total = total_count as usize;
    let total_pages = total.div_ceil(per_page);

    // Convert posts to summaries
    let post_summaries: Vec<PostSummary> = posts.into_iter()
        .map(PostSummary::from)
        .collect();

    let response = PostListResponse {
        posts: post_summaries,
        total,
        page,
        per_page,
        total_pages,
    };

    Ok(Json(response))
}

/// GET /api/posts/{slug} - Get individual post by slug
pub async fn get_post_api(
    Path(slug): Path<String>,
    State(state): State<ApiState>
) -> Result<Json<PostResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting post by slug: {}", slug);

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

    let response = PostResponse::from(post);
    Ok(Json(response))
}

/// GET /api/blog/stats - Get blog statistics
pub async fn blog_stats_api(
    State(state): State<ApiState>
) -> Result<Json<BlogStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting blog stats");

    let stats = state.database.get_post_stats().await
        .map_err(|e| {
            error!("Database error getting stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load statistics"))
            )
        })?;

    // Get recent posts for the stats
    let recent_filters = PostFilters {
        published: Some(true),
        limit: Some(5),
        ..Default::default()
    };

    let recent_posts = state.database.list_posts(recent_filters).await
        .map_err(|e| {
            error!("Database error getting recent posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load recent posts"))
            )
        })?;

    let recent_summaries: Vec<PostSummary> = recent_posts.into_iter()
        .map(PostSummary::from)
        .collect();

    // Convert categories
    let categories: Vec<CategoryInfo> = stats.categories.into_iter()
        .map(|cat| CategoryInfo {
            name: cat.name,
            count: cat.count,
        })
        .collect();

    // Convert tags  
    let tags: Vec<TagInfo> = stats.tags.into_iter()
        .map(|tag| TagInfo {
            name: tag.name,
            count: tag.count,
        })
        .collect();

    let response = BlogStatsResponse {
        total_posts: stats.total_posts,
        published_posts: stats.published_posts,
        draft_posts: stats.draft_posts,
        featured_posts: stats.featured_posts,
        categories,
        tags,
        recent_posts: recent_summaries,
    };

    Ok(Json(response))
}

/// GET /api/categories - List all categories
pub async fn list_categories_api(
    State(state): State<ApiState>
) -> Result<Json<Vec<CategoryInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Listing categories");

    let stats = state.database.get_post_stats().await
        .map_err(|e| {
            error!("Database error getting categories: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load categories"))
            )
        })?;

    let categories: Vec<CategoryInfo> = stats.categories.into_iter()
        .map(|cat| CategoryInfo {
            name: cat.name,
            count: cat.count,
        })
        .collect();

    Ok(Json(categories))
}

/// GET /api/tags - List all tags
pub async fn list_tags_api(
    State(state): State<ApiState>
) -> Result<Json<Vec<TagInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Listing tags");

    let stats = state.database.get_post_stats().await
        .map_err(|e| {
            error!("Database error getting tags: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load tags"))
            )
        })?;

    let tags: Vec<TagInfo> = stats.tags.into_iter()
        .map(|tag| TagInfo {
            name: tag.name,
            count: tag.count,
        })
        .collect();

    Ok(Json(tags))
}

/// GET /api/search - Search posts
pub async fn search_posts_api(
    Query(query): Query<SearchQuery>,
    State(state): State<ApiState>
) -> Result<Json<PostListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Searching posts with query: {:?}", query);

    let search_query = query.q.unwrap_or_default();
    if search_query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Search query 'q' parameter is required"))
        ));
    }

    let limit = query.limit.unwrap_or(20).min(100);

    let posts = state.database.search_posts(&search_query, Some(limit as i64)).await
        .map_err(|e| {
            error!("Database error searching posts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Search failed"))
            )
        })?;

    let post_summaries: Vec<PostSummary> = posts.into_iter()
        .map(PostSummary::from)
        .collect();

    let total = post_summaries.len();

    let response = PostListResponse {
        posts: post_summaries,
        total,
        page: 1,
        per_page: limit,
        total_pages: 1, // Search results are not paginated
    };

    Ok(Json(response))
}

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}