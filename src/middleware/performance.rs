use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{debug, info, warn};

use crate::services::CacheService;

/// Performance monitoring middleware that tracks request timing and cache performance
pub async fn performance_tracking_middleware(
    request: Request,
    State(cache): State<CacheService>,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    // Call the handler
    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis() as f64;
    
    // Log slow requests
    if duration_ms > 1000.0 {
        warn!(
            "Slow request: {} {} took {:.2}ms",
            method,
            uri,
            duration_ms
        );
    } else if duration_ms > 500.0 {
        info!(
            "Request: {} {} took {:.2}ms",
            method,
            uri,
            duration_ms
        );
    } else {
        debug!(
            "Request: {} {} took {:.2}ms",
            method,
            uri,
            duration_ms
        );
    }
    
    // Update performance metrics
    if let Err(e) = cache.update_metrics(|metrics| {
        // Update page load time with exponential moving average
        if metrics.page_load_time == 0.0 {
            metrics.page_load_time = duration_ms;
        } else {
            // Exponential moving average with alpha = 0.1
            metrics.page_load_time = 0.9 * metrics.page_load_time + 0.1 * duration_ms;
        }
    }).await {
        warn!("Failed to update performance metrics: {}", e);
    }
    
    response
}

/// Middleware to add cache-friendly headers for static resources
pub async fn cache_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let uri = request.uri().clone();
    let response = next.run(request).await;
    
    // Add cache headers for static assets
    if uri.path().starts_with("/static/") || 
       uri.path().starts_with("/css/") || 
       uri.path().starts_with("/js/") || 
       uri.path().starts_with("/images/") {
        
        let mut response = response;
        let headers = response.headers_mut();
        
        // Cache static assets for 1 hour
        headers.insert("Cache-Control", "public, max-age=3600".parse().unwrap());
        
        // Add ETag for cache validation (simplified version)
        let etag = format!("\"{}\"", uri.path().len());
        headers.insert("ETag", etag.parse().unwrap());
        
        response
    } else {
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        "test response"
    }

    #[tokio::test]
    async fn test_cache_headers_middleware() {
        let app = Router::new()
            .route("/static/test.css", get(test_handler))
            .route("/api/test", get(test_handler))
            .layer(middleware::from_fn(cache_headers_middleware));

        // Test static asset gets cache headers
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/static/test.css")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let cache_control = response.headers().get("Cache-Control");
        assert!(cache_control.is_some());
        assert_eq!(cache_control.unwrap().to_str().unwrap(), "public, max-age=3600");
        
        let etag = response.headers().get("ETag");
        assert!(etag.is_some());

        // Test API endpoint doesn't get cache headers
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/api/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let cache_control = response.headers().get("Cache-Control");
        assert!(cache_control.is_none());
    }
}