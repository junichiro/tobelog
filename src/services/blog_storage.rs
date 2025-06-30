use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

use super::dropbox::{DropboxClient, FileMetadata};

/// Blog post metadata extracted from markdown frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostMetadata {
    pub title: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub published: bool,
    pub author: Option<String>,
    pub excerpt: Option<String>,
}

/// Complete blog post with content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPost {
    pub metadata: BlogPostMetadata,
    pub content: String,
    pub dropbox_path: String,
    pub file_metadata: Option<FileMetadata>,
}

/// Blog folder structure management
#[derive(Debug, Clone)]
pub struct BlogFolders {
    pub posts: String,
    pub drafts: String,
    pub media: String,
    pub templates: String,
    pub config: String,
}

impl Default for BlogFolders {
    fn default() -> Self {
        Self {
            posts: "/BlogStorage/posts".to_string(),
            drafts: "/BlogStorage/drafts".to_string(),
            media: "/BlogStorage/media".to_string(),
            templates: "/BlogStorage/templates".to_string(),
            config: "/BlogStorage/config".to_string(),
        }
    }
}

/// Rate limiting state for Dropbox API
#[derive(Debug)]
struct RateLimiter {
    requests: Vec<Instant>,
    max_requests: usize,
    time_window: Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, time_window: Duration) -> Self {
        Self {
            requests: Vec::new(),
            max_requests,
            time_window,
        }
    }

    async fn check_rate_limit(&mut self) -> Result<()> {
        let now = Instant::now();
        
        // Remove old requests outside the time window
        self.requests.retain(|&request_time| now.duration_since(request_time) < self.time_window);
        
        // Check if we're at the rate limit
        if self.requests.len() >= self.max_requests {
            let oldest_request = self.requests[0];
            let wait_time = self.time_window - now.duration_since(oldest_request);
            
            warn!("Rate limit reached, waiting {:?}", wait_time);
            tokio::time::sleep(wait_time).await;
            
            // Remove the oldest request after waiting
            self.requests.remove(0);
        }
        
        self.requests.push(now);
        Ok(())
    }
}

/// High-level blog storage service using Dropbox
#[derive(Clone)]
pub struct BlogStorageService {
    dropbox_client: Arc<DropboxClient>,
    folders: BlogFolders,
    rate_limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
}

impl BlogStorageService {
    /// Create a new blog storage service
    pub fn new(dropbox_client: Arc<DropboxClient>) -> Self {
        // Dropbox allows 500 requests per minute
        let rate_limiter = RateLimiter::new(450, Duration::from_secs(60)); // Leave some buffer
        
        Self {
            dropbox_client,
            folders: BlogFolders::default(),
            rate_limiter: Arc::new(tokio::sync::Mutex::new(rate_limiter)),
        }
    }

    /// Create a new service with custom folder configuration
    pub fn with_folders(dropbox_client: Arc<DropboxClient>, folders: BlogFolders) -> Self {
        let rate_limiter = RateLimiter::new(450, Duration::from_secs(60));
        
        Self {
            dropbox_client,
            folders,
            rate_limiter: Arc::new(tokio::sync::Mutex::new(rate_limiter)),
        }
    }

    /// Check and wait for rate limit if necessary
    async fn check_rate_limit(&self) -> Result<()> {
        let mut limiter = self.rate_limiter.lock().await;
        limiter.check_rate_limit().await
    }

    /// Initialize blog folder structure in Dropbox
    pub async fn initialize_blog_structure(&self) -> Result<()> {
        info!("Initializing blog folder structure...");
        
        let images_folder = format!("{}/images", self.folders.media);
        let videos_folder = format!("{}/videos", self.folders.media);
        
        let folders = vec![
            &self.folders.posts,
            &self.folders.drafts,
            &self.folders.media,
            &self.folders.templates,
            &self.folders.config,
            &images_folder,
            &videos_folder,
        ];

        for folder in folders {
            self.check_rate_limit().await?;
            
            match self.dropbox_client.list_folder(folder).await {
                Ok(_) => {
                    debug!("Folder already exists: {}", folder);
                }
                Err(_) => {
                    info!("Creating folder: {}", folder);
                    self.dropbox_client.create_folder(folder).await
                        .with_context(|| format!("Failed to create folder: {}", folder))?;
                }
            }
        }

        info!("Blog folder structure initialized successfully");
        Ok(())
    }

