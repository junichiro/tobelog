use anyhow::{Context, Result};
use chrono::Utc;
use tracing::{debug, info};

use crate::models::{
    CreatePostVersion, Post, PostVersion, VersionDiff, VersionFilters, VersionHistory,
    VersionSummary,
};
use crate::services::{DatabaseService, MarkdownService};

/// Service for managing post version history
#[derive(Clone)]
pub struct VersionService {
    database: DatabaseService,
    #[allow(dead_code)]
    markdown: MarkdownService,
}

impl VersionService {
    /// Create a new version service
    pub fn new(database: DatabaseService, markdown: MarkdownService) -> Self {
        Self { database, markdown }
    }

    /// Create a version snapshot of a post
    pub async fn create_version(
        &self,
        post: &Post,
        change_summary: Option<String>,
    ) -> Result<PostVersion> {
        debug!("Creating version {} for post {}", post.version, post.id);

        let create_version = CreatePostVersion {
            post_id: post.id,
            version: post.version,
            title: post.title.clone(),
            content: post.content.clone(),
            html_content: post.html_content.clone(),
            excerpt: post.excerpt.clone(),
            category: post.category.clone(),
            tags: post.get_tags(),
            metadata: None, // Could extract metadata here if needed
            change_summary,
            created_by: post.author.clone(),
        };

        self.database.create_post_version(&create_version).await
    }

