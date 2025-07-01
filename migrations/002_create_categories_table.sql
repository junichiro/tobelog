-- Create categories table for category metadata
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    post_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create index for category lookups
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);
CREATE INDEX IF NOT EXISTS idx_categories_name ON categories(name);

-- Create tags table for tag metadata
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    post_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create index for tag lookups
CREATE INDEX IF NOT EXISTS idx_tags_slug ON tags(slug);
CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);

-- Create blog_config table for site configuration
CREATE TABLE IF NOT EXISTS blog_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Insert default configuration
INSERT OR IGNORE INTO blog_config (key, value, updated_at) VALUES
    ('title', 'My Personal Blog', datetime('now')),
    ('description', 'A personal blog built with Rust', datetime('now')),
    ('author', 'Blog Author', datetime('now')),
    ('base_url', 'http://localhost:3000', datetime('now')),
    ('theme', 'default', datetime('now')),
    ('posts_per_page', '10', datetime('now')),
    ('excerpt_length', '50', datetime('now'));