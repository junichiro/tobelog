use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{debug, error};

use crate::models::{
    response::ErrorResponse, CreateThemeRequest, SiteConfig, SiteConfigResponse, ThemeFilters,
    ThemeListResponse, ThemePreviewResponse, ThemeResponse, UpdateThemeRequest,
};
use crate::services::{DatabaseService, ThemeService};

/// App state for theme handlers
#[derive(Clone)]
pub struct ThemeState {
    pub theme_service: ThemeService,
    pub database: DatabaseService,
}

/// Query parameters for theme listing
#[derive(Debug, Deserialize)]
pub struct ThemeQuery {
    pub is_active: Option<bool>,
    pub layout: Option<String>,
    pub dark_mode_enabled: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Sync query parameters
#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub force: Option<bool>,
}

/// GET /api/themes - List all themes
pub async fn list_themes(
    Query(query): Query<ThemeQuery>,
    State(state): State<ThemeState>,
) -> Result<Json<ThemeListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Listing themes");

    let layout = query.layout.as_ref().and_then(|l| match l.as_str() {
        "single" => Some(crate::models::ThemeLayout::Single),
        "sidebar" => Some(crate::models::ThemeLayout::Sidebar),
        "magazine" => Some(crate::models::ThemeLayout::Magazine),
        _ => None,
    });

    let filters = ThemeFilters {
        is_active: query.is_active,
        layout,
        dark_mode_enabled: query.dark_mode_enabled,
        limit: query.limit,
        offset: query.offset,
    };

    let themes = state
        .theme_service
        .list_themes(filters)
        .await
        .map_err(|e| {
            error!("Failed to list themes: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to list themes")),
            )
        })?;

    let response = ThemeListResponse {
        success: true,
        data: themes.clone(),
        total: themes.len(),
    };

    Ok(Json(response))
}

/// GET /api/themes/{name} - Get a specific theme
pub async fn get_theme(
    Path(name): Path<String>,
    State(state): State<ThemeState>,
) -> Result<Json<ThemeResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting theme: {}", name);

    let theme = state.theme_service.get_theme(&name).await.map_err(|e| {
        error!("Failed to get theme {}: {}", name, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error("Failed to get theme")),
        )
    })?;

    match theme {
        Some(theme_data) => {
            let response = ThemeResponse {
                success: true,
                data: theme_data,
            };
            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(format!(
                "Theme '{}' not found",
                name
            ))),
        )),
    }
}

/// GET /api/themes/active - Get the currently active theme
pub async fn get_active_theme(
    State(state): State<ThemeState>,
) -> Result<Json<ThemeResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting active theme");

    let theme = state.theme_service.get_active_theme().await.map_err(|e| {
        error!("Failed to get active theme: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error("Failed to get active theme")),
        )
    })?;

    let response = ThemeResponse {
        success: true,
        data: theme,
    };

    Ok(Json(response))
}

/// POST /api/themes - Create a new theme
pub async fn create_theme(
    State(state): State<ThemeState>,
    Json(request): Json<CreateThemeRequest>,
) -> Result<Json<ThemeResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Creating theme: {}", request.name);

    let theme = state
        .theme_service
        .create_theme(request)
        .await
        .map_err(|e| {
            error!("Failed to create theme: {}", e);
            let status = if e.to_string().contains("already exists") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse::new(
                    "Theme Creation Failed",
                    e.to_string(),
                    status.as_u16(),
                )),
            )
        })?;

    let response = ThemeResponse {
        success: true,
        data: theme,
    };

    Ok(Json(response))
}

/// PUT /api/themes/{name} - Update a theme
pub async fn update_theme(
    Path(name): Path<String>,
    State(state): State<ThemeState>,
    Json(request): Json<UpdateThemeRequest>,
) -> Result<Json<ThemeResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Updating theme: {}", name);

    let theme = state
        .theme_service
        .update_theme(&name, request)
        .await
        .map_err(|e| {
            error!("Failed to update theme {}: {}", name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to update theme")),
            )
        })?;

    let response = ThemeResponse {
        success: true,
        data: theme,
    };

    Ok(Json(response))
}