    /// Get version history for a post
    pub async fn get_version_history(&self, post_id: uuid::Uuid) -> Result<VersionHistory> {
        debug!("Getting version history for post {}", post_id);

        // Get the current post info
        let post = self
            .database
            .get_post_by_id(post_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

        // Get all versions for this post
        let filters = VersionFilters {
            post_id: Some(post_id),
            ..Default::default()
        };

        let versions = self.database.list_post_versions(filters).await?;

        // Convert to version summaries
        let version_summaries: Vec<VersionSummary> = versions
            .into_iter()
            .map(|v| VersionSummary {
                version: v.version,
                title: v.title,
                change_summary: v.change_summary,
                created_at: v.created_at,
                created_by: v.created_by,
                is_current: v.version == post.version,
            })
            .collect();

        let total_versions = version_summaries.len();

        Ok(VersionHistory {
            post_id,
            post_slug: post.slug,
            post_title: post.title,
            versions: version_summaries,
            total_versions,
        })
    }

    /// Get a specific version of a post
    pub async fn get_version(
        &self,
        post_id: uuid::Uuid,
        version: i32,
    ) -> Result<Option<PostVersion>> {
        debug!("Getting version {} for post {}", version, post_id);

        self.database.get_post_version(post_id, version).await
    }

    /// Compare two versions of a post
    pub async fn compare_versions(
        &self,
        post_id: uuid::Uuid,
        version_from: i32,
        version_to: i32,
    ) -> Result<VersionDiff> {
        debug!(
            "Comparing versions {} and {} for post {}",
            version_from, version_to, post_id
        );

        let version_from_data = self
            .database
            .get_post_version(post_id, version_from)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Version {} not found", version_from))?;

        let version_to_data = self
            .database
            .get_post_version(post_id, version_to)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Version {} not found", version_to))?;

        // Generate diffs
        let title_diff = if version_from_data.title != version_to_data.title {
            Some(self.generate_text_diff(&version_from_data.title, &version_to_data.title))
        } else {
            None
        };

        let content_diff =
            self.generate_text_diff(&version_from_data.content, &version_to_data.content);

        // Generate metadata diff (simplified)
        let metadata_diff = if version_from_data.metadata != version_to_data.metadata {
            Some(serde_json::json!({
                "from": version_from_data.metadata,
                "to": version_to_data.metadata
            }))
        } else {
            None
        };

        Ok(VersionDiff {
            post_id,
            version_from,
            version_to,
            title_diff,
            content_diff,
            metadata_diff,
            created_at_from: version_from_data.created_at,
            created_at_to: version_to_data.created_at,
        })
    }

    /// Restore a post to a previous version
    ///
    /// Note: This operation involves multiple database writes and should ideally be wrapped
    /// in a database transaction to ensure atomicity. Currently, we rely on manual error
    /// handling and cleanup, but this could be improved with proper transaction support.
    pub async fn restore_version(
        &self,
        post_id: uuid::Uuid,
        target_version: i32,
        change_summary: Option<String>,
    ) -> Result<Post> {
        debug!("Restoring post {} to version {}", post_id, target_version);

        // Get the target version first to fail early if it doesn't exist
        let target_version_data = self
            .database
            .get_post_version(post_id, target_version)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target version {} not found", target_version))?;

        // Get the current post and validate it exists
        let mut current_post = self
            .database
            .get_post_by_id(post_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

        // Create a version snapshot of current state before restoring
        let current_summary = format!("Auto-backup before restore to version {}", target_version);
        self.create_version(&current_post, Some(current_summary))
            .await
            .context("Failed to create backup version before restore")?;

        // Update the post with target version data
        current_post.title = target_version_data.title;
        current_post.content = target_version_data.content;
        current_post.html_content = target_version_data.html_content;
        current_post.excerpt = target_version_data.excerpt;
        current_post.category = target_version_data.category;
        current_post.set_tags(target_version_data.tags);
        current_post.version += 1; // Increment version for the restore
        current_post.updated_at = Utc::now();

        // Save the restored post
        let update_data = crate::models::UpdatePost {
            title: Some(current_post.title.clone()),
            content: Some(current_post.content.clone()),
            html_content: Some(current_post.html_content.clone()),
            excerpt: current_post.excerpt.clone(),
            category: current_post.category.clone(),
            tags: Some(current_post.get_tags()),
            published: Some(current_post.published),
            featured: Some(current_post.featured),
            author: current_post.author.clone(),
            dropbox_path: Some(current_post.dropbox_path.clone()),
        };

        let updated_post = self
            .database
            .update_post(post_id, update_data)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to update post during restore"))?;

        // Create a version for the restore
        let restore_summary =
            change_summary.unwrap_or_else(|| format!("Restored to version {}", target_version));
        self.create_version(&updated_post, Some(restore_summary))
            .await?;

        info!(
            "Successfully restored post {} to version {}",
            post_id, target_version
        );
        Ok(updated_post)
    }

    /// Auto-create version when post is updated
    #[allow(dead_code)]
    pub async fn auto_version_on_update(&self, old_post: &Post, new_post: &Post) -> Result<()> {
        debug!(
            "Auto-versioning post {} from version {} to {}",
            old_post.id, old_post.version, new_post.version
        );

        // Generate automatic change summary
        let change_summary = self.generate_change_summary(old_post, new_post);

        // Create version for the old state
        self.create_version(old_post, Some(change_summary)).await?;

        Ok(())
    }

    /// Generate a text diff using a basic but more accurate algorithm
    ///
    /// Note: This is a simplified implementation. For production use, consider using
    /// a dedicated diff library like `similar` or `dissimilar` for better performance
    /// and more sophisticated algorithms like Myers' diff algorithm.
    fn generate_text_diff(&self, from: &str, to: &str) -> String {
        if from == to {
            return "No changes".to_string();
        }

        let from_lines: Vec<&str> = from.lines().collect();
        let to_lines: Vec<&str> = to.lines().collect();

        // Use a simple diff algorithm that tracks common subsequences
        let mut diff = Vec::new();
        let mut i = 0; // index for from_lines
        let mut j = 0; // index for to_lines

        while i < from_lines.len() || j < to_lines.len() {
            match (from_lines.get(i), to_lines.get(j)) {
                (Some(from_line), Some(to_line)) => {
                    if from_line == to_line {
                        // Lines are identical - show context
                        diff.push(format!("  {}", from_line));
                        i += 1;
                        j += 1;
                    } else {
                        // Lines differ - look ahead to see if we can find a match
                        let mut found_match = false;

                        // Look for the to_line in the next few from_lines (deletion case)
                        for look_ahead in (i + 1)..=(i + 3).min(from_lines.len()) {
                            if from_lines.get(look_ahead) == Some(to_line) {
                                // Found the to_line later in from_lines, so lines were deleted
                                for del_idx in i..look_ahead {
                                    if let Some(del_line) = from_lines.get(del_idx) {
                                        diff.push(format!("- {}", del_line));
                                    }
                                }
                                diff.push(format!("  {}", to_line));
                                i = look_ahead + 1;
                                j += 1;
                                found_match = true;
                                break;
                            }
                        }

                        if !found_match {
                            // Look for the from_line in the next few to_lines (insertion case)
                            for look_ahead in (j + 1)..=(j + 3).min(to_lines.len()) {
                                if to_lines.get(look_ahead) == Some(from_line) {
                                    // Found the from_line later in to_lines, so lines were inserted
                                    for ins_idx in j..look_ahead {
                                        if let Some(ins_line) = to_lines.get(ins_idx) {
                                            diff.push(format!("+ {}", ins_line));
                                        }
                                    }
                                    diff.push(format!("  {}", from_line));
                                    i += 1;
                                    j = look_ahead + 1;
                                    found_match = true;
                                    break;
                                }
                            }
                        }

                        if !found_match {
                            // No match found, treat as substitution
                            diff.push(format!("- {}", from_line));
                            diff.push(format!("+ {}", to_line));
                            i += 1;
                            j += 1;
                        }
                    }
                }
                (Some(from_line), None) => {
                    // Remaining lines in from (deletions)
                    diff.push(format!("- {}", from_line));
                    i += 1;
                }
                (None, Some(to_line)) => {
                    // Remaining lines in to (insertions)
                    diff.push(format!("+ {}", to_line));
                    j += 1;
                }
                (None, None) => break,
            }
        }

        if diff.is_empty() {
            "No changes".to_string()
        } else {
            // Limit diff output to prevent overwhelming responses
            let max_lines = 100;
            if diff.len() > max_lines {
                diff.truncate(max_lines - 1);
                diff.push("... (diff truncated for brevity)".to_string());
            }
            diff.join("\n")
        }
    }

    /// Generate automatic change summary
    #[allow(dead_code)]
    fn generate_change_summary(&self, old_post: &Post, new_post: &Post) -> String {
        let mut changes = Vec::new();

        if old_post.title != new_post.title {
            changes.push("title");
        }

        if old_post.category != new_post.category {
            changes.push("category");
        }

        if old_post.get_tags() != new_post.get_tags() {
            changes.push("tags");
        }

        if old_post.content != new_post.content {
            changes.push("content");
        }

        if old_post.published != new_post.published {
            if new_post.published {
                changes.push("published");
            } else {
                changes.push("unpublished");
            }
        }

        if old_post.featured != new_post.featured {
            if new_post.featured {
                changes.push("featured");
            } else {
                changes.push("unfeatured");
            }
        }

        if changes.is_empty() {
            "Minor updates".to_string()
        } else {
            format!("Updated: {}", changes.join(", "))
        }
    }

    /// Clean up old versions (keep last N versions)
    pub async fn cleanup_old_versions(
        &self,
        post_id: uuid::Uuid,
        keep_versions: i32,
    ) -> Result<usize> {
        debug!(
            "Cleaning up old versions for post {}, keeping {} versions",
            post_id, keep_versions
        );

        let deleted_count = self
            .database
            .cleanup_old_versions(post_id, keep_versions)
            .await?;

        if deleted_count > 0 {
            info!(
                "Cleaned up {} old versions for post {}",
                deleted_count, post_id
            );
        }

        Ok(deleted_count)
    }
}
