use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata extracted from markdown frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMetadata {
    pub title: String,
    pub slug: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub published: bool,
    pub featured: bool,
    pub author: Option<String>,
    pub excerpt: Option<String>,
    pub media: Vec<String>,
    pub version: i32,
    pub custom_fields: HashMap<String, serde_yaml::Value>,
}

/// Blog configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogConfig {
    pub title: String,
    pub description: String,
    pub author: String,
    pub base_url: String,
    pub theme: String,
    pub posts_per_page: usize,
    pub excerpt_length: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Site metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteMetadata {
    pub total_posts: i64,
    pub published_posts: i64,
    pub total_categories: i64,
    pub total_tags: i64,
    pub last_updated: DateTime<Utc>,
    pub categories: Vec<CategoryMeta>,
    pub tags: Vec<TagMeta>,
    pub recent_posts: Vec<MetaPostSummary>,
}

/// Category metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMeta {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
    pub created_at: DateTime<Utc>,
}

/// Tag metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagMeta {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
    pub created_at: DateTime<Utc>,
}

/// Post summary for metadata (renamed to avoid conflict)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaPostSummary {
    pub slug: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Media file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub path: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PostMetadata {
    /// Create new post metadata with default values
    #[allow(dead_code)]
    pub fn new(title: String, slug: String) -> Self {
        let now = Utc::now();
        Self {
            title,
            slug,
            created_at: Some(now),
            updated_at: Some(now),
            published_at: None,
            category: None,
            tags: Vec::new(),
            published: false,
            featured: false,
            author: None,
            excerpt: None,
            media: Vec::new(),
            version: 1,
            custom_fields: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    /// Create from frontmatter data
    pub fn from_frontmatter(
        frontmatter: &HashMap<String, serde_yaml::Value>,
        slug: String,
    ) -> Self {
        let now = Utc::now();

        let title = frontmatter
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        let tags = frontmatter
            .get("tags")
            .and_then(|v| {
                if let Ok(tags) = serde_yaml::from_value::<Vec<String>>(v.clone()) {
                    Some(tags)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let category = frontmatter
            .get("category")
            .and_then(|v| v.as_str())
            .map(String::from);

        let published = frontmatter
            .get("published")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let featured = frontmatter
            .get("featured")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let author = frontmatter
            .get("author")
            .and_then(|v| v.as_str())
            .map(String::from);

        let excerpt = frontmatter
            .get("excerpt")
            .and_then(|v| v.as_str())
            .map(String::from);

        let media = frontmatter
            .get("media")
            .and_then(|v| {
                if let Ok(media) = serde_yaml::from_value::<Vec<String>>(v.clone()) {
                    Some(media)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let created_at = frontmatter
            .get("created_at")
            .and_then(|v| {
                if let Ok(dt) = serde_yaml::from_value::<DateTime<Utc>>(v.clone()) {
                    Some(dt)
                } else {
                    None
                }
            })
            .or(Some(now));

        let updated_at = frontmatter
            .get("updated_at")
            .and_then(|v| {
                if let Ok(dt) = serde_yaml::from_value::<DateTime<Utc>>(v.clone()) {
                    Some(dt)
                } else {
                    None
                }
            })
            .or(Some(now));

        let published_at = if published {
            frontmatter
                .get("published_at")
                .and_then(|v| {
                    if let Ok(dt) = serde_yaml::from_value::<DateTime<Utc>>(v.clone()) {
                        Some(dt)
                    } else {
                        None
                    }
                })
                .or(created_at)
        } else {
            None
        };

        let version = frontmatter
            .get("version")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        // Collect custom fields (anything not in standard fields)
        let standard_fields = [
            "title",
            "tags",
            "category",
            "published",
            "featured",
            "author",
            "excerpt",
            "media",
            "created_at",
            "updated_at",
            "published_at",
            "version",
        ];

        let custom_fields = frontmatter
            .iter()
            .filter(|(key, _)| !standard_fields.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        Self {
            title,
            slug,
            created_at,
            updated_at,
            published_at,
            category,
            tags,
            published,
            featured,
            author,
            excerpt,
            media,
            version,
            custom_fields,
        }
    }

    /// Convert to frontmatter HashMap
    #[allow(dead_code)]
    pub fn to_frontmatter(&self) -> HashMap<String, serde_yaml::Value> {
        let mut frontmatter = HashMap::new();

        frontmatter.insert(
            "title".to_string(),
            serde_yaml::Value::String(self.title.clone()),
        );

        if let Some(created_at) = self.created_at {
            frontmatter.insert(
                "created_at".to_string(),
                serde_yaml::to_value(created_at).unwrap_or(serde_yaml::Value::Null),
            );
        }

        if let Some(updated_at) = self.updated_at {
            frontmatter.insert(
                "updated_at".to_string(),
                serde_yaml::to_value(updated_at).unwrap_or(serde_yaml::Value::Null),
            );
        }

        if let Some(published_at) = self.published_at {
            frontmatter.insert(
                "published_at".to_string(),
                serde_yaml::to_value(published_at).unwrap_or(serde_yaml::Value::Null),
            );
        }

        if let Some(category) = &self.category {
            frontmatter.insert(
                "category".to_string(),
                serde_yaml::Value::String(category.clone()),
            );
        }

        if !self.tags.is_empty() {
            frontmatter.insert(
                "tags".to_string(),
                serde_yaml::to_value(&self.tags).unwrap_or(serde_yaml::Value::Null),
            );
        }

        frontmatter.insert(
            "published".to_string(),
            serde_yaml::Value::Bool(self.published),
        );

        if self.featured {
            frontmatter.insert(
                "featured".to_string(),
                serde_yaml::Value::Bool(self.featured),
            );
        }

        if let Some(author) = &self.author {
            frontmatter.insert(
                "author".to_string(),
                serde_yaml::Value::String(author.clone()),
            );
        }

        if let Some(excerpt) = &self.excerpt {
            frontmatter.insert(
                "excerpt".to_string(),
                serde_yaml::Value::String(excerpt.clone()),
            );
        }

        if !self.media.is_empty() {
            frontmatter.insert(
                "media".to_string(),
                serde_yaml::to_value(&self.media).unwrap_or(serde_yaml::Value::Null),
            );
        }

        if self.version > 1 {
            frontmatter.insert(
                "version".to_string(),
                serde_yaml::Value::Number(self.version.into()),
            );
        }

        // Add custom fields
        for (key, value) in &self.custom_fields {
            frontmatter.insert(key.clone(), value.clone());
        }

        frontmatter
    }

    /// Update metadata
    #[allow(dead_code)]
    pub fn update(&mut self) {
        self.updated_at = Some(Utc::now());
        self.version += 1;
    }

    /// Mark as published
    #[allow(dead_code)]
    pub fn publish(&mut self) {
        if !self.published {
            self.published = true;
            self.published_at = Some(Utc::now());
            self.update();
        }
    }

    /// Mark as unpublished
    #[allow(dead_code)]
    pub fn unpublish(&mut self) {
        if self.published {
            self.published = false;
            self.published_at = None;
            self.update();
        }
    }
}

impl Default for BlogConfig {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            title: "My Blog".to_string(),
            description: "A personal blog".to_string(),
            author: "Blog Author".to_string(),
            base_url: "https://example.com".to_string(),
            theme: "default".to_string(),
            posts_per_page: 10,
            excerpt_length: 50,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_metadata_creation() {
        let metadata = PostMetadata::new("Test Post".to_string(), "test-post".to_string());

        assert_eq!(metadata.title, "Test Post");
        assert_eq!(metadata.slug, "test-post");
        assert!(!metadata.published);
        assert_eq!(metadata.version, 1);
        assert!(metadata.created_at.is_some());
    }

    #[test]
    fn test_from_frontmatter() {
        let mut frontmatter = HashMap::new();
        frontmatter.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test Title".to_string()),
        );
        frontmatter.insert("published".to_string(), serde_yaml::Value::Bool(true));
        frontmatter.insert(
            "tags".to_string(),
            serde_yaml::to_value(vec!["rust", "blog"]).unwrap(),
        );

        let metadata = PostMetadata::from_frontmatter(&frontmatter, "test-slug".to_string());

        assert_eq!(metadata.title, "Test Title");
        assert_eq!(metadata.slug, "test-slug");
        assert!(metadata.published);
        assert_eq!(metadata.tags, vec!["rust", "blog"]);
    }

    #[test]
    fn test_publish_unpublish() {
        let mut metadata = PostMetadata::new("Test".to_string(), "test".to_string());

        assert!(!metadata.published);
        assert!(metadata.published_at.is_none());

        metadata.publish();
        assert!(metadata.published);
        assert!(metadata.published_at.is_some());

        metadata.unpublish();
        assert!(!metadata.published);
        assert!(metadata.published_at.is_none());
    }
}
