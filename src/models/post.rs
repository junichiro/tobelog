use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Blog post entity for database storage
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub html_content: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: String, // JSON array stored as string
    pub published: bool,
    pub featured: bool,
    pub author: Option<String>,
    pub dropbox_path: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Post creation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePost {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub html_content: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub published: bool,
    pub featured: bool,
    pub author: Option<String>,
    pub dropbox_path: String,
}

/// Post update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePost {
    pub title: Option<String>,
    pub content: Option<String>,
    pub html_content: Option<String>,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub published: Option<bool>,
    pub featured: Option<bool>,
    pub author: Option<String>,
    pub dropbox_path: Option<String>,
}

/// Post query filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PostFilters {
    pub published: Option<bool>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub author: Option<String>,
    pub featured: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Post statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostStats {
    pub total_posts: i64,
    pub published_posts: i64,
    pub draft_posts: i64,
    pub featured_posts: i64,
    pub categories: Vec<CategoryStat>,
    pub tags: Vec<TagStat>,
}

/// Category statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStat {
    pub name: String,
    pub count: i64,
}

/// Tag statistics  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStat {
    pub name: String,
    pub count: i64,
}

impl Post {
    /// Create a new post with generated UUID and timestamps
    pub fn new(data: CreatePost) -> Self {
        let now = Utc::now();
        let published_at = if data.published { Some(now) } else { None };

        Self {
            id: Uuid::new_v4(),
            slug: data.slug,
            title: data.title,
            content: data.content,
            html_content: data.html_content,
            excerpt: data.excerpt,
            category: data.category,
            tags: serde_json::to_string(&data.tags).unwrap_or_default(),
            published: data.published,
            featured: data.featured,
            author: data.author,
            dropbox_path: data.dropbox_path,
            version: 1,
            created_at: now,
            updated_at: now,
            published_at,
        }
    }

    /// Get tags as a vector
    pub fn get_tags(&self) -> Vec<String> {
        serde_json::from_str(&self.tags).unwrap_or_default()
    }

    /// Set tags from a vector
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = serde_json::to_string(&tags).unwrap_or_default();
    }

    /// Update post data
    pub fn update(&mut self, data: UpdatePost) {
        if let Some(title) = data.title {
            self.title = title;
        }
        if let Some(content) = data.content {
            self.content = content;
        }
        if let Some(html_content) = data.html_content {
            self.html_content = html_content;
        }
        if let Some(excerpt) = data.excerpt {
            self.excerpt = Some(excerpt);
        }
        if let Some(category) = data.category {
            self.category = Some(category);
        }
        if let Some(tags) = data.tags {
            self.set_tags(tags);
        }
        if let Some(published) = data.published {
            if published && !self.published {
                self.published_at = Some(Utc::now());
            } else if !published {
                self.published_at = None;
            }
            self.published = published;
        }
        if let Some(featured) = data.featured {
            self.featured = featured;
        }
        if let Some(author) = data.author {
            self.author = Some(author);
        }
        if let Some(dropbox_path) = data.dropbox_path {
            self.dropbox_path = dropbox_path;
        }

        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Check if post is published
    pub fn is_published(&self) -> bool {
        self.published
    }

    /// Check if post is draft
    pub fn is_draft(&self) -> bool {
        !self.published
    }

    /// Get URL-friendly path
    pub fn get_url_path(&self) -> String {
        let year = self.created_at.format("%Y");
        format!("/posts/{}/{}", year, self.slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_creation() {
        let create_data = CreatePost {
            slug: "test-post".to_string(),
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            html_content: "<p>Test content</p>".to_string(),
            excerpt: Some("Test excerpt".to_string()),
            category: Some("tech".to_string()),
            tags: vec!["rust".to_string(), "blog".to_string()],
            published: true,
            featured: false,
            author: Some("Test Author".to_string()),
            dropbox_path: "/posts/test.md".to_string(),
        };

        let post = Post::new(create_data);

        assert_eq!(post.slug, "test-post");
        assert_eq!(post.title, "Test Post");
        assert!(post.published);
        assert!(post.published_at.is_some());
        assert_eq!(post.version, 1);
        assert_eq!(post.get_tags(), vec!["rust", "blog"]);
    }

    #[test]
    fn test_post_update() {
        let create_data = CreatePost {
            slug: "test-post".to_string(),
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            html_content: "<p>Test content</p>".to_string(),
            excerpt: None,
            category: None,
            tags: vec![],
            published: false,
            featured: false,
            author: None,
            dropbox_path: "/posts/test.md".to_string(),
        };

        let mut post = Post::new(create_data);
        let original_updated_at = post.updated_at;

        // Wait a moment to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        let update_data = UpdatePost {
            title: Some("Updated Title".to_string()),
            published: Some(true),
            tags: Some(vec!["updated".to_string()]),
            ..Default::default()
        };

        post.update(update_data);

        assert_eq!(post.title, "Updated Title");
        assert!(post.published);
        assert!(post.published_at.is_some());
        assert_eq!(post.version, 2);
        assert!(post.updated_at > original_updated_at);
        assert_eq!(post.get_tags(), vec!["updated"]);
    }

    #[test]
    fn test_url_path_generation() {
        let create_data = CreatePost {
            slug: "hello-world".to_string(),
            title: "Hello World".to_string(),
            content: "Content".to_string(),
            html_content: "<p>Content</p>".to_string(),
            excerpt: None,
            category: None,
            tags: vec![],
            published: true,
            featured: false,
            author: None,
            dropbox_path: "/posts/hello.md".to_string(),
        };

        let post = Post::new(create_data);
        let url_path = post.get_url_path();
        
        assert!(url_path.starts_with("/posts/"));
        assert!(url_path.ends_with("/hello-world"));
    }
}

impl Default for UpdatePost {
    fn default() -> Self {
        Self {
            title: None,
            content: None,
            html_content: None,
            excerpt: None,
            category: None,
            tags: None,
            published: None,
            featured: None,
            author: None,
            dropbox_path: None,
        }
    }
}