use anyhow::{Context, Result};
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Markdown processing service for converting markdown to HTML and extracting frontmatter
pub struct MarkdownService;

/// Parsed markdown content with frontmatter and body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMarkdown {
    pub frontmatter: HashMap<String, serde_yaml::Value>,
    pub content: String,
    pub html: String,
}

impl MarkdownService {
    /// Create a new markdown service instance
    pub fn new() -> Self {
        Self
    }

    /// Parse markdown content with frontmatter and convert to HTML
    pub fn parse_markdown(&self, content: &str) -> Result<ParsedMarkdown> {
        debug!("Parsing markdown content");

        let (frontmatter, markdown_content) = self.extract_frontmatter(content)?;
        let html = self.markdown_to_html(&markdown_content)?;

        Ok(ParsedMarkdown {
            frontmatter,
            content: markdown_content,
            html,
        })
    }

    /// Extract YAML frontmatter from markdown content
    fn extract_frontmatter(&self, content: &str) -> Result<(HashMap<String, serde_yaml::Value>, String)> {
        let content = content.trim();
        
        if !content.starts_with("---") {
            debug!("No frontmatter found in markdown");
            return Ok((HashMap::new(), content.to_string()));
        }

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            warn!("Invalid frontmatter format");
            return Ok((HashMap::new(), content.to_string()));
        }

        let frontmatter_str = parts[1].trim();
        let markdown_content = parts[2].trim();

        let frontmatter: HashMap<String, serde_yaml::Value> = if frontmatter_str.is_empty() {
            HashMap::new()
        } else {
            serde_yaml::from_str(frontmatter_str)
                .context("Failed to parse YAML frontmatter")?
        };

        debug!("Extracted {} frontmatter fields", frontmatter.len());
        Ok((frontmatter, markdown_content.to_string()))
    }

    /// Convert markdown content to HTML
    fn markdown_to_html(&self, markdown: &str) -> Result<String> {
        debug!("Converting markdown to HTML");

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);

        let parser = Parser::new_ext(markdown, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        debug!("Generated {} bytes of HTML", html_output.len());
        Ok(html_output)
    }

    /// Extract a specific field from frontmatter with type conversion
    pub fn extract_frontmatter_field<T>(&self, frontmatter: &HashMap<String, serde_yaml::Value>, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        frontmatter.get(key).and_then(|value| {
            T::deserialize(value.clone()).ok()
        })
    }

    /// Extract title from frontmatter or generate from content
    pub fn extract_title(&self, frontmatter: &HashMap<String, serde_yaml::Value>, content: &str) -> String {
        // Try to get title from frontmatter
        if let Some(title) = self.extract_frontmatter_field::<String>(frontmatter, "title") {
            return title;
        }

        // Fallback: extract from first heading in content
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("# ") {
                return line[2..].trim().to_string();
            }
        }

        // Final fallback
        "Untitled".to_string()
    }

    /// Extract tags from frontmatter
    pub fn extract_tags(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> Vec<String> {
        self.extract_frontmatter_field::<Vec<String>>(frontmatter, "tags")
            .unwrap_or_default()
    }

    /// Extract category from frontmatter
    pub fn extract_category(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> Option<String> {
        self.extract_frontmatter_field::<String>(frontmatter, "category")
    }

    /// Extract published status from frontmatter (defaults to true)
    pub fn extract_published(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> bool {
        self.extract_frontmatter_field::<bool>(frontmatter, "published")
            .unwrap_or(true)
    }

    /// Extract author from frontmatter
    pub fn extract_author(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> Option<String> {
        self.extract_frontmatter_field::<String>(frontmatter, "author")
    }

    /// Extract excerpt from frontmatter
    pub fn extract_excerpt(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> Option<String> {
        self.extract_frontmatter_field::<String>(frontmatter, "excerpt")
    }

    /// Generate excerpt from content if not provided in frontmatter
    pub fn generate_excerpt(&self, content: &str, max_words: usize) -> String {
        let words: Vec<&str> = content
            .split_whitespace()
            .take(max_words)
            .collect();
        
        let excerpt = words.join(" ");
        if words.len() < content.split_whitespace().count() {
            format!("{}...", excerpt)
        } else {
            excerpt
        }
    }
}

impl Default for MarkdownService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_with_frontmatter() {
        let service = MarkdownService::new();
        let content = r#"---
title: "Test Post"
tags: ["rust", "blog"]
published: true
---

# Hello World

This is a test post."#;

        let result = service.parse_markdown(content).unwrap();
        
        assert_eq!(result.frontmatter.get("title").unwrap().as_str().unwrap(), "Test Post");
        assert!(result.html.contains("<h1>Hello World</h1>"));
        assert!(result.html.contains("<p>This is a test post.</p>"));
    }

    #[test]
    fn test_parse_markdown_without_frontmatter() {
        let service = MarkdownService::new();
        let content = "# Hello World\n\nThis is a test post.";

        let result = service.parse_markdown(content).unwrap();
        
        assert!(result.frontmatter.is_empty());
        assert!(result.html.contains("<h1>Hello World</h1>"));
    }

    #[test]
    fn test_extract_title() {
        let service = MarkdownService::new();
        let mut frontmatter = HashMap::new();
        frontmatter.insert("title".to_string(), serde_yaml::Value::String("Test Title".to_string()));

        let title = service.extract_title(&frontmatter, "# Fallback Title");
        assert_eq!(title, "Test Title");
    }

    #[test]
    fn test_generate_excerpt() {
        let service = MarkdownService::new();
        let content = "This is a long piece of content that should be truncated at some point.";
        
        let excerpt = service.generate_excerpt(content, 5);
        assert_eq!(excerpt, "This is a long piece...");
    }
}