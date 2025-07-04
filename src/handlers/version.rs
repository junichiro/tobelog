use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{debug, error};
use uuid::Uuid;

use crate::models::{
    response::ErrorResponse, RestoreVersionRequest, RestoreVersionResponse, VersionDiffResponse,
    VersionHistoryResponse, VersionResponse,
};
use crate::services::{DatabaseService, VersionService};

/// App state for version handlers
#[derive(Clone)]
pub struct VersionState {
    pub version_service: VersionService,
    pub database: DatabaseService,
}

/// Query parameters for version listing
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VersionQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for cleanup operation
#[derive(Debug, Deserialize)]
pub struct CleanupQuery {
    pub keep_versions: Option<i32>,
}

/// Helper function to get post ID by slug
async fn get_post_id_by_slug(
    database: &DatabaseService,
    slug: &str,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let post = database.get_post_by_slug(slug).await.map_err(|e| {
        error!("Database error when getting post by slug {}: {}", slug, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error("Failed to get post")),
        )
    })?;

    match post {
        Some(post) => Ok(post.id),
        None => {
            error!("Post not found with slug: {}", slug);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found(format!(
                    "Post with slug '{}' not found",
                    slug
                ))),
            ))
        }
    }
}

/// GET /api/posts/{slug}/versions - Get version history for a post
pub async fn get_version_history(
    Path(slug): Path<String>,
    Query(_query): Query<VersionQuery>,
    State(state): State<VersionState>,
) -> Result<Json<VersionHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting version history for post: {}", slug);

    // Get the post ID by slug
    let post_id = get_post_id_by_slug(&state.database, &slug).await?;

    let history = state
        .version_service
        .get_version_history(post_id)
        .await
        .map_err(|e| {
            error!("Failed to get version history for post {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(
                    "Failed to get version history",
                )),
            )
        })?;

    let response = VersionHistoryResponse {
        success: true,
        data: history,
    };

    Ok(Json(response))
}

/// GET /api/posts/{slug}/versions/{version} - Get a specific version of a post
pub async fn get_post_version(
    Path((slug, version)): Path<(String, i32)>,
    State(state): State<VersionState>,
) -> Result<Json<VersionResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting version {} for post: {}", version, slug);

    let post_id = get_post_id_by_slug(&state.database, &slug).await?;

    let post_version = state
        .version_service
        .get_version(post_id, version)
        .await
        .map_err(|e| {
            error!("Failed to get version {} for post {}: {}", version, slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to get post version")),
            )
        })?;

    match post_version {
        Some(version_data) => {
            let response = VersionResponse {
                success: true,
                data: version_data,
            };
            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(format!(
                "Version {} not found for post {}",
                version, slug
            ))),
        )),
    }
}

/// GET /api/posts/{slug}/diff/{version_from}/{version_to} - Compare two versions
pub async fn compare_versions(
    Path((slug, version_from, version_to)): Path<(String, i32, i32)>,
    State(state): State<VersionState>,
) -> Result<Json<VersionDiffResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!(
        "API: Comparing versions {} and {} for post: {}",
        version_from, version_to, slug
    );

    let post_id = get_post_id_by_slug(&state.database, &slug).await?;

    let diff = state
        .version_service
        .compare_versions(post_id, version_from, version_to)
        .await
        .map_err(|e| {
            error!(
                "Failed to compare versions {} and {} for post {}: {}",
                version_from, version_to, slug, e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to compare versions")),
            )
        })?;

    let response = VersionDiffResponse {
        success: true,
        data: diff,
    };

    Ok(Json(response))
}

/// POST /api/posts/{slug}/restore/{version} - Restore a post to a previous version
pub async fn restore_version(
    Path((slug, target_version)): Path<(String, i32)>,
    State(state): State<VersionState>,
    Json(request): Json<RestoreVersionRequest>,
) -> Result<Json<RestoreVersionResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Restoring post {} to version {}", slug, target_version);

    let post_id = get_post_id_by_slug(&state.database, &slug).await?;

    let restored_post = state
        .version_service
        .restore_version(post_id, target_version, request.change_summary)
        .await
        .map_err(|e| {
            error!(
                "Failed to restore post {} to version {}: {}",
                slug, target_version, e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to restore version")),
            )
        })?;

    let response = RestoreVersionResponse {
        success: true,
        message: format!("Successfully restored post to version {}", target_version),
        new_version: restored_post.version,
    };

    Ok(Json(response))
}

/// POST /api/posts/{slug}/versions/cleanup - Clean up old versions
pub async fn cleanup_old_versions(
    Path(slug): Path<String>,
    Query(query): Query<CleanupQuery>,
    State(state): State<VersionState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Cleaning up old versions for post: {}", slug);

    let post_id = get_post_id_by_slug(&state.database, &slug).await?;

    // Use query parameter or default to keeping last 10 versions
    let keep_versions = query.keep_versions.unwrap_or(10);

    // Validate the keep_versions parameter
    if keep_versions < 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(
                "keep_versions must be at least 1",
            )),
        ));
    }

    let deleted_count = state
        .version_service
        .cleanup_old_versions(post_id, keep_versions)
        .await
        .map_err(|e| {
            error!("Failed to cleanup old versions for post {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(
                    "Failed to cleanup old versions",
                )),
            )
        })?;

    let response = serde_json::json!({
        "success": true,
        "message": format!("Cleaned up {} old versions", deleted_count),
        "deleted_count": deleted_count,
        "kept_versions": keep_versions
    });

    Ok(Json(response))
}
