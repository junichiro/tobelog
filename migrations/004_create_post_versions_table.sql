-- Migration 004: Create post versions table for version history management
-- This migration creates the infrastructure for storing and managing post version history

CREATE TABLE IF NOT EXISTS post_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    html_content TEXT NOT NULL,
    excerpt TEXT,
    category TEXT,
    tags TEXT, -- JSON array of tags
    metadata TEXT, -- JSON metadata
    change_summary TEXT, -- Brief description of changes
    created_at TEXT NOT NULL,
    created_by TEXT,
    
    -- Foreign key constraint
    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE,
    
    -- Unique constraint on post_id + version combination
    UNIQUE (post_id, version)
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_post_versions_post_id ON post_versions (post_id);
CREATE INDEX IF NOT EXISTS idx_post_versions_version ON post_versions (post_id, version);
CREATE INDEX IF NOT EXISTS idx_post_versions_created_at ON post_versions (created_at);

-- Create an index for finding the latest version of each post
CREATE INDEX IF NOT EXISTS idx_post_versions_latest ON post_versions (post_id, version DESC);