    /// List all published blog posts
    pub async fn list_published_posts(&self) -> Result<Vec<BlogPost>> {
        self.check_rate_limit().await?;
        
        info!("Listing published blog posts from {}", self.folders.posts);
        
        let folder_result = self.dropbox_client.list_folder(&self.folders.posts).await
            .with_context(|| format!("Failed to list posts folder: {}", self.folders.posts))?;

        let mut posts = Vec::new();
        
        for entry in folder_result.entries {
            if entry.name.ends_with(".md") || entry.name.ends_with(".markdown") {
                debug!("Found markdown file: {}", entry.name);
                
                match self.load_blog_post_from_file(&entry).await {
                    Ok(Some(post)) if post.metadata.published => {
                        posts.push(post);
                    }
                    Ok(Some(_)) => {
                        debug!("Skipping unpublished post: {}", entry.name);
                    }
                    Ok(None) => {
                        debug!("Skipping invalid post file: {}", entry.name);
                    }
                    Err(e) => {
                        warn!("Failed to load post {}: {}", entry.name, e);
                    }
                }
            }
        }

        // Sort posts by creation date (newest first)
        posts.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));
        
        info!("Found {} published posts", posts.len());
        Ok(posts)
    }

    /// List all draft blog posts
    pub async fn list_draft_posts(&self) -> Result<Vec<BlogPost>> {
        self.check_rate_limit().await?;
        
        info!("Listing draft blog posts from {}", self.folders.drafts);
        
        let folder_result = self.dropbox_client.list_folder(&self.folders.drafts).await
            .with_context(|| format!("Failed to list drafts folder: {}", self.folders.drafts))?;

        let mut posts = Vec::new();
        
        for entry in folder_result.entries {
            if entry.name.ends_with(".md") || entry.name.ends_with(".markdown") {
                debug!("Found draft file: {}", entry.name);
                
                match self.load_blog_post_from_file(&entry).await {
                    Ok(Some(post)) => {
                        posts.push(post);
                    }
                    Ok(None) => {
                        debug!("Skipping invalid draft file: {}", entry.name);
                    }
                    Err(e) => {
                        warn!("Failed to load draft {}: {}", entry.name, e);
                    }
                }
            }
        }

        // Sort drafts by update date (newest first)
        posts.sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));
        
        info!("Found {} draft posts", posts.len());
        Ok(posts)
    }

    /// Get a specific blog post by slug
    pub async fn get_post_by_slug(&self, slug: &str) -> Result<Option<BlogPost>> {
        info!("Looking for post with slug: {}", slug);
        
        // Search in published posts first
        let published_posts = self.list_published_posts().await?;
        if let Some(post) = published_posts.iter().find(|p| p.metadata.slug == slug) {
            return Ok(Some(post.clone()));
        }

        // Search in drafts if not found in published
        let draft_posts = self.list_draft_posts().await?;
        if let Some(post) = draft_posts.iter().find(|p| p.metadata.slug == slug) {
            return Ok(Some(post.clone()));
        }

        Ok(None)
    }

    /// Save a blog post (create or update)
    pub async fn save_post(&self, post: &BlogPost, is_draft: bool) -> Result<()> {
        self.check_rate_limit().await?;
        
        let folder = if is_draft { &self.folders.drafts } else { &self.folders.posts };
        let file_path = format!("{}/{}.md", folder, post.metadata.slug);
        
        let content = self.serialize_blog_post(post)?;
        
        info!("Saving post '{}' to {}", post.metadata.title, file_path);
        
        self.dropbox_client.upload_file(&file_path, &content).await
            .with_context(|| format!("Failed to save post to {}", file_path))?;
        
        info!("Post saved successfully");
        Ok(())
    }

    /// Delete a blog post
    pub async fn delete_post(&self, slug: &str) -> Result<bool> {
        self.check_rate_limit().await?;
        
        info!("Deleting post with slug: {}", slug);
        
        // Try to find and delete from published posts
        let published_path = format!("{}/{}.md", self.folders.posts, slug);
        if let Ok(_) = self.dropbox_client.delete_file(&published_path).await {
            info!("Deleted published post: {}", published_path);
            return Ok(true);
        }

        // Try to find and delete from drafts
        let draft_path = format!("{}/{}.md", self.folders.drafts, slug);
        if let Ok(_) = self.dropbox_client.delete_file(&draft_path).await {
            info!("Deleted draft post: {}", draft_path);
            return Ok(true);
        }

        warn!("Post with slug '{}' not found", slug);
        Ok(false)
    }

    /// Move a post between drafts and published
    pub async fn publish_post(&self, slug: &str) -> Result<bool> {
        info!("Publishing post with slug: {}", slug);
        
        // Load from drafts
        if let Some(mut post) = self.get_post_by_slug(slug).await? {
            if post.dropbox_path.contains(&self.folders.drafts) {
                // Update metadata
                post.metadata.published = true;
                post.metadata.updated_at = Utc::now();
                
                // Save to published folder
                self.save_post(&post, false).await?;
                
                // Delete from drafts
                let draft_path = format!("{}/{}.md", self.folders.drafts, slug);
                if let Ok(_) = self.dropbox_client.delete_file(&draft_path).await {
                    info!("Post '{}' published successfully", slug);
                    return Ok(true);
                }
            }
        }

        warn!("Could not publish post with slug '{}'", slug);
        Ok(false)
    }

    /// Load blog post from file metadata
    async fn load_blog_post_from_file(&self, file_metadata: &FileMetadata) -> Result<Option<BlogPost>> {
        self.check_rate_limit().await?;
        
        let content = self.dropbox_client.download_file(&file_metadata.path_display).await
            .with_context(|| format!("Failed to download file: {}", file_metadata.path_display))?;

        self.parse_blog_post(&content, file_metadata)
    }

    /// Parse markdown content into BlogPost
    fn parse_blog_post(&self, content: &str, file_metadata: &FileMetadata) -> Result<Option<BlogPost>> {
        let (frontmatter, body) = self.extract_frontmatter(content)?;
        
        let metadata = match frontmatter {
            Some(fm) => self.parse_frontmatter(&fm, file_metadata)?,
            None => {
                debug!("No frontmatter found in {}", file_metadata.name);
                return Ok(None);
            }
        };

        Ok(Some(BlogPost {
            metadata,
            content: body,
            dropbox_path: file_metadata.path_display.clone(),
            file_metadata: Some(file_metadata.clone()),
        }))
    }

    /// Extract frontmatter from markdown content
    fn extract_frontmatter(&self, content: &str) -> Result<(Option<String>, String)> {
        if !content.starts_with("---") {
            return Ok((None, content.to_string()));
        }

        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 3 {
            return Ok((None, content.to_string()));
        }

        // Find the closing ---
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                let frontmatter = lines[1..i].join("\n");
                let body = lines[i+1..].join("\n");
                return Ok((Some(frontmatter), body));
            }
        }

        Ok((None, content.to_string()))
    }

    /// Parse YAML frontmatter into metadata
    fn parse_frontmatter(&self, frontmatter: &str, file_metadata: &FileMetadata) -> Result<BlogPostMetadata> {
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(frontmatter)
            .context("Failed to parse YAML frontmatter")?;

        let yaml_map = yaml_value.as_mapping()
            .ok_or_else(|| anyhow::anyhow!("Frontmatter must be a YAML mapping"))?;

        let title = yaml_map.get(&serde_yaml::Value::String("title".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or(&file_metadata.name.replace(".md", ""))
            .to_string();

        let slug = yaml_map.get(&serde_yaml::Value::String("slug".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.generate_slug(&title))
            .to_string();

        let created_at = yaml_map.get(&serde_yaml::Value::String("created_at".to_string()))
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let updated_at = yaml_map.get(&serde_yaml::Value::String("updated_at".to_string()))
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(created_at);

        let category = yaml_map.get(&serde_yaml::Value::String("category".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = yaml_map.get(&serde_yaml::Value::String("tags".to_string()))
            .and_then(|v| v.as_sequence())
            .map(|seq| seq.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let published = yaml_map.get(&serde_yaml::Value::String("published".to_string()))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let author = yaml_map.get(&serde_yaml::Value::String("author".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let excerpt = yaml_map.get(&serde_yaml::Value::String("excerpt".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(BlogPostMetadata {
            title,
            slug,
            created_at,
            updated_at,
            category,
            tags,
            published,
            author,
            excerpt,
        })
    }

    /// Serialize blog post to markdown with frontmatter
    fn serialize_blog_post(&self, post: &BlogPost) -> Result<String> {
        let mut frontmatter = serde_yaml::to_string(&post.metadata)
            .context("Failed to serialize metadata to YAML")?;

        // Clean up the YAML output
        frontmatter = frontmatter.trim().to_string();

        Ok(format!("---\n{}\n---\n\n{}", frontmatter, post.content))
    }

    /// Generate a URL-friendly slug from title
    fn generate_slug(&self, title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' => c,
                ' ' | '_' => '-',
                _ => '-',
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Get blog statistics
    pub async fn get_blog_stats(&self) -> Result<serde_json::Map<String, serde_json::Value>> {
        let published = self.list_published_posts().await?;
        let drafts = self.list_draft_posts().await?;

        let mut categories = HashMap::new();
        let mut tags = HashMap::new();

        for post in &published {
            if let Some(ref category) = post.metadata.category {
                *categories.entry(category.clone()).or_insert(0) += 1;
            }
            for tag in &post.metadata.tags {
                *tags.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        Ok(serde_json::json!({
            "published_posts": published.len(),
            "draft_posts": drafts.len(),
            "categories": categories,
            "tags": tags,
            "last_updated": Utc::now().to_rfc3339()
        }).as_object().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_service() -> BlogStorageService {
        let dropbox_client = Arc::new(DropboxClient::new("test_token".to_string()));
        BlogStorageService::new(dropbox_client)
    }

    #[test]
    fn test_generate_slug() {
        let service = create_test_service();
        
        assert_eq!(service.generate_slug("Hello World"), "hello-world");
        assert_eq!(service.generate_slug("This is a Test!"), "this-is-a-test");
        assert_eq!(service.generate_slug("Special Characters @#$%"), "special-characters");
        assert_eq!(service.generate_slug("Multiple   Spaces"), "multiple-spaces");
    }

    #[test]
    fn test_extract_frontmatter() {
        let service = create_test_service();
        
        let content_with_frontmatter = r#"---
title: Test Post
published: true
---

This is the content."#;

        let (frontmatter, body) = service.extract_frontmatter(content_with_frontmatter).unwrap();
        assert!(frontmatter.is_some());
        assert_eq!(body.trim(), "This is the content.");

        let content_without_frontmatter = "Just content here.";
        let (frontmatter, body) = service.extract_frontmatter(content_without_frontmatter).unwrap();
        assert!(frontmatter.is_none());
        assert_eq!(body, content_without_frontmatter);
    }

    #[test]
    fn test_serialize_blog_post() {
        let service = create_test_service();
        
        let post = BlogPost {
            metadata: BlogPostMetadata {
                title: "Test Post".to_string(),
                slug: "test-post".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                category: Some("tech".to_string()),
                tags: vec!["rust".to_string(), "blog".to_string()],
                published: true,
                author: Some("Test Author".to_string()),
                excerpt: None,
            },
            content: "This is the post content.".to_string(),
            dropbox_path: "/test/path".to_string(),
            file_metadata: None,
        };

        let serialized = service.serialize_blog_post(&post).unwrap();
        assert!(serialized.starts_with("---"));
        assert!(serialized.contains("title: Test Post"));
        assert!(serialized.contains("This is the post content."));
    }
}