/// DELETE /api/themes/{name} - Delete a theme
pub async fn delete_theme(
    Path(name): Path<String>,
    State(state): State<ThemeState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Deleting theme: {}", name);

    let deleted = state.theme_service.delete_theme(&name).await.map_err(|e| {
        error!("Failed to delete theme {}: {}", name, e);
        let status = if e.to_string().contains("Cannot delete active theme") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        (
            status,
            Json(ErrorResponse::new(
                "Theme Deletion Failed",
                e.to_string(),
                status.as_u16(),
            )),
        )
    })?;

    if deleted {
        let response = serde_json::json!({
            "success": true,
            "message": format!("Theme '{}' deleted successfully", name)
        });
        Ok(Json(response))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(format!(
                "Theme '{}' not found",
                name
            ))),
        ))
    }
}

/// POST /api/themes/{name}/activate - Set a theme as active
pub async fn activate_theme(
    Path(name): Path<String>,
    State(state): State<ThemeState>,
) -> Result<Json<ThemeResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Activating theme: {}", name);

    let theme = state
        .theme_service
        .set_active_theme(&name)
        .await
        .map_err(|e| {
            error!("Failed to activate theme {}: {}", name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to activate theme")),
            )
        })?;

    let response = ThemeResponse {
        success: true,
        data: theme,
    };

    Ok(Json(response))
}

/// GET /api/themes/{name}/preview - Get theme preview with compiled CSS
pub async fn get_theme_preview(
    Path(name): Path<String>,
    State(state): State<ThemeState>,
) -> Result<Json<ThemePreviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting theme preview: {}", name);

    let (css, variables) = state
        .theme_service
        .get_theme_preview(&name)
        .await
        .map_err(|e| {
            error!("Failed to get theme preview {}: {}", name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error("Failed to get theme preview")),
            )
        })?;

    let response = ThemePreviewResponse {
        success: true,
        css,
        variables,
    };

    Ok(Json(response))
}

/// GET /api/themes/{name}/css - Get compiled CSS for a theme
pub async fn get_theme_css(
    Path(name): Path<String>,
    State(state): State<ThemeState>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting theme CSS: {}", name);

    let css = state
        .theme_service
        .generate_theme_css(&name)
        .await
        .map_err(|e| {
            error!("Failed to generate theme CSS {}: {}", name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(
                    "Failed to generate theme CSS",
                )),
            )
        })?;

    Ok(css)
}

/// POST /api/themes/sync - Sync themes from Dropbox
pub async fn sync_dropbox_themes(
    Query(_query): Query<SyncQuery>,
    State(state): State<ThemeState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Syncing themes from Dropbox");

    let templates = state
        .theme_service
        .sync_dropbox_themes()
        .await
        .map_err(|e| {
            error!("Failed to sync Dropbox themes: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(
                    "Failed to sync themes from Dropbox",
                )),
            )
        })?;

    let response = serde_json::json!({
        "success": true,
        "message": format!("Synced {} templates from Dropbox", templates.len()),
        "templates": templates.len(),
        "synced_files": templates.iter().map(|t| &t.name).collect::<Vec<_>>()
    });

    Ok(Json(response))
}

/// POST /api/themes/presets - Create preset themes
pub async fn create_preset_themes(
    State(state): State<ThemeState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Creating preset themes");

    state
        .theme_service
        .create_preset_themes()
        .await
        .map_err(|e| {
            error!("Failed to create preset themes: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(
                    "Failed to create preset themes",
                )),
            )
        })?;

    let response = serde_json::json!({
        "success": true,
        "message": "Preset themes created successfully"
    });

    Ok(Json(response))
}

// Site configuration endpoints

/// GET /api/site/config - Get site configuration
pub async fn get_site_config(
    State(state): State<ThemeState>,
) -> Result<Json<SiteConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting site configuration");

    let config = state.theme_service.get_site_config().await.map_err(|e| {
        error!("Failed to get site config: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(
                "Failed to get site configuration",
            )),
        )
    })?;

    let response = SiteConfigResponse {
        success: true,
        data: config,
    };

    Ok(Json(response))
}

/// PUT /api/site/config - Update site configuration
pub async fn update_site_config(
    State(state): State<ThemeState>,
    Json(config): Json<SiteConfig>,
) -> Result<Json<SiteConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Updating site configuration");

    let updated_config = state
        .theme_service
        .update_site_config(config)
        .await
        .map_err(|e| {
            error!("Failed to update site config: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(
                    "Failed to update site configuration",
                )),
            )
        })?;

    let response = SiteConfigResponse {
        success: true,
        data: updated_config,
    };

    Ok(Json(response))
}
