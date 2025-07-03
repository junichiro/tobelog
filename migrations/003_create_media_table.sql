-- Migration 003: Create media files table
CREATE TABLE IF NOT EXISTS media_files (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    dropbox_path TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mime_type TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    uploaded_at TEXT NOT NULL,
    thumbnail_url TEXT,
    alt_text TEXT,
    caption TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_media_files_mime_type ON media_files(mime_type);
CREATE INDEX IF NOT EXISTS idx_media_files_uploaded_at ON media_files(uploaded_at);
CREATE INDEX IF NOT EXISTS idx_media_files_filename ON media_files(filename);

-- Create posts_media junction table for many-to-many relationship
CREATE TABLE IF NOT EXISTS posts_media (
    post_id TEXT NOT NULL,
    media_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (post_id, media_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (media_id) REFERENCES media_files(id) ON DELETE CASCADE
);

-- Create index for posts_media queries
CREATE INDEX IF NOT EXISTS idx_posts_media_post_id ON posts_media(post_id);
CREATE INDEX IF NOT EXISTS idx_posts_media_media_id ON posts_media(media_id);