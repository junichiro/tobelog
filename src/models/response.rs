use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Response model for individual post details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    pub id: Uuid,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub url_path: String,
}

/// Summary model for post listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSummary {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub featured: bool,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub url_path: String,
}

/// Response model for post list pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostListResponse {
    pub posts: Vec<PostSummary>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// Response model for API errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

/// Response model for blog statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogStatsResponse {
    pub total_posts: i64,
    pub published_posts: i64,
    pub draft_posts: i64,
    pub featured_posts: i64,
    pub categories: Vec<CategoryInfo>,
    pub tags: Vec<TagInfo>,
    pub recent_posts: Vec<PostSummary>,
}

/// Category information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub name: String,
    pub count: i64,
}

/// Tag information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub name: String,
    pub count: i64,
}

/// Home page data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomePageData {
    pub featured_posts: Vec<PostSummary>,
    pub recent_posts: Vec<PostSummary>,
    pub categories: Vec<CategoryInfo>,
    pub stats: BlogStatsResponse,
}

/// Post page data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostPageData {
    pub post: PostResponse,
    pub related_posts: Vec<PostSummary>,
    pub navigation: PostNavigation,
}

/// Navigation for posts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostNavigation {
    pub previous: Option<PostSummary>,
    pub next: Option<PostSummary>,
}

impl From<crate::models::Post> for PostResponse {
    fn from(post: crate::models::Post) -> Self {
        let url_path = post.get_url_path();
        let tags = post.get_tags();

        Self {
            id: post.id,
            slug: post.slug,
            title: post.title,
            content: post.content,
            html_content: post.html_content,
            excerpt: post.excerpt,
            category: post.category,
            tags,
            published: post.published,
            featured: post.featured,
            author: post.author,
            created_at: post.created_at,
            updated_at: post.updated_at,
            published_at: post.published_at,
            url_path,
        }
    }
}

impl From<crate::models::Post> for PostSummary {
    fn from(post: crate::models::Post) -> Self {
        let url_path = post.get_url_path();
        let tags = post.get_tags();

        Self {
            id: post.id,
            slug: post.slug,
            title: post.title,
            excerpt: post.excerpt,
            category: post.category,
            tags,
            featured: post.featured,
            author: post.author,
            created_at: post.created_at,
            published_at: post.published_at,
            url_path,
        }
    }
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>, status_code: u16) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            status_code,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message, 404)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("internal_server_error", message, 500)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message, 400)
    }
}
