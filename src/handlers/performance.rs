use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::Value;
use tracing::{debug, error};

use crate::models::response::ErrorResponse;
use crate::services::CacheService;

/// Performance monitoring handler state
#[derive(Clone)]
pub struct PerformanceState {
    pub cache: CacheService,
}

/// GET /api/performance/metrics - Get current performance metrics
pub async fn get_performance_metrics(
    State(state): State<PerformanceState>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Getting performance metrics");

    let metrics = state.cache.get_metrics().await;
    let cache_stats = state.cache.get_cache_stats().await;

    let response = serde_json::json!({
        "success": true,
        "data": {
            "performance": metrics,
            "cache": cache_stats,
            "targets": {
                "page_load_time_target": 2000.0, // 2 seconds
                "cache_hit_rate_target": 80.0,   // 80%
                "dropbox_api_reduction_target": 50.0 // 50% reduction
            },
            "status": {
                "page_load_ok": metrics.page_load_time <= 2000.0,
                "cache_hit_ok": metrics.cache_hit_rate >= 80.0,
                "overall_healthy": metrics.page_load_time <= 2000.0 && metrics.cache_hit_rate >= 80.0
            }
        }
    });

    Ok(Json(response))
}

/// POST /api/performance/cache/clear - Clear all caches
pub async fn clear_cache(
    State(state): State<PerformanceState>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Clearing all caches");

    state.cache.invalidate_all().await.map_err(|e| {
        error!("Failed to clear cache: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error("Failed to clear cache")),
        )
    })?;

    let response = serde_json::json!({
        "success": true,
        "message": "All caches cleared successfully"
    });

    Ok(Json(response))
}

/// GET /api/performance/health - Performance health check
pub async fn performance_health_check(
    State(state): State<PerformanceState>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("API: Performance health check");

    let metrics = state.cache.get_metrics().await;
    let cache_stats = state.cache.get_cache_stats().await;

    // Determine health status
    let page_load_healthy = metrics.page_load_time <= 2000.0 || metrics.page_load_time == 0.0;
    let cache_healthy = metrics.cache_hit_rate >= 80.0 || metrics.total_requests < 10; // Don't fail on low traffic
    let overall_healthy = page_load_healthy && cache_healthy;

    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = serde_json::json!({
        "success": overall_healthy,
        "status": if overall_healthy { "healthy" } else { "unhealthy" },
        "checks": {
            "page_load_time": {
                "healthy": page_load_healthy,
                "value": metrics.page_load_time,
                "target": 2000.0,
                "unit": "ms"
            },
            "cache_hit_rate": {
                "healthy": cache_healthy,
                "value": metrics.cache_hit_rate,
                "target": 80.0,
                "unit": "%"
            },
            "cache_stats": cache_stats
        },
        "metrics": metrics
    });

    match status_code {
        StatusCode::OK => Ok(Json(response)),
        _ => Err((
            status_code,
            Json(ErrorResponse::new(
                "performance_unhealthy",
                "Performance metrics indicate unhealthy system state",
                status_code.as_u16(),
            )),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::CacheService;

    #[tokio::test]
    async fn test_performance_health_check_healthy() {
        let cache = CacheService::new();
        let state = PerformanceState { cache };

        let result = performance_health_check(State(state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let cache = CacheService::new();
        let state = PerformanceState { cache };

        let result = clear_cache(State(state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_performance_metrics() {
        let cache = CacheService::new();
        let state = PerformanceState { cache };

        let result = get_performance_metrics(State(state)).await;
        assert!(result.is_ok());
    }
}
