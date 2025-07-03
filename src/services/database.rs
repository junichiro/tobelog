use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use sqlx::sqlite::SqliteRow;
use tracing::{debug, info};
use uuid::Uuid;

use crate::models::{Post, CreatePost, UpdatePost, PostFilters, PostStats, CategoryStat, MediaFile, MediaFilters};

#[derive(sqlx::FromRow)]
struct MediaFileRow {
    id: String,
    filename: String,
    original_filename: String,
    dropbox_path: String,
    url: String,
    file_size: i64,
    mime_type: String,
    width: Option<i64>,
    height: Option<i64>,
    uploaded_at: String,
    thumbnail_url: Option<String>,
    alt_text: Option<String>,
    caption: Option<String>,
}

/// Database service for managing SQLite operations
#[derive(Clone)]
pub struct DatabaseService {
    pool: Pool<Sqlite>,
}

impl DatabaseService {
    /// Create a new database service with connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to database: {}", database_url);
        
        let pool = SqlitePool::connect(database_url)
            .await
            .context("Failed to connect to database")?;

        let service = Self { pool };
        service.run_migrations().await?;
        
        Ok(service)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        // Migration 1: Create posts table
        let migration_1 = include_str!("../../migrations/001_create_posts_table.sql");
        sqlx::query(migration_1)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 001")?;

        // Migration 2: Create categories and tags tables
        let migration_2 = include_str!("../../migrations/002_create_categories_table.sql");
        sqlx::query(migration_2)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 002")?;

        // Migration 3: Create media files table
        let migration_3 = include_str!("../../migrations/003_create_media_table.sql");
        sqlx::query(migration_3)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 003")?;

