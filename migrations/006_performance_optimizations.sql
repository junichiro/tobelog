-- Migration 006: Performance optimizations
-- Add composite indexes for common query patterns to improve performance

-- Composite index for published posts ordered by creation date (most common query)
CREATE INDEX IF NOT EXISTS idx_posts_published_created_desc ON posts(published, created_at DESC) WHERE published = 1;

-- Composite index for category + published filtering
CREATE INDEX IF NOT EXISTS idx_posts_category_published ON posts(category, published) WHERE published = 1;

-- Composite index for featured posts
CREATE INDEX IF NOT EXISTS idx_posts_featured_published ON posts(featured, published) WHERE featured = 1 AND published = 1;

-- Index for author filtering
CREATE INDEX IF NOT EXISTS idx_posts_author ON posts(author) WHERE author IS NOT NULL;

-- Composite index for tag queries (if we use tag filtering)
-- This will be useful when we optimize tag searching
CREATE INDEX IF NOT EXISTS idx_posts_tags_published ON posts(tags, published) WHERE published = 1;

-- Index for update operations (frequently accessed by slug for updates)
CREATE INDEX IF NOT EXISTS idx_posts_slug_updated_at ON posts(slug, updated_at);

-- Optimize media queries
CREATE INDEX IF NOT EXISTS idx_media_uploaded_at ON media_files(uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_media_mime_type ON media_files(mime_type);

-- Optimize post versions for performance monitoring
CREATE INDEX IF NOT EXISTS idx_post_versions_post_id_version ON post_versions(post_id, version DESC);

-- Add theme performance indexes
CREATE INDEX IF NOT EXISTS idx_themes_active ON themes(is_active) WHERE is_active = 1;
CREATE INDEX IF NOT EXISTS idx_themes_updated_at ON themes(updated_at DESC);

-- Statistics table for caching frequently accessed counts
CREATE TABLE IF NOT EXISTS performance_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stat_key TEXT NOT NULL UNIQUE,
    stat_value TEXT NOT NULL,
    cached_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_performance_stats_key_expires ON performance_stats(stat_key, expires_at);

-- Performance monitoring table for tracking slow queries and requests
CREATE TABLE IF NOT EXISTS performance_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    log_type TEXT NOT NULL, -- 'request', 'query', 'dropbox_api'
    operation TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    details TEXT, -- JSON details about the operation
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_performance_logs_type_created ON performance_logs(log_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_performance_logs_duration ON performance_logs(duration_ms DESC);

-- Cache invalidation tracking
CREATE TABLE IF NOT EXISTS cache_invalidations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_key TEXT NOT NULL,
    invalidation_reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cache_invalidations_created ON cache_invalidations(created_at DESC);

-- Insert initial performance stats
INSERT OR IGNORE INTO performance_stats (stat_key, stat_value, cached_at, expires_at) VALUES
('total_posts', '0', datetime('now'), datetime('now', '+1 hour')),
('published_posts', '0', datetime('now'), datetime('now', '+1 hour')),
('draft_posts', '0', datetime('now'), datetime('now', '+1 hour')),
('featured_posts', '0', datetime('now'), datetime('now', '+1 hour'));