-- Create posts table for blog post metadata storage
CREATE TABLE IF NOT EXISTS posts (
    id TEXT PRIMARY KEY, -- UUID as TEXT
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    html_content TEXT NOT NULL,
    excerpt TEXT,
    category TEXT,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON array as TEXT
    published BOOLEAN NOT NULL DEFAULT FALSE,
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    author TEXT,
    dropbox_path TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL, -- ISO 8601 timestamp
    updated_at TEXT NOT NULL, -- ISO 8601 timestamp
    published_at TEXT -- ISO 8601 timestamp, NULL if not published
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts(slug);
CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(published);
CREATE INDEX IF NOT EXISTS idx_posts_category ON posts(category);
CREATE INDEX IF NOT EXISTS idx_posts_featured ON posts(featured);
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at);
CREATE INDEX IF NOT EXISTS idx_posts_published_at ON posts(published_at);
CREATE INDEX IF NOT EXISTS idx_posts_dropbox_path ON posts(dropbox_path);

-- Create full-text search index for posts
CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5(
    title,
    content,
    excerpt,
    content='posts',
    content_rowid='rowid'
);

-- Triggers to keep FTS table in sync
CREATE TRIGGER IF NOT EXISTS posts_fts_insert AFTER INSERT ON posts BEGIN
    INSERT INTO posts_fts(rowid, title, content, excerpt)
    VALUES (new.rowid, new.title, new.content, COALESCE(new.excerpt, ''));
END;

CREATE TRIGGER IF NOT EXISTS posts_fts_delete AFTER DELETE ON posts BEGIN
    DELETE FROM posts_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS posts_fts_update AFTER UPDATE ON posts BEGIN
    DELETE FROM posts_fts WHERE rowid = old.rowid;
    INSERT INTO posts_fts(rowid, title, content, excerpt)
    VALUES (new.rowid, new.title, new.content, COALESCE(new.excerpt, ''));
END;