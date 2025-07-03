use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Post version information for version history management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostVersion {
    pub id: i64,
    pub post_id: Uuid,
    pub version: i32,
    pub title: String,
    pub content: String,
    pub html_content: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub change_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

/// Post version creation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostVersion {
    pub post_id: Uuid,
    pub version: i32,
    pub title: String,
    pub content: String,
    pub html_content: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub change_summary: Option<String>,
    pub created_by: Option<String>,
}

/// Version comparison data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub post_id: Uuid,
    pub version_from: i32,
    pub version_to: i32,
    pub title_diff: Option<String>,
    pub content_diff: String,
    pub metadata_diff: Option<serde_json::Value>,
    pub created_at_from: DateTime<Utc>,
    pub created_at_to: DateTime<Utc>,
}

/// Version history summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    pub post_id: Uuid,
    pub post_slug: String,
    pub post_title: String,
    pub versions: Vec<VersionSummary>,
    pub total_versions: usize,
}

/// Individual version summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSummary {
    pub version: i32,
    pub title: String,
    pub change_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub is_current: bool,
}

/// Version restore request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreVersionRequest {
    pub target_version: i32,
    pub change_summary: Option<String>,
}

/// Version filters for querying
#[derive(Debug, Clone, Default)]
pub struct VersionFilters {
    pub post_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Change type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Content,
    Title,
    Category,
    Tags,
    Metadata,
    Multiple,
}

impl From<crate::models::Post> for CreatePostVersion {
    fn from(post: crate::models::Post) -> Self {
        let tags = post.get_tags();
        CreatePostVersion {
            post_id: post.id,
            version: post.version,
            title: post.title,
            content: post.content,
            html_content: post.html_content,
            excerpt: post.excerpt,
            category: post.category,
            tags,
            metadata: None, // TODO: Extract metadata from post if needed
            change_summary: None,
            created_by: post.author,
        }
    }
}

impl PostVersion {
    /// Get tags as a vector from JSON string
    pub fn get_tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    /// Check if this version is a major change (title, category, or significant content changes)
    pub fn is_major_change(&self, previous: Option<&PostVersion>) -> bool {
        match previous {
            Some(prev) => {
                // Title changed
                if self.title != prev.title {
                    return true;
                }

                // Category changed
                if self.category != prev.category {
                    return true;
                }

                // Content changed significantly (more than 20% difference)
                let content_diff_ratio = self.calculate_content_diff_ratio(&prev.content);
                content_diff_ratio > 0.2
            }
            None => true, // First version is always major
        }
    }

    /// Calculate content difference ratio (0.0 = no change, 1.0 = completely different)
    fn calculate_content_diff_ratio(&self, other_content: &str) -> f64 {
        let self_len = self.content.len() as f64;
        let other_len = other_content.len() as f64;

        if self_len == 0.0 && other_len == 0.0 {
            return 0.0;
        }

        if self_len == 0.0 || other_len == 0.0 {
            return 1.0;
        }

        // Simple difference calculation based on length and character differences
        let len_diff = (self_len - other_len).abs() / self_len.max(other_len);

        // Count character differences (simple approximation)
        let char_diff = self
            .content
            .chars()
            .zip(other_content.chars())
            .filter(|(a, b)| a != b)
            .count() as f64;

        let max_len = self.content.len().max(other_content.len()) as f64;
        let char_diff_ratio = char_diff / max_len;

        // Combine length and character differences
        (len_diff + char_diff_ratio) / 2.0
    }
}

/// Response types for API endpoints

#[derive(Debug, Serialize)]
pub struct VersionHistoryResponse {
    pub success: bool,
    pub data: VersionHistory,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub success: bool,
    pub data: PostVersion,
}

#[derive(Debug, Serialize)]
pub struct VersionDiffResponse {
    pub success: bool,
    pub data: VersionDiff,
}

#[derive(Debug, Serialize)]
pub struct RestoreVersionResponse {
    pub success: bool,
    pub message: String,
    pub new_version: i32,
}
