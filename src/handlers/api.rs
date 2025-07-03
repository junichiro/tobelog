use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Json, Response},
    body::Body,
};
use axum_extra::extract::{Multipart, multipart::Field};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use crate::models::{
    response::{PostListResponse, PostResponse, PostSummary, ErrorResponse, 
              BlogStatsResponse, CategoryInfo, TagInfo},
    PostFilters, CreatePost, UpdatePost, LLMArticleImportRequest, LLMArticleImportResponse,
    BatchImportRequest, BatchImportResponse, MediaQuery, MediaListResponse, 
    MediaUploadResponse, MediaFilters
};
use crate::services::{DatabaseService, MarkdownService, BlogStorageService, LLMImportService, MediaService};
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
    pub llm_import: LLMImportService,
    pub media: MediaService,
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

    // Save to Dropbox using blog storage service
    let blog_post = crate::services::blog_storage::BlogPost {
        metadata: crate::services::blog_storage::BlogPostMetadata {
            title: post.title.clone(),
            slug: post.slug.clone(),
            created_at: post.created_at,
            updated_at: post.updated_at,
            category: post.category.clone(),
            tags: parse_tags_from_json(&post.tags),
            published: post.published,
            author: post.author.clone(),
            excerpt: post.excerpt.clone(),
        },
        content: post.content.clone(),
        dropbox_path: post.dropbox_path.clone(),
        file_metadata: None,
    };

    match state.blog_storage.save_post(&blog_post, false).await {
        Ok(_) => {
            info!("Post saved to Dropbox: {}", dropbox_path);
        }
        Err(e) => {
            error!("Failed to save post to Dropbox: {}", e);
            // Don't fail the request, but log the error
            // The post is already saved in the database
        }
    }

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

    let existing_post = match existing_post {
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

    // Update in Dropbox if content changed
    if let Some(ref updated_post) = updated_post {
        let blog_post = crate::services::blog_storage::BlogPost {
            metadata: crate::services::blog_storage::BlogPostMetadata {
                title: updated_post.title.clone(),
                slug: updated_post.slug.clone(),
                created_at: updated_post.created_at,
                updated_at: updated_post.updated_at,
                category: updated_post.category.clone(),
                tags: parse_tags_from_json(&updated_post.tags),
                published: updated_post.published,
                author: updated_post.author.clone(),
                excerpt: updated_post.excerpt.clone(),
            },
            content: updated_post.content.clone(),
            dropbox_path: updated_post.dropbox_path.clone(),
            file_metadata: None,
        };

        match state.blog_storage.save_post(&blog_post, false).await {
            Ok(_) => {
                info!("Post updated in Dropbox: {}", existing_post.dropbox_path);
            }
            Err(e) => {
                error!("Failed to update post in Dropbox: {}", e);
                // Don't fail the request, but log the error
            }
        }
    }

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

    // Delete from Dropbox (or move to archive folder)
    match state.blog_storage.delete_post(&slug).await {
        Ok(true) => {
            info!("Post deleted from Dropbox: {}", existing_post.dropbox_path);
        }
        Ok(false) => {
            warn!("Post not found in Dropbox: {}", slug);
        }
        Err(e) => {
            error!("Failed to delete post from Dropbox: {}", e);
            // Don't fail the request, but log the error
        }
    }

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
    State(state): State<ApiState>,
    Json(request): Json<SyncDropboxRequest>
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("API: Syncing posts from Dropbox (force: {:?})", request.force);

    let mut synced = 0;
    let mut errors = Vec::new();

    // Get all published posts from Dropbox
    match state.blog_storage.list_published_posts().await {
        Ok(dropbox_posts) => {
            for dropbox_post in dropbox_posts {
                // Check if post exists in database
                match state.database.get_post_by_slug(&dropbox_post.metadata.slug).await {
                    Ok(Some(db_post)) => {
                        // Post exists, check if we should update
                        if request.force.unwrap_or(false) || dropbox_post.metadata.updated_at > db_post.updated_at {
                            // Update existing post
                            let update_data = crate::models::UpdatePost {
                                title: Some(dropbox_post.metadata.title.clone()),
                                content: Some(dropbox_post.content.clone()),
                                html_content: None, // Will be generated from content
                                excerpt: dropbox_post.metadata.excerpt.clone(),
                                category: dropbox_post.metadata.category.clone(),
                                tags: Some(dropbox_post.metadata.tags.clone()),
                                published: Some(dropbox_post.metadata.published),
                                featured: None,
                                author: dropbox_post.metadata.author.clone(),
                                dropbox_path: Some(dropbox_post.dropbox_path.clone()),
                            };

                            match state.database.update_post(db_post.id, update_data).await {
                                Ok(_) => {
                                    synced += 1;
                                    info!("Updated existing post: {}", dropbox_post.metadata.slug);
                                }
                                Err(e) => {
                                    errors.push(format!("Failed to update post '{}': {}", dropbox_post.metadata.slug, e));
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // New post, create it
                        let create_data = crate::models::CreatePost {
                            slug: dropbox_post.metadata.slug.clone(),
                            title: dropbox_post.metadata.title.clone(),
                            content: dropbox_post.content.clone(),
                            html_content: String::new(), // Will be generated
                            excerpt: dropbox_post.metadata.excerpt,
                            category: dropbox_post.metadata.category,
                            tags: dropbox_post.metadata.tags,
                            published: dropbox_post.metadata.published,
                            featured: false,
                            author: dropbox_post.metadata.author,
                            dropbox_path: dropbox_post.dropbox_path,
                        };

                        match state.database.create_post(create_data).await {
                            Ok(_) => {
                                synced += 1;
                                info!("Created new post: {}", dropbox_post.metadata.slug);
                            }
                            Err(e) => {
                                errors.push(format!("Failed to create post '{}': {}", dropbox_post.metadata.slug, e));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Database error checking post '{}': {}", dropbox_post.metadata.slug, e));
                    }
                }
            }
        }
        Err(e) => {
            errors.push(format!("Failed to list Dropbox posts: {}", e));
        }
    }

    let response = SyncResponse {
        success: errors.is_empty(),
        message: format!("Synced {} posts from Dropbox", synced),
        synced_count: Some(synced),
        errors: if errors.is_empty() { None } else { Some(errors) },
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
            Ok(post) => {
                imported += 1;
                
                // Save to Dropbox as well
                let blog_post = crate::services::blog_storage::BlogPost {
                    metadata: crate::services::blog_storage::BlogPostMetadata {
                        title: post.title.clone(),
                        slug: post.slug.clone(),
                        created_at: post.created_at,
                        updated_at: post.updated_at,
                        category: post.category.clone(),
                        tags: parse_tags_from_json(&post.tags),
                        published: post.published,
                        author: post.author.clone(),
                        excerpt: post.excerpt.clone(),
                    },
                    content: post.content.clone(),
                    dropbox_path: post.dropbox_path.clone(),
                    file_metadata: None,
                };

                if let Err(e) = state.blog_storage.save_post(&blog_post, false).await {
                    errors.push(format!("Failed to save '{}' to Dropbox: {}", slug, e));
                }
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

fn parse_tags_from_json(tags_json: &str) -> Vec<String> {
    serde_json::from_str(tags_json).unwrap_or_default()
}

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

// Media API endpoints

/// POST /api/media/upload - Upload media file
pub async fn upload_media_api(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<MediaUploadResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Uploading media file");

    let mut alt_text: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut file_field: Option<Field> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await
        .map_err(|e| {
            error!("Failed to read multipart field: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::bad_request("Invalid multipart data"))
            )
        })? {
        
        match field.name() {
            Some("file") => {
                file_field = Some(field);
            }
            Some("alt_text") => {
                alt_text = field.text().await.ok();
            }
            Some("caption") => {
                caption = field.text().await.ok();
            }
            _ => {
                // Skip unknown fields
                let _ = field.bytes().await;
            }
        }
    }

    let file_field = file_field.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("No file provided"))
        )
    })?;

    // Upload file using media service
    let media_file = state.media.upload_file(file_field, alt_text, caption).await
        .map_err(|e| {
            error!("Media upload error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(format!("Upload failed: {}", e)))
            )
        })?;

    let response = MediaUploadResponse {
        success: true,
        message: format!("File '{}' uploaded successfully", media_file.filename),
        media: Some(media_file),
        errors: None,
    };

    Ok(Json(response))
}

/// GET /api/media - List media files
pub async fn list_media_api(
    Query(query): Query<MediaQuery>,
    State(state): State<ApiState>,
) -> Result<Json<MediaListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Listing media files with query: {:?}", query);

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100); // Limit to 100 per page
    let offset = (page.saturating_sub(1)) * per_page;

    let filters = MediaFilters {
        folder: query.folder.clone(),
        mime_type: query.mime_type.clone(),
        search: query.search.clone(),
        limit: Some(per_page as i64),
        offset: Some(offset as i64),
    };

    // Get media files
    let media_files = state.media.list_media_files(filters.clone()).await
        .map_err(|e| {
            error!("Database error listing media: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to load media files"))
            )
        })?;

    // Get total count
    let mut count_filters = filters.clone();
    count_filters.limit = None;
    count_filters.offset = None;

    let total_count = state.media.count_media_files(count_filters).await
        .map_err(|e| {
            error!("Database error counting media: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to count media files"))
            )
        })?;

    let total_pages = total_count.div_ceil(per_page);

    let response = MediaListResponse {
        media: media_files,
        total: total_count,
        page,
        per_page,
        total_pages,
    };

    Ok(Json(response))
}

/// DELETE /api/media/{id} - Delete media file
pub async fn delete_media_api(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<MediaUploadResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Deleting media file with ID: {}", id);

    let media_id = Uuid::parse_str(&id)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::bad_request("Invalid media ID format"))
            )
        })?;

    let deleted = state.media.delete_media_file(media_id).await
        .map_err(|e| {
            error!("Media deletion error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to delete media file"))
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Media file not found"))
        ));
    }

    let response = MediaUploadResponse {
        success: true,
        message: "Media file deleted successfully".to_string(),
        media: None,
        errors: None,
    };

    Ok(Json(response))
}

/// GET /media/{path} - Serve media file
pub async fn serve_media_file(
    Path(path): Path<String>,
    State(state): State<ApiState>,
) -> Result<Response<Body>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Serving media file: {}", path);

    let (data, mime_type) = state.media.serve_media_file(&path).await
        .map_err(|e| {
            error!("Media serving error: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Media file not found"))
            )
        })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000") // Cache for 1 year
        .body(Body::from(data))
        .map_err(|e| {
            error!("Failed to build response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to serve file"))
            )
        })?;

    Ok(response)
}