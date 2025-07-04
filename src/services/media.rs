use anyhow::{anyhow, Result};
use axum_extra::extract::multipart::Field;
use chrono::Utc;
use image::{DynamicImage, ImageFormat};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::media::{
    CreateMediaFile, ImageProcessingConfig, MediaConstraints, MediaFile, MediaFilters, MediaType,
};
use crate::services::{BlogStorageService, DatabaseService, DropboxClient};

#[derive(Clone)]
pub struct MediaService {
    dropbox_client: std::sync::Arc<DropboxClient>,
    #[allow(dead_code)]
    blog_storage: std::sync::Arc<BlogStorageService>,
    database: DatabaseService,
    constraints: MediaConstraints,
    image_config: ImageProcessingConfig,
}

impl MediaService {
    pub fn new(
        dropbox_client: std::sync::Arc<DropboxClient>,
        blog_storage: std::sync::Arc<BlogStorageService>,
        database: DatabaseService,
    ) -> Self {
        Self {
            dropbox_client,
            blog_storage,
            database,
            constraints: MediaConstraints::default(),
            image_config: ImageProcessingConfig::default(),
        }
    }

    pub fn with_constraints(mut self, constraints: MediaConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn with_image_config(mut self, config: ImageProcessingConfig) -> Self {
        self.image_config = config;
        self
    }

    /// Upload a media file from multipart field
    pub async fn upload_file(
        &self,
        mut field: Field,
        alt_text: Option<String>,
        caption: Option<String>,
    ) -> Result<MediaFile> {
        // Get field information
        let filename = field
            .file_name()
            .ok_or_else(|| anyhow!("No filename provided"))?
            .to_string();

        let content_type = field
            .content_type()
            .map(|ct| ct.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        info!("Uploading file: {} ({})", filename, content_type);

        // Validate MIME type
        if !self.constraints.allowed_mime_types.contains(&content_type) {
            return Err(anyhow!("File type '{}' not allowed", content_type));
        }

        // Read file data
        let mut file_data = Vec::new();
        while let Some(chunk) = field.chunk().await? {
            file_data.extend_from_slice(&chunk);
        }

        // Validate file size
        if file_data.len() as u64 > self.constraints.max_file_size {
            return Err(anyhow!(
                "File size ({} bytes) exceeds limit ({} bytes)",
                file_data.len(),
                self.constraints.max_file_size
            ));
        }

        // Generate unique filename
        let media_type = MediaType::from_mime_type(&content_type);
        let unique_filename = self.generate_unique_filename(&filename)?;
        
        // Determine folder structure
        let folder_name = media_type.folder_name();
        let now = Utc::now();
        let year = now.format("%Y");
        let month = now.format("%m");
        
        let dropbox_path = format!(
            "/BlogStorage/media/{}/{}/{}/{}",
            folder_name, year, month, unique_filename
        );

        // Process image if it's an image file
        let (processed_data, width, height, thumbnail_data) = 
            if media_type == MediaType::Image {
                self.process_image(&file_data, &content_type).await?
            } else {
                (file_data, None, None, None)
            };

        // Upload main file to Dropbox
        self.upload_to_dropbox(&dropbox_path, &processed_data).await?;

        // Upload thumbnail if generated
        let thumbnail_url = if let Some(thumb_data) = thumbnail_data {
            let thumbnail_path = format!(
                "/BlogStorage/media/thumbnails/{}/{}/{}/thumb_{}",
                year, month, folder_name, unique_filename
            );
            self.upload_to_dropbox(&thumbnail_path, &thumb_data).await?;
            Some(self.generate_media_url(&thumbnail_path))
        } else {
            None
        };

        // Generate public URL
        let media_url = self.generate_media_url(&dropbox_path);

        // Create media file record
        let create_data = CreateMediaFile {
            filename: unique_filename.clone(),
            original_filename: filename,
            dropbox_path: dropbox_path.clone(),
            url: media_url,
            file_size: processed_data.len() as u64,
            mime_type: content_type,
            width,
            height,
            thumbnail_url,
            alt_text,
            caption,
        };

        // Save to database
        let media_file = self.save_to_database(create_data).await?;

        info!("Successfully uploaded file: {}", unique_filename);
        Ok(media_file)
    }

    /// Generate a unique filename to avoid conflicts
    fn generate_unique_filename(&self, original_filename: &str) -> Result<String> {
        let extension = std::path::Path::new(original_filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let base_name = std::path::Path::new(original_filename)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("file");

        // Generate hash from original filename and timestamp
        let mut hasher = Sha256::new();
        hasher.update(original_filename.as_bytes());
        hasher.update(Utc::now().timestamp().to_string().as_bytes());
        hasher.update(Uuid::new_v4().to_string().as_bytes());
        
        let hash = hasher.finalize();
        let hash_str = format!("{:x}", hash)[0..8].to_string();

        let unique_filename = if extension.is_empty() {
            format!("{}_{}", base_name, hash_str)
        } else {
            format!("{}_{}.{}", base_name, hash_str, extension)
        };

        Ok(unique_filename)
    }

    /// Process image: resize, optimize, and generate thumbnail
    async fn process_image(
        &self,
        image_data: &[u8],
        content_type: &str,
    ) -> Result<(Vec<u8>, Option<u32>, Option<u32>, Option<Vec<u8>>)> {
        debug!("Processing image with MIME type: {}", content_type);

        // Parse image
        let img = image::load_from_memory(image_data)
            .map_err(|e| anyhow!("Failed to parse image: {}", e))?;

        let (original_width, original_height) = (img.width(), img.height());
        debug!("Original image dimensions: {}x{}", original_width, original_height);

        // Validate dimensions
        if let Some(max_width) = self.constraints.max_width {
            if original_width > max_width {
                return Err(anyhow!("Image width ({}) exceeds limit ({})", original_width, max_width));
            }
        }
        if let Some(max_height) = self.constraints.max_height {
            if original_height > max_height {
                return Err(anyhow!("Image height ({}) exceeds limit ({})", original_height, max_height));
            }
        }

        // Resize if needed
        let resized_img = self.resize_image_if_needed(img)?;
        let (final_width, final_height) = (resized_img.width(), resized_img.height());

        // Generate main image data
        let main_data = self.encode_image(&resized_img, content_type)?;

        // Generate thumbnail if enabled
        let thumbnail_data = if self.image_config.generate_thumbnail {
            Some(self.generate_thumbnail(&resized_img)?)
        } else {
            None
        };

        Ok((main_data, Some(final_width), Some(final_height), thumbnail_data))
    }

    /// Resize image if it exceeds configured limits
    fn resize_image_if_needed(&self, img: DynamicImage) -> Result<DynamicImage> {
        let (width, height) = (img.width(), img.height());
        
        let needs_resize = if let (Some(max_w), Some(max_h)) = (self.image_config.max_width, self.image_config.max_height) {
            width > max_w || height > max_h
        } else if let Some(max_w) = self.image_config.max_width {
            width > max_w
        } else if let Some(max_h) = self.image_config.max_height {
            height > max_h
        } else {
            false
        };

        if !needs_resize {
            return Ok(img);
        }

        let (target_width, target_height) = self.calculate_target_dimensions(width, height);
        debug!("Resizing image from {}x{} to {}x{}", width, height, target_width, target_height);

        Ok(img.resize(target_width, target_height, image::imageops::FilterType::Lanczos3))
    }

    /// Calculate target dimensions maintaining aspect ratio
    fn calculate_target_dimensions(&self, width: u32, height: u32) -> (u32, u32) {
        let max_width = self.image_config.max_width.unwrap_or(width);
        let max_height = self.image_config.max_height.unwrap_or(height);

        let width_ratio = max_width as f64 / width as f64;
        let height_ratio = max_height as f64 / height as f64;
        let ratio = width_ratio.min(height_ratio);

        if ratio >= 1.0 {
            (width, height)
        } else {
            ((width as f64 * ratio) as u32, (height as f64 * ratio) as u32)
        }
    }

    /// Generate thumbnail image
    fn generate_thumbnail(&self, img: &DynamicImage) -> Result<Vec<u8>> {
        let config = &self.image_config.thumbnail_config;
        
        let thumbnail = img.resize_exact(
            config.width,
            config.height,
            image::imageops::FilterType::Lanczos3,
        );

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        
        thumbnail.write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| anyhow!("Failed to encode thumbnail: {}", e))?;

        Ok(buffer)
    }

    /// Encode image to bytes
    fn encode_image(&self, img: &DynamicImage, original_content_type: &str) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        // Determine output format
        let format = match original_content_type {
            "image/png" => ImageFormat::Png,
            "image/gif" => ImageFormat::Gif,
            "image/webp" => ImageFormat::WebP,
            _ => ImageFormat::Jpeg, // Default to JPEG for other formats
        };

        img.write_to(&mut cursor, format)
            .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

        Ok(buffer)
    }

    /// Upload data to Dropbox
    async fn upload_to_dropbox(&self, path: &str, data: &[u8]) -> Result<()> {
        // Create directory structure if needed
        let parent_dir = std::path::Path::new(path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        if !parent_dir.is_empty() {
            if let Err(e) = self.dropbox_client.create_folder(parent_dir).await {
                warn!("Failed to create directory {}: {}", parent_dir, e);
            }
        }

        // Upload file
        self.dropbox_client.upload_binary_file(path, data).await
            .map_err(|e| anyhow!("Failed to upload to Dropbox: {}", e))?;

        debug!("Uploaded to Dropbox: {}", path);
        Ok(())
    }

    /// Generate public media URL
    fn generate_media_url(&self, dropbox_path: &str) -> String {
        // For now, generate a local serving URL
        // In production, this would be a CDN URL or direct Dropbox link
        format!("/media{}", dropbox_path.strip_prefix("/BlogStorage/media").unwrap_or(dropbox_path))
    }

    /// Save media file to database
    async fn save_to_database(&self, create_data: CreateMediaFile) -> Result<MediaFile> {
        let id = Uuid::new_v4();
        let uploaded_at = Utc::now();

        let media_file = MediaFile {
            id,
            filename: create_data.filename,
            original_filename: create_data.original_filename,
            dropbox_path: create_data.dropbox_path,
            url: create_data.url,
            file_size: create_data.file_size,
            mime_type: create_data.mime_type,
            width: create_data.width,
            height: create_data.height,
            uploaded_at,
            thumbnail_url: create_data.thumbnail_url,
            alt_text: create_data.alt_text,
            caption: create_data.caption,
        };

        // Save to database (implementation will be added with database service)
        self.database.create_media_file(&media_file).await
            .map_err(|e| anyhow!("Failed to save to database: {}", e))?;

        Ok(media_file)
    }

    /// List media files with filtering and pagination
    pub async fn list_media_files(&self, filters: MediaFilters) -> Result<Vec<MediaFile>> {
        self.database.list_media_files(filters).await
            .map_err(|e| anyhow!("Failed to list media files: {}", e))
    }

    /// Get media file count
    pub async fn count_media_files(&self, filters: MediaFilters) -> Result<usize> {
        self.database.count_media_files(filters).await
            .map_err(|e| anyhow!("Failed to count media files: {}", e))
    }

    /// Get media file by ID
    pub async fn get_media_file(&self, id: Uuid) -> Result<Option<MediaFile>> {
        self.database.get_media_file(id).await
            .map_err(|e| anyhow!("Failed to get media file: {}", e))
    }

    /// Delete media file
    pub async fn delete_media_file(&self, id: Uuid) -> Result<bool> {
        let media_file = match self.get_media_file(id).await? {
            Some(file) => file,
            None => return Ok(false),
        };

        // Delete from Dropbox
        if let Err(e) = self.dropbox_client.delete_file(&media_file.dropbox_path).await {
            warn!("Failed to delete file from Dropbox: {}", e);
        }

        // Delete thumbnail if exists
        if let Some(thumbnail_url) = &media_file.thumbnail_url {
            // Convert URL back to Dropbox path for deletion
            // This is a simplified approach; in production, store thumbnail path separately
            let thumbnail_path = thumbnail_url.replace("/media", "/BlogStorage/media");
            if let Err(e) = self.dropbox_client.delete_file(&thumbnail_path).await {
                warn!("Failed to delete thumbnail from Dropbox: {}", e);
            }
        }

        // Delete from database
        self.database.delete_media_file(id).await
            .map_err(|e| anyhow!("Failed to delete from database: {}", e))?;

        info!("Deleted media file: {}", media_file.filename);
        Ok(true)
    }

    /// Serve media file from Dropbox
    pub async fn serve_media_file(&self, path: &str) -> Result<(Vec<u8>, String)> {
        let dropbox_path = format!("/BlogStorage/media{}", path);
        
        let data = self.dropbox_client.download_file(&dropbox_path).await
            .map_err(|e| anyhow!("Failed to download from Dropbox: {}", e))?;

        // Determine MIME type from file extension
        let mime_type = self.get_mime_type_from_path(path);

        Ok((data, mime_type))
    }

    /// Get MIME type from file path
    fn get_mime_type_from_path(&self, path: &str) -> String {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "ogg" => "video/ogg",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "pdf" => "application/pdf",
            "txt" => "text/plain",
            "md" => "text/markdown",
            _ => "application/octet-stream",
        }.to_string()
    }
}