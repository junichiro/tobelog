use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use crate::models::{
    response::{PostListResponse, PostResponse, PostSummary, ErrorResponse, 
              BlogStatsResponse, CategoryInfo, TagInfo},
    PostFilters, CreatePost, UpdatePost
};
use crate::services::{DatabaseService, MarkdownService, BlogStorageService};
use std::sync::Arc;

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
    pub markdown: MarkdownService,
    pub blog_storage: Arc<BlogStorageService>,
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

/// Request body for creating a new post
#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub published: Option<bool>,
    pub featured: Option<bool>,
    pub author: Option<String>,
}

/// Request body for updating a post
#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub published: Option<bool>,
    pub featured: Option<bool>,
    pub author: Option<String>,
}

/// Response for post operations (create, update, delete)
#[derive(Debug, Serialize)]
pub struct PostOperationResponse {
    pub success: bool,
    pub slug: String,
    pub message: String,
    pub post: Option<PostResponse>,
}

/// Request body for Dropbox sync
#[derive(Debug, Deserialize)]
pub struct SyncDropboxRequest {
    pub force: Option<bool>,
}

/// Response for sync operations
#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
    pub synced_count: Option<usize>,
    pub errors: Option<Vec<String>>,
}

/// Request body for markdown import
#[derive(Debug, Deserialize)]
pub struct ImportMarkdownRequest {
    pub files: Vec<MarkdownFileImport>,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MarkdownFileImport {
    pub path: String,
    pub content: String,
    pub metadata: Option<PostMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct PostMetadata {
    pub title: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub published: Option<bool>,
    pub author: Option<String>,
}

/// POST /api/posts - Create a new post
pub async fn create_post_api(
    State(state): State<ApiState>,
    Json(request): Json<CreatePostRequest>
) -> Result<Json<PostOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("API: Creating new post with title: {}", request.title);

    // Validate request
    if request.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Title cannot be empty"))
        ));
    }

    if request.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Content cannot be empty"))
        ));
    }

    // Generate slug from title
    let slug = generate_slug(&request.title);
    
    // Check if slug already exists
    if let Ok(Some(_)) = state.database.get_post_by_slug(&slug).await {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new("conflict", format!("Post with slug '{}' already exists", slug), 409))
        ));
    }

    // Parse markdown content to HTML
    let parsed = state.markdown.parse_markdown(&request.content)
        .map_err(|e| {
            error!("Failed to parse markdown: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to parse markdown"))
            )
        })?;
    let html_content = parsed.html;
    
    // Generate excerpt if not provided
    let excerpt = generate_excerpt(&request.content, 200);

    // Prepare the year-based path
    let now = chrono::Utc::now();
    let year = now.format("%Y");
    let filename = format!("{}.md", slug);
    let dropbox_path = format!("/posts/{}/{}", year, filename);

    // Create post data
    let create_data = CreatePost {
        slug: slug.clone(),
        title: request.title.clone(),
        content: request.content.clone(),
        html_content,
        excerpt: Some(excerpt),
        category: request.category,
        tags: request.tags.unwrap_or_default(),
        published: request.published.unwrap_or(false),
        featured: request.featured.unwrap_or(false),
        author: request.author,
        dropbox_path: dropbox_path.clone(),
    };

    // Save to database first
    let post = state.database.create_post(create_data).await
        .map_err(|e| {
            error!("Database error creating post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to create post"))
            )
        })?;

    // TODO: Save to Dropbox - implement when blog storage service is ready
    // match state.blog_storage.save_post(&post, false).await {
    //     Ok(_) => {
    //         info!("Post saved to Dropbox: {}", dropbox_path);
    //     }
    //     Err(e) => {
    //         error!("Failed to save post to Dropbox: {}", e);
    //         // Don't fail the request, but log the error
    //         // TODO: Consider a background job to retry Dropbox sync
    //     }
    // }

    let response = PostOperationResponse {
        success: true,
        slug: post.slug.clone(),
        message: format!("Post '{}' created successfully", request.title),
        post: Some(PostResponse::from(post)),
    };

    Ok(Json(response))
}

/// PUT /api/posts/{slug} - Update an existing post
pub async fn update_post_api(
    Path(slug): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<UpdatePostRequest>
) -> Result<Json<PostOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("API: Updating post with slug: {}", slug);

    // Get existing post
    let existing_post = state.database.get_post_by_slug(&slug).await
        .map_err(|e| {
            error!("Database error getting post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Database error"))
            )
        })?;

    let mut existing_post = match existing_post {
        Some(post) => post,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found(format!("Post '{}' not found", slug)))
            ));
        }
    };

    // Update HTML content if content is being updated
    let html_content = if let Some(ref content) = request.content {
        let parsed = state.markdown.parse_markdown(content)
            .map_err(|e| {
                error!("Failed to parse markdown: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::internal_error("Failed to parse markdown"))
                )
            })?;
        Some(parsed.html)
    } else {
        None
    };

    // Create update data
    let update_data = UpdatePost {
        title: request.title.clone(),
        content: request.content.clone(),
        html_content,
        excerpt: None, // Keep existing excerpt unless content changes
        category: request.category,
        tags: request.tags,
        published: request.published,
        featured: request.featured,
        author: request.author,
        dropbox_path: None, // Keep existing path
    };

    // Update in database
    let updated_post = state.database.update_post(existing_post.id, update_data).await
        .map_err(|e| {
            error!("Database error updating post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to update post"))
            )
        })?;

    // TODO: Update in Dropbox if content changed - implement when blog storage service is ready
    // if let Some(ref content) = request.content {
    //     match state.blog_storage.save_post(&post, false).await {
    //         Ok(_) => {
    //             info!("Post updated in Dropbox: {}", existing_post.dropbox_path);
    //         }
    //         Err(e) => {
    //             error!("Failed to update post in Dropbox: {}", e);
    //             // Don't fail the request, but log the error
    //         }
    //     }
    // }

    let response = PostOperationResponse {
        success: true,
        slug: updated_post.as_ref().map(|p| p.slug.clone()).unwrap_or_else(|| slug.clone()),
        message: format!("Post '{}' updated successfully", updated_post.as_ref().map(|p| p.title.as_str()).unwrap_or(&slug)),
        post: updated_post.map(PostResponse::from),
    };

    Ok(Json(response))
}