        // Migration 4: Create post versions table
        let migration_4 = include_str!("../../migrations/004_create_post_versions_table.sql");
        sqlx::query(migration_4)
            .execute(&self.pool)
            .await
            .context("Failed to run migration 004")?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Create a new post
    #[allow(dead_code)]
    pub async fn create_post(&self, data: CreatePost) -> Result<Post> {
        debug!("Creating new post: {}", data.slug);

        let post = Post::new(data);
        
        sqlx::query(
            r#"
            INSERT INTO posts (
                id, slug, title, content, html_content, excerpt, category, tags,
                published, featured, author, dropbox_path, version, created_at, updated_at, published_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(post.id.to_string())
        .bind(&post.slug)
        .bind(&post.title)
        .bind(&post.content)
        .bind(&post.html_content)
        .bind(&post.excerpt)
        .bind(&post.category)
        .bind(&post.tags)
        .bind(if post.published { 1 } else { 0 })
        .bind(if post.featured { 1 } else { 0 })
        .bind(&post.author)
        .bind(&post.dropbox_path)
        .bind(post.version)
        .bind(post.created_at.to_rfc3339())
        .bind(post.updated_at.to_rfc3339())
        .bind(post.published_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await
        .context("Failed to create post")?;

        debug!("Created post with ID: {}", post.id);
        Ok(post)
    }

    /// Get post by slug
    pub async fn get_post_by_slug(&self, slug: &str) -> Result<Option<Post>> {
        debug!("Getting post by slug: {}", slug);

        let row = sqlx::query(
            "SELECT * FROM posts WHERE slug = ? LIMIT 1"
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get post by slug")?;

        if let Some(row) = row {
            let post = self.row_to_post(&row)?;
            Ok(Some(post))
        } else {
            Ok(None)
        }
    }

    /// Get post by ID
    #[allow(dead_code)]
    pub async fn get_post_by_id(&self, id: Uuid) -> Result<Option<Post>> {
        debug!("Getting post by ID: {}", id);

        let row = sqlx::query(
            "SELECT * FROM posts WHERE id = ? LIMIT 1"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get post by ID")?;

        if let Some(row) = row {
            let post = self.row_to_post(&row)?;
            Ok(Some(post))
        } else {
            Ok(None)
        }
    }

    /// Update post
    #[allow(dead_code)]
    pub async fn update_post(&self, id: Uuid, data: UpdatePost) -> Result<Option<Post>> {
        debug!("Updating post: {}", id);

        let mut post = match self.get_post_by_id(id).await? {
            Some(post) => post,
            None => return Ok(None),
        };

        post.update(data);

        sqlx::query(
            r#"
            UPDATE posts SET
                title = ?, content = ?, html_content = ?, excerpt = ?, category = ?, tags = ?,
                published = ?, featured = ?, author = ?, dropbox_path = ?, version = ?,
                updated_at = ?, published_at = ?
            WHERE id = ?
            "#
        )
        .bind(&post.title)
        .bind(&post.content)
        .bind(&post.html_content)
        .bind(&post.excerpt)
        .bind(&post.category)
        .bind(&post.tags)
        .bind(if post.published { 1 } else { 0 })
        .bind(if post.featured { 1 } else { 0 })
        .bind(&post.author)
        .bind(&post.dropbox_path)
        .bind(post.version)
        .bind(post.updated_at.to_rfc3339())
        .bind(post.published_at.map(|dt| dt.to_rfc3339()))
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update post")?;

        debug!("Updated post: {}", id);
        Ok(Some(post))
    }

    /// Delete post
    #[allow(dead_code)]
    pub async fn delete_post(&self, id: Uuid) -> Result<bool> {
        debug!("Deleting post: {}", id);

        let result = sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete post")?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            debug!("Deleted post: {}", id);
        }
        Ok(deleted)
    }

    /// List posts with filters
    pub async fn list_posts(&self, filters: PostFilters) -> Result<Vec<Post>> {
        debug!("Listing posts with filters: {:?}", filters);

        let mut query = "SELECT * FROM posts WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(published) = filters.published {
            query.push_str(" AND published = ?");
            params.push(if published { "1" } else { "0" }.to_string());
        }

        if let Some(category) = &filters.category {
            query.push_str(" AND category = ?");
            params.push(category.clone());
        }

        if let Some(tag) = &filters.tag {
            query.push_str(" AND tags LIKE ?");
            params.push(format!("%\"{}\"%", tag));
        }

        if let Some(author) = &filters.author {
            query.push_str(" AND author = ?");
            params.push(author.clone());
        }

        if let Some(featured) = filters.featured {
            query.push_str(" AND featured = ?");
            params.push(if featured { "1" } else { "0" }.to_string());
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to list posts")?;

        let posts = rows
            .iter()
            .map(|row| self.row_to_post(row))
            .collect::<Result<Vec<_>>>()?;

        debug!("Found {} posts", posts.len());
        Ok(posts)
    }

    /// Search posts using full-text search
    pub async fn search_posts(&self, query: &str, limit: Option<i64>) -> Result<Vec<Post>> {
        debug!("Searching posts with query: {}", query);

        let sql = if limit.is_some() {
            r#"
            SELECT p.* FROM posts p
            JOIN posts_fts fts ON p.rowid = fts.rowid
            WHERE posts_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#
        } else {
            r#"
            SELECT p.* FROM posts p
            JOIN posts_fts fts ON p.rowid = fts.rowid
            WHERE posts_fts MATCH ?
            ORDER BY rank
            "#
        };

        let mut sql_query = sqlx::query(sql).bind(query);

        if let Some(limit) = limit {
            sql_query = sql_query.bind(limit);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to search posts")?;

        let posts = rows
            .iter()
            .map(|row| self.row_to_post(row))
            .collect::<Result<Vec<_>>>()?;

        debug!("Found {} posts matching search", posts.len());
        Ok(posts)
    }

    /// Get post statistics
    pub async fn get_post_stats(&self) -> Result<PostStats> {
        debug!("Getting post statistics");

        let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get total posts count")?;

        let published_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE published = true")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get published posts count")?;

        let draft_posts = total_posts - published_posts;

        let featured_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE featured = true")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get featured posts count")?;

        // Get category statistics
        let category_rows = sqlx::query(
            r#"
            SELECT category, COUNT(*) as count
            FROM posts
            WHERE category IS NOT NULL AND published = true
            GROUP BY category
            ORDER BY count DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get category stats")?;

        let categories = category_rows
            .iter()
            .map(|row| CategoryStat {
                name: row.get("category"),
                count: row.get("count"),
            })
            .collect();

        // Get tag statistics (this is simplified - in a real implementation you'd parse the JSON)
        let tags = Vec::new(); // TODO: Implement tag parsing from JSON

        Ok(PostStats {
            total_posts,
            published_posts,
            draft_posts,
            featured_posts,
            categories,
            tags,
        })
    }

    /// Convert database row to Post struct
    fn row_to_post(&self, row: &SqliteRow) -> Result<Post> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).context("Invalid UUID format")?;

        let created_at_str: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .context("Invalid created_at format")?
            .with_timezone(&Utc);

        let updated_at_str: String = row.try_get("updated_at")?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .context("Invalid updated_at format")?
            .with_timezone(&Utc);

        let published_at = row
            .try_get::<Option<String>, _>("published_at")?
            .and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

        Ok(Post {
            id,
            slug: row.try_get("slug")?,
            title: row.try_get("title")?,
            content: row.try_get("content")?,
            html_content: row.try_get("html_content")?,
            excerpt: row.try_get("excerpt")?,
            category: row.try_get("category")?,
            tags: row.try_get("tags")?,
            published: row.try_get::<i32, _>("published")? != 0,
            featured: row.try_get::<i32, _>("featured")? != 0,
            author: row.try_get("author")?,
            dropbox_path: row.try_get("dropbox_path")?,
            version: row.try_get("version")?,
            created_at,
            updated_at,
            published_at,
        })
    }

    /// Count posts with filters for efficient pagination
    pub async fn count_posts(&self, filters: PostFilters) -> Result<i64> {
        debug!("Counting posts with filters: {:?}", filters);

        let mut query = "SELECT COUNT(*) FROM posts WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(published) = filters.published {
            query.push_str(" AND published = ?");
            params.push(if published { "1" } else { "0" }.to_string());
        }

        if let Some(category) = &filters.category {
            query.push_str(" AND category = ?");
            params.push(category.clone());
        }

        if let Some(tag) = &filters.tag {
            query.push_str(" AND tags LIKE ?");
            params.push(format!("%\"{}\"%", tag));
        }

        if let Some(author) = &filters.author {
            query.push_str(" AND author = ?");
            params.push(author.clone());
        }

        if let Some(featured) = filters.featured {
            query.push_str(" AND featured = ?");
            params.push(if featured { "1" } else { "0" }.to_string());
        }

        let mut sql_query = sqlx::query_scalar::<_, i64>(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let count = sql_query
            .fetch_one(&self.pool)
            .await
            .context("Failed to count posts")?;

        debug!("Found {} posts matching filters", count);
        Ok(count)
    }

    /// Get database pool reference
    #[allow(dead_code)]
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    // Media file management methods

    /// Create a new media file record
    pub async fn create_media_file(&self, media: &MediaFile) -> Result<()> {
        debug!("Creating media file: {}", media.filename);

        sqlx::query(
            r#"
            INSERT INTO media_files (
                id, filename, original_filename, dropbox_path, url, file_size,
                mime_type, width, height, uploaded_at, thumbnail_url, alt_text, caption
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(media.id.to_string())
        .bind(&media.filename)
        .bind(&media.original_filename)
        .bind(&media.dropbox_path)
        .bind(&media.url)
        .bind(media.file_size as i64)
        .bind(&media.mime_type)
        .bind(media.width.map(|w| w as i64))
        .bind(media.height.map(|h| h as i64))
        .bind(media.uploaded_at.to_rfc3339())
        .bind(&media.thumbnail_url)
        .bind(&media.alt_text)
        .bind(&media.caption)
        .execute(&self.pool)
        .await
        .context("Failed to insert media file")?;

        info!("Created media file: {}", media.filename);
        Ok(())
    }

    /// List media files with filters and pagination
    pub async fn list_media_files(&self, filters: MediaFilters) -> Result<Vec<MediaFile>> {
        debug!("Listing media files with filters: {:?}", filters);

        let mut query = "SELECT * FROM media_files WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(folder) = &filters.folder {
            query.push_str(" AND dropbox_path LIKE ?");
            params.push(format!("%/{}/%", folder));
        }

        if let Some(mime_type) = &filters.mime_type {
            query.push_str(" AND mime_type LIKE ?");
            params.push(format!("{}%", mime_type));
        }

        if let Some(search) = &filters.search {
            query.push_str(" AND (filename LIKE ? OR original_filename LIKE ? OR alt_text LIKE ? OR caption LIKE ?)");
            let search_param = format!("%{}%", search);
            params.push(search_param.clone());
            params.push(search_param.clone());
            params.push(search_param.clone());
            params.push(search_param);
        }

        query.push_str(" ORDER BY uploaded_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch media files")?;

        let media_files = rows
            .into_iter()
            .map(|row| self.row_to_media_file(row))
            .collect::<Result<Vec<_>>>()?;

        debug!("Found {} media files", media_files.len());
        Ok(media_files)
    }

    /// Count media files with filters
    pub async fn count_media_files(&self, filters: MediaFilters) -> Result<usize> {
        debug!("Counting media files with filters: {:?}", filters);

        let mut query = "SELECT COUNT(*) FROM media_files WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(folder) = &filters.folder {
            query.push_str(" AND dropbox_path LIKE ?");
            params.push(format!("%/{}/%", folder));
        }

        if let Some(mime_type) = &filters.mime_type {
            query.push_str(" AND mime_type LIKE ?");
            params.push(format!("{}%", mime_type));
        }

        if let Some(search) = &filters.search {
            query.push_str(" AND (filename LIKE ? OR original_filename LIKE ? OR alt_text LIKE ? OR caption LIKE ?)");
            let search_param = format!("%{}%", search);
            params.push(search_param.clone());
            params.push(search_param.clone());
            params.push(search_param.clone());
            params.push(search_param);
        }

        let mut sql_query = sqlx::query_scalar::<_, i64>(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let count = sql_query
            .fetch_one(&self.pool)
            .await
            .context("Failed to count media files")?;

        debug!("Found {} media files matching filters", count);
        Ok(count as usize)
    }

    /// Get media file by ID
    pub async fn get_media_file(&self, id: Uuid) -> Result<Option<MediaFile>> {
        debug!("Getting media file by ID: {}", id);

        let row = sqlx::query_as::<_, MediaFileRow>(
            "SELECT * FROM media_files WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch media file")?;

        match row {
            Some(row) => {
                let media_file = MediaFile {
                    id: Uuid::parse_str(&row.id).context("Invalid UUID in database")?,
                    filename: row.filename,
                    original_filename: row.original_filename,
                    dropbox_path: row.dropbox_path,
                    url: row.url,
                    file_size: row.file_size as u64,
                    mime_type: row.mime_type,
                    width: row.width.map(|w| w as u32),
                    height: row.height.map(|h| h as u32),
                    uploaded_at: DateTime::parse_from_rfc3339(&row.uploaded_at)
                        .context("Invalid uploaded_at timestamp")?
                        .with_timezone(&Utc),
                    thumbnail_url: row.thumbnail_url,
                    alt_text: row.alt_text,
                    caption: row.caption,
                };
                Ok(Some(media_file))
            }
            None => Ok(None),
        }
    }

    /// Delete media file by ID
    pub async fn delete_media_file(&self, id: Uuid) -> Result<bool> {
        debug!("Deleting media file by ID: {}", id);

        let result = sqlx::query(
            "DELETE FROM media_files WHERE id = ?"
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete media file")?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!("Deleted media file: {}", id);
        } else {
            debug!("Media file not found for deletion: {}", id);
        }

        Ok(deleted)
    }

    /// Associate media file with a post
    pub async fn associate_media_with_post(&self, post_id: Uuid, media_id: Uuid) -> Result<()> {
        debug!("Associating media {} with post {}", media_id, post_id);

        sqlx::query(
            "INSERT OR IGNORE INTO posts_media (post_id, media_id) VALUES (?, ?)"
        )
        .bind(post_id.to_string())
        .bind(media_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to associate media with post")?;

        Ok(())
    }

    /// Get media files associated with a post
    pub async fn get_post_media(&self, post_id: Uuid) -> Result<Vec<MediaFile>> {
        debug!("Getting media files for post: {}", post_id);

        let rows = sqlx::query_as::<_, MediaFileRow>(
            r#"
            SELECT m.* FROM media_files m
            JOIN posts_media pm ON m.id = pm.media_id
            WHERE pm.post_id = ?
            ORDER BY m.uploaded_at DESC
            "#,
        )
        .bind(post_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch post media")?;

        let media_files = rows
            .into_iter()
            .map(|row| -> Result<MediaFile> {
                Ok(MediaFile {
                    id: Uuid::parse_str(&row.id).context("Invalid UUID in database")?,
                    filename: row.filename,
                    original_filename: row.original_filename,
                    dropbox_path: row.dropbox_path,
                    url: row.url,
                    file_size: row.file_size as u64,
                    mime_type: row.mime_type,
                    width: row.width.map(|w| w as u32),
                    height: row.height.map(|h| h as u32),
                    uploaded_at: DateTime::parse_from_rfc3339(&row.uploaded_at)
                        .context("Invalid uploaded_at timestamp")?
                        .with_timezone(&Utc),
                    thumbnail_url: row.thumbnail_url,
                    alt_text: row.alt_text,
                    caption: row.caption,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        debug!("Found {} media files for post", media_files.len());
        Ok(media_files)
    }

    /// Helper method to convert SqliteRow to MediaFile
    fn row_to_media_file(&self, row: SqliteRow) -> Result<MediaFile> {
        Ok(MediaFile {
            id: Uuid::parse_str(row.try_get("id")?).context("Invalid UUID in database")?,
            filename: row.try_get("filename")?,
            original_filename: row.try_get("original_filename")?,
            dropbox_path: row.try_get("dropbox_path")?,
            url: row.try_get("url")?,
            file_size: row.try_get::<i64, _>("file_size")? as u64,
            mime_type: row.try_get("mime_type")?,
            width: row.try_get::<Option<i64>, _>("width")?.map(|w| w as u32),
            height: row.try_get::<Option<i64>, _>("height")?.map(|h| h as u32),
            uploaded_at: DateTime::parse_from_rfc3339(row.try_get("uploaded_at")?)
                .context("Invalid uploaded_at timestamp")?
                .with_timezone(&Utc),
            thumbnail_url: row.try_get("thumbnail_url")?,
            alt_text: row.try_get("alt_text")?,
            caption: row.try_get("caption")?,
        })
    }

    // Version management methods

    /// Create a new post version record
    pub async fn create_post_version(&self, version: &crate::models::CreatePostVersion) -> Result<crate::models::PostVersion> {
        debug!("Creating post version {} for post {}", version.version, version.post_id);

        let now = Utc::now();
        let version_id = sqlx::query(
            r#"
            INSERT INTO post_versions (
                post_id, version, title, content, html_content, excerpt, category, tags,
                metadata, change_summary, created_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(version.post_id.to_string())
        .bind(version.version)
        .bind(&version.title)
        .bind(&version.content)
        .bind(&version.html_content)
        .bind(&version.excerpt)
        .bind(&version.category)
        .bind(serde_json::to_string(&version.tags).unwrap_or_else(|_| "[]".to_string()))
        .bind(version.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string())))
        .bind(&version.change_summary)
        .bind(now.to_rfc3339())
        .bind(&version.created_by)
        .execute(&self.pool)
        .await
        .context("Failed to insert post version")?;

        let id = version_id.last_insert_rowid();

        Ok(crate::models::PostVersion {
            id,
            post_id: version.post_id,
            version: version.version,
            title: version.title.clone(),
            content: version.content.clone(),
            html_content: version.html_content.clone(),
            excerpt: version.excerpt.clone(),
            category: version.category.clone(),
            tags: version.tags.clone(),
            metadata: version.metadata.clone(),
            change_summary: version.change_summary.clone(),
            created_at: now,
            created_by: version.created_by.clone(),
        })
    }

    /// Get a specific version of a post
    pub async fn get_post_version(&self, post_id: uuid::Uuid, version: i32) -> Result<Option<crate::models::PostVersion>> {
        debug!("Getting version {} for post {}", version, post_id);

        let row = sqlx::query(
            "SELECT * FROM post_versions WHERE post_id = ? AND version = ? LIMIT 1"
        )
        .bind(post_id.to_string())
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get post version")?;

        if let Some(row) = row {
            let version = self.row_to_post_version(&row)?;
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    /// List post versions with filters
    pub async fn list_post_versions(&self, filters: crate::models::VersionFilters) -> Result<Vec<crate::models::PostVersion>> {
        debug!("Listing post versions with filters: {:?}", filters);

        let mut query = "SELECT * FROM post_versions WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(post_id) = filters.post_id {
            query.push_str(" AND post_id = ?");
            params.push(post_id.to_string());
        }

        query.push_str(" ORDER BY version DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch post versions")?;

        let versions = rows
            .iter()
            .map(|row| self.row_to_post_version(row))
            .collect::<Result<Vec<_>>>()?;

        debug!("Found {} versions", versions.len());
        Ok(versions)
    }

    /// Delete old versions, keeping only the most recent N versions
    pub async fn cleanup_old_versions(&self, post_id: uuid::Uuid, keep_versions: i32) -> Result<usize> {
        debug!("Cleaning up old versions for post {}, keeping {} versions", post_id, keep_versions);

        let result = sqlx::query(
            r#"
            DELETE FROM post_versions 
            WHERE post_id = ? 
            AND version NOT IN (
                SELECT version FROM post_versions 
                WHERE post_id = ? 
                ORDER BY version DESC 
                LIMIT ?
            )
            "#
        )
        .bind(post_id.to_string())
        .bind(post_id.to_string())
        .bind(keep_versions)
        .execute(&self.pool)
        .await
        .context("Failed to cleanup old versions")?;

        let deleted_count = result.rows_affected() as usize;
        debug!("Deleted {} old versions", deleted_count);
        Ok(deleted_count)
    }

    /// Helper method to convert SqliteRow to PostVersion
    fn row_to_post_version(&self, row: &sqlx::sqlite::SqliteRow) -> Result<crate::models::PostVersion> {
        let tags_json: String = row.try_get("tags")?;
        let tags: Vec<String> = serde_json::from_str(&tags_json)
            .unwrap_or_else(|_| Vec::new());

        let metadata_json: Option<String> = row.try_get("metadata")?;
        let metadata = metadata_json
            .and_then(|json| serde_json::from_str(&json).ok());

        Ok(crate::models::PostVersion {
            id: row.try_get("id")?,
            post_id: uuid::Uuid::parse_str(row.try_get("post_id")?).context("Invalid UUID in database")?,
            version: row.try_get("version")?,
            title: row.try_get("title")?,
            content: row.try_get("content")?,
            html_content: row.try_get("html_content")?,
            excerpt: row.try_get("excerpt")?,
            category: row.try_get("category")?,
            tags,
            metadata,
            change_summary: row.try_get("change_summary")?,
            created_at: DateTime::parse_from_rfc3339(row.try_get("created_at")?)
                .context("Invalid created_at timestamp")?
                .with_timezone(&Utc),
            created_by: row.try_get("created_by")?,
        })
    }
}