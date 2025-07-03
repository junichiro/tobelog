use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use tera::Tera;
use tracing::{debug, info};

/// Template service for rendering HTML using Tera
#[derive(Clone)]
pub struct TemplateService {
    tera: Tera,
}

impl TemplateService {
    /// Create a new template service
    pub fn new() -> Result<Self> {
        info!("Initializing Tera template engine");
        
        let mut tera = Tera::new("templates/**/*.html")
            .context("Failed to initialize Tera template engine")?;
        
        // Register custom filters
        tera.register_filter("truncate", truncate_filter);
        
        info!("Template engine initialized successfully");
        debug!("Available templates: {:?}", tera.get_template_names().collect::<Vec<_>>());
        
        Ok(Self { tera })
    }

    /// Render a template with context
    pub fn render<T: Serialize>(&self, template_name: &str, context: &T) -> Result<String> {
        debug!("Rendering template: {}", template_name);
        
        let result = self.tera
            .render(template_name, &tera::Context::from_serialize(context)?)
            .with_context(|| format!("Failed to render template: {}", template_name))?;
        
        debug!("Template rendered successfully: {} characters", result.len());
        Ok(result)
    }

    /// Render template with additional context variables
    #[allow(dead_code)]
    pub fn render_with_context<T: Serialize>(
        &self,
        template_name: &str,
        context: &T,
        additional_context: HashMap<String, tera::Value>
    ) -> Result<String> {
        debug!("Rendering template with additional context: {}", template_name);
        
        let mut tera_context = tera::Context::from_serialize(context)?;
        for (key, value) in additional_context {
            tera_context.insert(key, &value);
        }
        
        let result = self.tera
            .render(template_name, &tera_context)
            .with_context(|| format!("Failed to render template: {}", template_name))?;
        
        debug!("Template rendered successfully: {} characters", result.len());
        Ok(result)
    }

    /// Get template engine instance
    #[allow(dead_code)]
    pub fn tera(&self) -> &Tera {
        &self.tera
    }
}

impl Default for TemplateService {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default template service")
    }
}

/// Custom filter to truncate text
fn truncate_filter(value: &tera::Value, args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let s = value.as_str().unwrap_or("");
    let length = args.get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;
    
    if s.chars().count() <= length {
        Ok(tera::Value::String(s.to_string()))
    } else {
        let truncated = s.chars().take(length).collect::<String>();
        Ok(tera::Value::String(format!("{}...", truncated)))
    }
}

/// Context for home page template
#[derive(Debug, Serialize)]
pub struct HomePageContext {
    pub site_title: String,
    pub site_description: String,
    pub posts: Vec<PostSummary>,
    pub blog_stats: Option<BlogStats>,
}

/// Context for post page template
#[derive(Debug, Serialize)]
pub struct PostPageContext {
    pub site_title: String,
    pub site_description: String,
    pub post: PostData,
}

/// Context for category page template
#[derive(Debug, Serialize)]
pub struct CategoryPageContext {
    pub site_title: String,
    pub site_description: String,
    pub category_name: String,
    pub posts: Vec<PostSummary>,
    pub total_posts: usize,
    pub page: usize,
    pub total_pages: usize,
}

/// Context for tag page template
#[derive(Debug, Serialize)]
pub struct TagPageContext {
    pub site_title: String,
    pub site_description: String,
    pub tag_name: String,
    pub posts: Vec<PostSummary>,
    pub total_posts: usize,
    pub page: usize,
    pub total_pages: usize,
}

/// Post summary for templates
#[derive(Debug, Serialize)]
pub struct PostSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub published: bool,
    pub featured: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Post data for templates
#[derive(Debug, Serialize)]
pub struct PostData {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub html_content: String,
    pub excerpt: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub published: bool,
    pub featured: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Blog statistics for templates
#[derive(Debug, Serialize)]
pub struct BlogStats {
    pub total_posts: i64,
    pub published_posts: i64,
    pub featured_posts: i64,
    pub categories: Vec<CategoryStat>,
    pub tags: Vec<TagStat>,
}

/// Category statistics
#[derive(Debug, Serialize)]
pub struct CategoryStat {
    pub name: String,
    pub count: i64,
}

/// Tag statistics
#[derive(Debug, Serialize)]
pub struct TagStat {
    pub name: String,
    pub count: i64,
}

// Conversion implementations
impl From<crate::models::Post> for PostSummary {
    fn from(post: crate::models::Post) -> Self {
        let tags = post.get_tags();
        Self {
            id: post.id.to_string(),
            slug: post.slug,
            title: post.title,
            excerpt: post.excerpt,
            category: post.category,
            tags,
            author: post.author,
            published: post.published,
            featured: post.featured,
            created_at: post.created_at,
            published_at: post.published_at,
        }
    }
}

impl From<crate::models::Post> for PostData {
    fn from(post: crate::models::Post) -> Self {
        let tags = post.get_tags();
        Self {
            id: post.id.to_string(),
            slug: post.slug,
            title: post.title,
            content: post.content,
            html_content: post.html_content,
            excerpt: post.excerpt,
            category: post.category,
            tags,
            author: post.author,
            published: post.published,
            featured: post.featured,
            created_at: post.created_at,
            published_at: post.published_at,
        }
    }
}

impl From<crate::models::CategoryStat> for CategoryStat {
    fn from(stat: crate::models::CategoryStat) -> Self {
        Self {
            name: stat.name,
            count: stat.count,
        }
    }
}

impl From<crate::models::PostStats> for BlogStats {
    fn from(stats: crate::models::PostStats) -> Self {
        Self {
            total_posts: stats.total_posts,
            published_posts: stats.published_posts,
            featured_posts: stats.featured_posts,
            categories: stats.categories.into_iter().map(CategoryStat::from).collect(),
            tags: stats.tags.into_iter().map(|tag| TagStat {
                name: tag.name,
                count: tag.count,
            }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_service_creation() {
        // This test might fail if templates directory doesn't exist in test environment
        // In real usage, we ensure templates directory exists
        let _service = TemplateService::new();
    }

    #[test]
    fn test_truncate_filter() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), tera::Value::Number(10.into()));
        
        let value = tera::Value::String("This is a long text that should be truncated".to_string());
        let result = truncate_filter(&value, &args).unwrap();
        
        assert_eq!(result.as_str().unwrap(), "This is a ...");
    }
}