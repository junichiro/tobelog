use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{Json, Response},
};
use serde_json::json;
use tracing::{debug, warn};

use crate::config::Config;

/// Authentication middleware for API endpoints
pub async fn auth_middleware(
    State(config): State<Config>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let path = request.uri().path();
    let method = request.method().as_str();
    
    // Skip authentication for read-only endpoints and GET methods
    if method == "GET" || is_read_only_endpoint(path, method) {
        debug!("Skipping auth for read-only endpoint: {} {}", method, path);
        return Ok(next.run(request).await);
    }

    debug!("Auth middleware processing: {} {}", method, path);
    
    // Skip authentication if no API key is configured
    let Some(expected_api_key) = &config.api_key else {
        debug!("No API key configured, allowing request to: {} {}", method, path);
        return Ok(next.run(request).await);
    };

    // Check for API key in headers
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .or_else(|| headers.get("X-API-Key").and_then(|h| h.to_str().ok()));

    match auth_header {
        Some(provided_key) => {
            let key = if provided_key.starts_with("Bearer ") {
                &provided_key[7..]
            } else {
                provided_key
            };

            if key == expected_api_key {
                debug!("API key authentication successful for: {}", path);
                Ok(next.run(request).await)
            } else {
                warn!("Invalid API key provided for: {}", path);
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "unauthorized",
                        "message": "Invalid API key"
                    })),
                ))
            }
        }
        None => {
            warn!("No API key provided for protected endpoint: {}", path);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "unauthorized",
                    "message": "API key required for this endpoint"
                })),
            ))
        }
    }
}

/// Check if the endpoint is read-only (doesn't require authentication)
fn is_read_only_endpoint(path: &str, method: &str) -> bool {
    // Always allow GET requests
    if method == "GET" {
        return true;
    }
    
    // Allow specific endpoints regardless of method
    matches!(path, 
        "/" |
        "/health" |
        "/api/dropbox/status"
    ) || path.starts_with("/posts/") 
      || path.starts_with("/static/")
}

/// Rate limiting middleware (placeholder for future implementation)
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // TODO: Implement rate limiting logic
    // For now, just pass through
    Ok(next.run(request).await)
}

/// CSRF protection middleware (placeholder for future implementation)
pub async fn csrf_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // TODO: Implement CSRF protection
    // For now, just pass through
    Ok(next.run(request).await)
}