use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Media file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub dropbox_path: String,
    pub url: String,
    pub file_size: u64,
    pub mime_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub uploaded_at: DateTime<Utc>,
    pub thumbnail_url: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
}

/// Response for media upload
#[derive(Debug, Serialize)]
pub struct MediaUploadResponse {
    pub success: bool,
    pub message: String,
    pub media: Option<MediaFile>,
    pub errors: Option<Vec<String>>,
}

/// Response for media list
#[derive(Debug, Serialize)]
pub struct MediaListResponse {
    pub media: Vec<MediaFile>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// Query parameters for media listing
#[derive(Debug, Deserialize)]
pub struct MediaQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub folder: Option<String>,
    pub mime_type: Option<String>,
    pub search: Option<String>,
}

/// Media file filters for database queries
#[derive(Debug, Clone, Default)]
pub struct MediaFilters {
    pub folder: Option<String>,
    pub mime_type: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Media file creation data
#[derive(Debug, Clone)]
pub struct CreateMediaFile {
    pub filename: String,
    pub original_filename: String,
    pub dropbox_path: String,
    pub url: String,
    pub file_size: u64,
    pub mime_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
}

/// Supported media file types
#[derive(Debug, Clone, PartialEq)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Document,
    Other,
}

impl MediaType {
    pub fn from_mime_type(mime: &str) -> Self {
        match mime.split('/').next().unwrap_or("") {
            "image" => MediaType::Image,
            "video" => MediaType::Video,
            "audio" => MediaType::Audio,
            "application" | "text" => MediaType::Document,
            _ => MediaType::Other,
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            MediaType::Image => "images",
            MediaType::Video => "videos",
            MediaType::Audio => "audio",
            MediaType::Document => "documents",
            MediaType::Other => "other",
        }
    }
}

/// Thumbnail generation configuration
#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    pub width: u32,
    pub height: u32,
    pub quality: u8,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            width: 300,
            height: 300,
            quality: 85,
        }
    }
}

/// Image processing configuration
#[derive(Debug, Clone)]
pub struct ImageProcessingConfig {
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub quality: u8,
    pub generate_thumbnail: bool,
    pub thumbnail_config: ThumbnailConfig,
}

impl Default for ImageProcessingConfig {
    fn default() -> Self {
        Self {
            max_width: Some(1920),
            max_height: Some(1080),
            quality: 85,
            generate_thumbnail: true,
            thumbnail_config: ThumbnailConfig::default(),
        }
    }
}

/// Media upload constraints
#[derive(Debug, Clone)]
pub struct MediaConstraints {
    pub max_file_size: u64, // in bytes
    pub allowed_mime_types: Vec<String>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

impl Default for MediaConstraints {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            allowed_mime_types: vec![
                // Images
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "image/svg+xml".to_string(),
                // Videos
                "video/mp4".to_string(),
                "video/webm".to_string(),
                "video/ogg".to_string(),
                // Audio
                "audio/mpeg".to_string(),
                "audio/wav".to_string(),
                "audio/ogg".to_string(),
                // Documents
                "application/pdf".to_string(),
                "text/plain".to_string(),
                "text/markdown".to_string(),
            ],
            max_width: Some(3840),  // 4K width
            max_height: Some(2160), // 4K height
        }
    }
}