/// DELETE /api/posts/{slug} - Delete a post
pub async fn delete_post_api(
    Path(slug): Path<String>,
    State(state): State<ApiState>
) -> Result<Json<PostOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("API: Deleting post with slug: {}", slug);

    // Get existing post
    let existing_post = state.database.get_post_by_slug(&slug).await
        .map_err(|e| {
            error!("Database error getting post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Database error"))
            )
        })?;

    let existing_post = match existing_post {
        Some(post) => post,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found(format!("Post '{}' not found", slug)))
            ));
        }
    };

    // Delete from database (soft delete by marking as unpublished)
    state.database.delete_post(existing_post.id).await
        .map_err(|e| {
            error!("Database error deleting post: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to delete post"))
            )
        })?;

    // TODO: Optionally delete from Dropbox or move to archive folder

    let response = PostOperationResponse {
        success: true,
        slug: slug.clone(),
        message: format!("Post '{}' deleted successfully", existing_post.title),
        post: None,
    };

    Ok(Json(response))
}

/// POST /api/sync/dropbox - Sync posts from Dropbox
pub async fn sync_dropbox_api(
    State(_state): State<ApiState>,
    Json(request): Json<SyncDropboxRequest>
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("API: Syncing posts from Dropbox (force: {:?})", request.force);

    // TODO: Implement Dropbox sync logic
    // This would:
    // 1. List all markdown files in Dropbox
    // 2. Compare with database entries
    // 3. Import new files
    // 4. Update modified files (if force=true or based on timestamps)

    let response = SyncResponse {
        success: false,
        message: "Dropbox sync not yet implemented".to_string(),
        synced_count: None,
        errors: None,
    };

    Ok(Json(response))
}

/// POST /api/import/markdown - Import markdown files in bulk
pub async fn import_markdown_api(
    State(state): State<ApiState>,
    Json(request): Json<ImportMarkdownRequest>
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("API: Importing {} markdown files", request.files.len());

    let mut imported = 0;
    let mut errors = Vec::new();

    for file in request.files {
        // Extract title from metadata or content
        let title = file.metadata.as_ref()
            .and_then(|m| m.title.clone())
            .unwrap_or_else(|| extract_title_from_markdown(&file.content));

        let slug = generate_slug(&title);

        // Check if should overwrite
        if !request.overwrite.unwrap_or(false) {
            if let Ok(Some(_)) = state.database.get_post_by_slug(&slug).await {
                errors.push(format!("Post '{}' already exists", slug));
                continue;
            }
        }

        // Parse markdown
        let html_content = match state.markdown.parse_markdown(&file.content) {
            Ok(parsed) => parsed.html,
            Err(e) => {
                errors.push(format!("Failed to parse markdown for '{}': {}", slug, e));
                continue;
            }
        };
        let excerpt = generate_excerpt(&file.content, 200);

        // Create post
        let create_data = CreatePost {
            slug: slug.clone(),
            title,
            content: file.content.clone(),
            html_content,
            excerpt: Some(excerpt),
            category: file.metadata.as_ref().and_then(|m| m.category.clone()),
            tags: file.metadata.as_ref().and_then(|m| m.tags.clone()).unwrap_or_default(),
            published: file.metadata.as_ref().and_then(|m| m.published).unwrap_or(false),
            featured: false,
            author: file.metadata.as_ref().and_then(|m| m.author.clone()),
            dropbox_path: file.path.clone(),
        };

        match state.database.create_post(create_data).await {
            Ok(_) => {
                imported += 1;
                // TODO: Save to Dropbox - implement when blog storage service is ready
            }
            Err(e) => {
                errors.push(format!("Failed to import '{}': {}", slug, e));
            }
        }
    }

    let response = SyncResponse {
        success: errors.is_empty(),
        message: format!("Imported {} posts", imported),
        synced_count: Some(imported),
        errors: if errors.is_empty() { None } else { Some(errors) },
    };

    Ok(Json(response))
}

// Helper functions

fn generate_slug(title: &str) -> String {
    title.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn generate_excerpt(content: &str, max_length: usize) -> String {
    let text = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    
    if text.len() <= max_length {
        text
    } else {
        format!("{}...", &text[..max_length])
    }
}

fn extract_title_from_markdown(content: &str) -> String {
    content.lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}