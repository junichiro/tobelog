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
    PostFilters, LLMArticleImportRequest, LLMArticleImportResponse,
    BatchImportRequest, BatchImportResponse, CreatePost
};
use crate::services::{DatabaseService, LLMImportService, BlogStorageService};

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
    pub llm_import: LLMImportService,
    pub blog_storage: BlogStorageService,
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

/// POST /api/posts - Create a new post
pub async fn create_post_api(
    State(state): State<ApiState>,
    Json(create_data): Json<CreatePost>,
) -> Result<Json<PostResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Creating new post with slug: {}", create_data.slug);

    // Check if slug already exists
    if state.database.get_post_by_slug(&create_data.slug).await
        .map_err(|e| {
            error!("Database error checking slug {}: {}", create_data.slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Database error"))
            )
        })?.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::bad_request(format!("Slug '{}' already exists", create_data.slug)))
        ));
    }

    let post = state.database.create_post(create_data).await
        .map_err(|e| {
            error!("Database error creating post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to create post"))
            )
        })?;

    let response = PostResponse::from(post);
    Ok(Json(response))
}

/// PUT /api/posts/{slug} - Update an existing post
pub async fn update_post_api(
    Path(slug): Path<String>,
    State(state): State<ApiState>,
    Json(update_data): Json<crate::models::UpdatePost>,
) -> Result<Json<PostResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Updating post with slug: {}", slug);

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

    let updated_post = state.database.update_post(post.id, update_data).await
        .map_err(|e| {
            error!("Database error updating post {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to update post"))
            )
        })?;

    let updated_post = match updated_post {
        Some(post) => post,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to update post"))
            ));
        }
    };

    let response = PostResponse::from(updated_post);
    Ok(Json(response))
}

/// POST /api/import/llm-article - Import a single LLM-generated article
pub async fn import_llm_article_api(
    State(state): State<ApiState>,
    Json(request): Json<LLMArticleImportRequest>,
) -> Result<Json<LLMArticleImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Importing LLM article from source: {}", request.source);

    if request.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Content cannot be empty"))
        ));
    }

    let import_response = state.llm_import.process_single_article(request.clone()).await
        .map_err(|e| {
            error!("LLM import error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to process article"))
            )
        })?;

    // Optionally save to database if requested
    if request.published.unwrap_or(false) {
        if let Err(e) = state.llm_import.save_imported_article(
            import_response.clone(),
            true
        ).await {
            error!("Failed to save imported article: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to save article"))
            ));
        }
    }

    Ok(Json(import_response))
}

/// POST /api/import/batch - Import multiple articles in batch
pub async fn batch_import_api(
    State(state): State<ApiState>,
    Json(request): Json<BatchImportRequest>,
) -> Result<Json<BatchImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Batch importing {} articles", request.articles.len());

    if request.articles.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("No articles provided for import"))
        ));
    }

    if request.articles.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Too many articles (max 50 per batch)"))
        ));
    }

    let batch_response = state.llm_import.process_batch_import(request).await;

    Ok(Json(batch_response))
}

/// POST /api/posts/{slug}/save - Save a processed LLM article to database
pub async fn save_llm_article_api(
    Path(slug): Path<String>,
    State(state): State<ApiState>,
    Json(save_request): Json<SaveLLMArticleRequest>,
) -> Result<Json<PostResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Saving LLM article with slug: {}", slug);

    // Check if article already exists
    if state.database.get_post_by_slug(&slug).await
        .map_err(|e| {
            error!("Database error checking slug {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Database error"))
            )
        })?.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::bad_request(format!("Article with slug '{}' already exists", slug)))
        ));
    }

    let create_post = CreatePost {
        slug: slug.clone(),
        title: save_request.title,
        content: save_request.content,
        html_content: save_request.html_content,
        excerpt: save_request.excerpt,
        category: save_request.category,
        tags: save_request.tags,
        published: save_request.published,
        featured: save_request.featured,
        author: save_request.author,
        dropbox_path: save_request.dropbox_path,
    };

    let post = state.database.create_post(create_post).await
        .map_err(|e| {
            error!("Database error creating post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to save article"))
            )
        })?;

    let response = PostResponse::from(post);
    Ok(Json(response))
}

/// Request for saving LLM article
#[derive(Debug, Deserialize)]
pub struct SaveLLMArticleRequest {
    pub title: String,
    pub content: String,
    pub html_content: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub published: bool,
    pub featured: bool,
    pub author: Option<String>,
    pub dropbox_path: String,
}