-- Migration 005: Create themes and site configuration tables
-- This migration creates the infrastructure for theme management and site configuration

-- Create themes table
CREATE TABLE IF NOT EXISTS themes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    primary_color TEXT NOT NULL,
    secondary_color TEXT NOT NULL,
    background_color TEXT NOT NULL,
    text_color TEXT NOT NULL,
    accent_color TEXT NOT NULL,
    font_family TEXT NOT NULL,
    heading_font TEXT,
    font_size_base TEXT NOT NULL DEFAULT '16px',
    layout TEXT NOT NULL DEFAULT 'sidebar', -- 'single', 'sidebar', 'magazine'
    dark_mode_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    custom_css TEXT,
    header_style TEXT, -- JSON data for header configuration
    footer_style TEXT, -- JSON data for footer configuration
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create site configuration table
CREATE TABLE IF NOT EXISTS site_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_title TEXT NOT NULL,
    site_description TEXT NOT NULL,
    site_logo TEXT,
    favicon TEXT,
    author_name TEXT NOT NULL,
    author_email TEXT,
    author_bio TEXT,
    social_links TEXT, -- JSON array of social links
    google_analytics_id TEXT,
    google_fonts TEXT, -- JSON array of Google Fonts
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create theme templates table for Dropbox sync tracking
CREATE TABLE IF NOT EXISTS theme_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    content_hash TEXT,
    file_type TEXT NOT NULL, -- 'css', 'theme', 'component', 'font'
    last_synced TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_themes_name ON themes (name);
CREATE INDEX IF NOT EXISTS idx_themes_active ON themes (is_active);
CREATE INDEX IF NOT EXISTS idx_themes_layout ON themes (layout);
CREATE INDEX IF NOT EXISTS idx_theme_templates_path ON theme_templates (path);
CREATE INDEX IF NOT EXISTS idx_theme_templates_type ON theme_templates (file_type);

-- Ensure only one active theme at a time
CREATE UNIQUE INDEX IF NOT EXISTS idx_themes_active_unique ON themes (is_active) WHERE is_active = TRUE;

-- Insert default theme if none exists
INSERT OR IGNORE INTO themes (
    name, display_name, description, is_active,
    primary_color, secondary_color, background_color, text_color, accent_color,
    font_family, heading_font, font_size_base, layout, dark_mode_enabled,
    header_style, footer_style, created_at, updated_at
) VALUES (
    'default',
    'Default Theme', 
    'Clean and professional default theme',
    TRUE,
    '#3B82F6',
    '#6366F1', 
    '#FFFFFF',
    '#1F2937',
    '#F59E0B',
    'Inter, system-ui, sans-serif',
    'Inter, system-ui, sans-serif',
    '16px',
    'sidebar',
    TRUE,
    '{"height":"80px","background_color":null,"text_color":null,"logo_position":"left","navigation_style":"horizontal","show_search":true,"sticky":true}',
    '{"background_color":null,"text_color":null,"show_social_links":true,"show_copyright":true,"custom_content":null}',
    datetime('now'),
    datetime('now')
);

-- Insert default site configuration if none exists  
INSERT OR IGNORE INTO site_config (
    site_title, site_description, author_name, social_links, google_fonts, created_at, updated_at
) VALUES (
    'Tobelog',
    'Personal Blog System built with Rust',
    'Blog Author',
    '[]',
    '["Inter:wght@400;500;600;700"]',
    datetime('now'),
    datetime('now')
);