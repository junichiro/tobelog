use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::models::{Post, PostSummary};

/// Performance metrics for monitoring cache effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub page_load_time: f64,
    pub first_contentful_paint: f64,
    pub largest_contentful_paint: f64,
    pub cumulative_layout_shift: f64,
    pub dropbox_api_calls: u32,
    pub cache_hit_rate: f64,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            page_load_time: 0.0,
            first_contentful_paint: 0.0,
            largest_contentful_paint: 0.0,
            cumulative_layout_shift: 0.0,
            dropbox_api_calls: 0,
            cache_hit_rate: 0.0,
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

/// Cached blog post with expiration
#[derive(Debug, Clone)]
pub struct CachedPost {
    pub post: Post,
    pub cached_at: Instant,
    pub expires_at: Instant,
}

impl CachedPost {
    pub fn new(post: Post, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            post,
            cached_at: now,
            expires_at: now + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Cached post list with metadata
#[derive(Debug, Clone)]
pub struct CachedPostList {
    pub posts: Vec<PostSummary>,
    pub total_count: usize,
    pub cached_at: Instant,
    pub expires_at: Instant,
}

impl CachedPostList {
    pub fn new(posts: Vec<PostSummary>, total_count: usize, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            posts,
            total_count,
            cached_at: now,
            expires_at: now + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Cached blog statistics
#[derive(Debug, Clone)]
pub struct CachedStats {
    pub total_posts: i64,
    pub published_posts: i64,
    pub draft_posts: i64,
    pub featured_posts: i64,
    pub categories: Vec<(String, i64)>,
    pub cached_at: Instant,
    pub expires_at: Instant,
}

impl CachedStats {
    pub fn new(
        total_posts: i64,
        published_posts: i64,
        draft_posts: i64,
        featured_posts: i64,
        categories: Vec<(String, i64)>,
        ttl: Duration,
    ) -> Self {
        let now = Instant::now();
        Self {
            total_posts,
            published_posts,
            draft_posts,
            featured_posts,
            categories,
            cached_at: now,
            expires_at: now + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub post_ttl: Duration,
    pub post_list_ttl: Duration,
    pub stats_ttl: Duration,
    pub max_posts: usize,
    pub max_lists: usize,
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            post_ttl: Duration::from_secs(600), // 10 minutes
            post_list_ttl: Duration::from_secs(300), // 5 minutes
            stats_ttl: Duration::from_secs(900), // 15 minutes
            max_posts: 1000,
            max_lists: 50,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// In-memory cache service for blog content and metadata
#[derive(Clone)]
pub struct CacheService {
    posts: Arc<RwLock<HashMap<String, CachedPost>>>,
    post_lists: Arc<RwLock<HashMap<String, CachedPostList>>>,
    stats: Arc<RwLock<Option<CachedStats>>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    config: CacheConfig,
    last_cleanup: Arc<RwLock<Instant>>,
}

impl CacheService {
    /// Create a new cache service with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache service with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            posts: Arc::new(RwLock::new(HashMap::new())),
            post_lists: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            config,
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Get a cached post by slug
    pub async fn get_post(&self, slug: &str) -> Option<Post> {
        let posts = self.posts.read().await;
        if let Some(cached_post) = posts.get(slug) {
            if !cached_post.is_expired() {
                debug!("Cache hit for post: {}", slug);
                self.record_cache_hit().await;
                return Some(cached_post.post.clone());
            } else {
                debug!("Cache expired for post: {}", slug);
            }
        }
        
        debug!("Cache miss for post: {}", slug);
        self.record_cache_miss().await;
        None
    }

    /// Cache a post with TTL
    pub async fn set_post(&self, slug: &str, post: Post) -> Result<()> {
        self.cleanup_if_needed().await;
        
        let mut posts = self.posts.write().await;
        
        // Check if we're at capacity and need to evict
        if posts.len() >= self.config.max_posts && !posts.contains_key(slug) {
            self.evict_oldest_posts(&mut posts).await;
        }
        
        let cached_post = CachedPost::new(post, self.config.post_ttl);
        posts.insert(slug.to_string(), cached_post);
        
        debug!("Cached post: {}", slug);
        Ok(())
    }

    /// Get cached post list by cache key
    pub async fn get_post_list(&self, cache_key: &str) -> Option<(Vec<PostSummary>, usize)> {
        let post_lists = self.post_lists.read().await;
        if let Some(cached_list) = post_lists.get(cache_key) {
            if !cached_list.is_expired() {
                debug!("Cache hit for post list: {}", cache_key);
                self.record_cache_hit().await;
                return Some((cached_list.posts.clone(), cached_list.total_count));
            } else {
                debug!("Cache expired for post list: {}", cache_key);
            }
        }
        
        debug!("Cache miss for post list: {}", cache_key);
        self.record_cache_miss().await;
        None
    }

    /// Cache a post list with TTL
    pub async fn set_post_list(&self, cache_key: &str, posts: Vec<PostSummary>, total_count: usize) -> Result<()> {
        self.cleanup_if_needed().await;
        
        let mut post_lists = self.post_lists.write().await;
        
        // Check if we're at capacity and need to evict
        if post_lists.len() >= self.config.max_lists && !post_lists.contains_key(cache_key) {
            self.evict_oldest_lists(&mut post_lists).await;
        }
        
        let cached_list = CachedPostList::new(posts, total_count, self.config.post_list_ttl);
        post_lists.insert(cache_key.to_string(), cached_list);
        
        debug!("Cached post list: {}", cache_key);
        Ok(())
    }

    /// Get cached blog statistics
    pub async fn get_stats(&self) -> Option<CachedStats> {
        let stats = self.stats.read().await;
        if let Some(cached_stats) = stats.as_ref() {
            if !cached_stats.is_expired() {
                debug!("Cache hit for blog stats");
                self.record_cache_hit().await;
                return Some(cached_stats.clone());
            } else {
                debug!("Cache expired for blog stats");
            }
        }
        
        debug!("Cache miss for blog stats");
        self.record_cache_miss().await;
        None
    }

    /// Cache blog statistics
    pub async fn set_stats(
        &self,
        total_posts: i64,
        published_posts: i64,
        draft_posts: i64,
        featured_posts: i64,
        categories: Vec<(String, i64)>,
    ) -> Result<()> {
        let mut stats = self.stats.write().await;
        let cached_stats = CachedStats::new(
            total_posts,
            published_posts,
            draft_posts,
            featured_posts,
            categories,
            self.config.stats_ttl,
        );
        *stats = Some(cached_stats);
        
        debug!("Cached blog stats");
        Ok(())
    }

    /// Invalidate all cached data
    pub async fn invalidate_all(&self) -> Result<()> {
        {
            let mut posts = self.posts.write().await;
            posts.clear();
        }
        {
            let mut post_lists = self.post_lists.write().await;
            post_lists.clear();
        }
        {
            let mut stats = self.stats.write().await;
            *stats = None;
        }
        
        info!("Invalidated all cache entries");
        Ok(())
    }

    /// Invalidate cached data for a specific post
    pub async fn invalidate_post(&self, slug: &str) -> Result<()> {
        {
            let mut posts = self.posts.write().await;
            posts.remove(slug);
        }
        
        // Invalidate all post lists since they might contain this post
        {
            let mut post_lists = self.post_lists.write().await;
            post_lists.clear();
        }
        
        // Invalidate stats as they might be affected
        {
            let mut stats = self.stats.write().await;
            *stats = None;
        }
        
        debug!("Invalidated cache for post: {}", slug);
        Ok(())
    }

    /// Generate cache key for post lists based on filters
    pub fn generate_list_cache_key(
        &self,
        category: Option<&str>,
        tag: Option<&str>,
        published: Option<bool>,
        featured: Option<bool>,
        page: Option<usize>,
        per_page: Option<usize>,
    ) -> String {
        let mut key_parts = Vec::new();
        
        if let Some(cat) = category {
            key_parts.push(format!("cat:{}", cat));
        }
        if let Some(t) = tag {
            key_parts.push(format!("tag:{}", t));
        }
        if let Some(pub_flag) = published {
            key_parts.push(format!("pub:{}", pub_flag));
        }
        if let Some(feat_flag) = featured {
            key_parts.push(format!("feat:{}", feat_flag));
        }
        if let Some(p) = page {
            key_parts.push(format!("page:{}", p));
        }
        if let Some(pp) = per_page {
            key_parts.push(format!("per_page:{}", pp));
        }
        
        if key_parts.is_empty() {
            "all_posts".to_string()
        } else {
            key_parts.join(":")
        }
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Update performance metrics
    pub async fn update_metrics<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut PerformanceMetrics),
    {
        let mut metrics = self.metrics.write().await;
        updater(&mut *metrics);
        
        // Recalculate cache hit rate
        if metrics.total_requests > 0 {
            metrics.cache_hit_rate = (metrics.cache_hits as f64 / metrics.total_requests as f64) * 100.0;
        }
        
        Ok(())
    }

    /// Record cache hit for metrics
    async fn record_cache_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.cache_hits += 1;
        metrics.total_requests += 1;
        
        if metrics.total_requests > 0 {
            metrics.cache_hit_rate = (metrics.cache_hits as f64 / metrics.total_requests as f64) * 100.0;
        }
    }

    /// Record cache miss for metrics
    async fn record_cache_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.cache_misses += 1;
        metrics.total_requests += 1;
        
        if metrics.total_requests > 0 {
            metrics.cache_hit_rate = (metrics.cache_hits as f64 / metrics.total_requests as f64) * 100.0;
        }
    }

    /// Clean up expired entries if needed
    async fn cleanup_if_needed(&self) {
        let last_cleanup = *self.last_cleanup.read().await;
        if last_cleanup.elapsed() > self.config.cleanup_interval {
            self.cleanup_expired().await;
            *self.last_cleanup.write().await = Instant::now();
        }
    }

    /// Clean up expired cache entries
    async fn cleanup_expired(&self) {
        let mut removed_count = 0;
        
        // Clean up expired posts
        {
            let mut posts = self.posts.write().await;
            let original_len = posts.len();
            posts.retain(|_, cached_post| !cached_post.is_expired());
            removed_count += original_len - posts.len();
        }
        
        // Clean up expired post lists
        {
            let mut post_lists = self.post_lists.write().await;
            let original_len = post_lists.len();
            post_lists.retain(|_, cached_list| !cached_list.is_expired());
            removed_count += original_len - post_lists.len();
        }
        
        // Clean up expired stats
        {
            let mut stats = self.stats.write().await;
            if let Some(cached_stats) = stats.as_ref() {
                if cached_stats.is_expired() {
                    *stats = None;
                    removed_count += 1;
                }
            }
        }
        
        if removed_count > 0 {
            debug!("Cleaned up {} expired cache entries", removed_count);
        }
    }

    /// Evict oldest posts when at capacity
    async fn evict_oldest_posts(&self, posts: &mut HashMap<String, CachedPost>) {
        let evict_count = posts.len() / 4; // Evict 25% when at capacity
        
        let mut post_ages: Vec<(String, Instant)> = posts
            .iter()
            .map(|(slug, cached_post)| (slug.clone(), cached_post.cached_at))
            .collect();
        
        post_ages.sort_by(|a, b| a.1.cmp(&b.1)); // Sort by cache time (oldest first)
        
        for (slug, _) in post_ages.into_iter().take(evict_count) {
            posts.remove(&slug);
        }
        
        debug!("Evicted {} old posts from cache", evict_count);
    }

    /// Evict oldest post lists when at capacity
    async fn evict_oldest_lists(&self, post_lists: &mut HashMap<String, CachedPostList>) {
        let evict_count = post_lists.len() / 4; // Evict 25% when at capacity
        
        let mut list_ages: Vec<(String, Instant)> = post_lists
            .iter()
            .map(|(key, cached_list)| (key.clone(), cached_list.cached_at))
            .collect();
        
        list_ages.sort_by(|a, b| a.1.cmp(&b.1)); // Sort by cache time (oldest first)
        
        for (key, _) in list_ages.into_iter().take(evict_count) {
            post_lists.remove(&key);
        }
        
        debug!("Evicted {} old post lists from cache", evict_count);
    }

    /// Get cache statistics for monitoring
    pub async fn get_cache_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        let posts = self.posts.read().await;
        stats.insert("cached_posts".to_string(), posts.len());
        
        let post_lists = self.post_lists.read().await;
        stats.insert("cached_lists".to_string(), post_lists.len());
        
        let blog_stats = self.stats.read().await;
        stats.insert("cached_stats".to_string(), if blog_stats.is_some() { 1 } else { 0 });
        
        stats
    }
}

impl Default for CacheService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreatePost, Post};

    #[tokio::test]
    async fn test_cache_service_creation() {
        let cache = CacheService::new();
        let stats = cache.get_cache_stats().await;
        assert_eq!(stats.get("cached_posts"), Some(&0));
        assert_eq!(stats.get("cached_lists"), Some(&0));
        assert_eq!(stats.get("cached_stats"), Some(&0));
    }

    #[tokio::test]
    async fn test_post_caching() {
        let cache = CacheService::new();
        let post = Post::new(CreatePost {
            slug: "test-post".to_string(),
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            html_content: "<p>Test content</p>".to_string(),
            category: Some("test".to_string()),
            tags: vec!["test".to_string()],
            published: true,
            featured: false,
            author: Some("test".to_string()),
            excerpt: None,
            dropbox_path: "/test/test-post.md".to_string(),
        });

        // Cache miss initially
        assert!(cache.get_post("test-post").await.is_none());

        // Cache the post
        cache.set_post("test-post", post.clone()).await.unwrap();

        // Cache hit now
        let cached_post = cache.get_post("test-post").await;
        assert!(cached_post.is_some());
        assert_eq!(cached_post.unwrap().slug, "test-post");
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = CacheService::new();
        let post = Post::new(CreatePost {
            slug: "test-post".to_string(),
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            html_content: "<p>Test content</p>".to_string(),
            category: Some("test".to_string()),
            tags: vec!["test".to_string()],
            published: true,
            featured: false,
            author: Some("test".to_string()),
            excerpt: None,
            dropbox_path: "/test/test-post.md".to_string(),
        });

        cache.set_post("test-post", post).await.unwrap();
        assert!(cache.get_post("test-post").await.is_some());

        cache.invalidate_post("test-post").await.unwrap();
        assert!(cache.get_post("test-post").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let cache = CacheService::new();
        
        let key1 = cache.generate_list_cache_key(None, None, None, None, None, None);
        assert_eq!(key1, "all_posts");
        
        let key2 = cache.generate_list_cache_key(
            Some("tech"),
            Some("rust"),
            Some(true),
            Some(false),
            Some(1),
            Some(10)
        );
        assert_eq!(key2, "cat:tech:tag:rust:pub:true:feat:false:page:1:per_page:10");
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let cache = CacheService::new();
        
        // Initial metrics
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
        
        // Generate cache miss
        cache.get_post("nonexistent").await;
        
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 1);
        assert_eq!(metrics.cache_hit_rate, 0.0);
    }
}