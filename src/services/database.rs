use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use sqlx::sqlite::SqliteRow;
use tracing::{debug, info};
use uuid::Uuid;

use crate::models::{Post, CreatePost, UpdatePost, PostFilters, PostStats, CategoryStat};

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
            query.push_str(" LIMIT ?");
            params.push(limit.to_string());
        }

        if let Some(offset) = filters.offset {
            query.push_str(" OFFSET ?");
            params.push(offset.to_string());
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
}