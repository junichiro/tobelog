use anyhow::Result;
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Markdown processing service for converting markdown to HTML and extracting frontmatter
#[derive(Clone)]
pub struct MarkdownService;

/// Supported frontmatter formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrontmatterFormat {
    Yaml,
    Toml,
    Json,
    None,
}

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
    #[allow(dead_code)]
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

    /// Detect frontmatter format
    fn detect_frontmatter_format(&self, content: &str) -> FrontmatterFormat {
        let trimmed = content.trim_start();
        
        if trimmed.starts_with("---") {
            FrontmatterFormat::Yaml
        } else if trimmed.starts_with("+++") {
            FrontmatterFormat::Toml
        } else if trimmed.starts_with('{') {
            // Simple check for JSON - might need more robust detection
            FrontmatterFormat::Json
        } else {
            FrontmatterFormat::None
        }
    }

    /// Extract frontmatter from markdown content (supports YAML, TOML, JSON)
    #[allow(dead_code)]
    fn extract_frontmatter(
        &self,
        content: &str,
    ) -> Result<(HashMap<String, serde_yaml::Value>, String)> {
        let format = self.detect_frontmatter_format(content);
        
        match format {
            FrontmatterFormat::Yaml => self.extract_yaml_frontmatter(content),
            FrontmatterFormat::Toml => self.extract_toml_frontmatter(content),
            FrontmatterFormat::Json => self.extract_json_frontmatter(content),
            FrontmatterFormat::None => {
                debug!("No frontmatter found in markdown");
                Ok((HashMap::new(), content.to_string()))
            }
        }
    }

    /// Extract YAML frontmatter
    fn extract_yaml_frontmatter(
        &self,
        content: &str,
    ) -> Result<(HashMap<String, serde_yaml::Value>, String)> {
        let content = content.trim();

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            warn!("Invalid YAML frontmatter format");
            return Ok((HashMap::new(), content.to_string()));
        }

        let frontmatter_str = parts[1].trim();
        let markdown_content = parts[2].trim();

        let frontmatter: HashMap<String, serde_yaml::Value> = if frontmatter_str.is_empty() {
            HashMap::new()
        } else {
            match serde_yaml::from_str(frontmatter_str) {
                Ok(fm) => fm,
                Err(e) => {
                    warn!("Failed to parse YAML frontmatter: {}", e);
                    return Ok((HashMap::new(), content.to_string()));
                }
            }
        };

        debug!("Extracted {} YAML frontmatter fields", frontmatter.len());
        Ok((frontmatter, markdown_content.to_string()))
    }

    /// Extract TOML frontmatter
    fn extract_toml_frontmatter(
        &self,
        content: &str,
    ) -> Result<(HashMap<String, serde_yaml::Value>, String)> {
        let content = content.trim();

        let parts: Vec<&str> = content.splitn(3, "+++").collect();
        if parts.len() < 3 {
            warn!("Invalid TOML frontmatter format");
            return Ok((HashMap::new(), content.to_string()));
        }

        let frontmatter_str = parts[1].trim();
        let markdown_content = parts[2].trim();

        // Parse TOML and convert to serde_yaml::Value
        let toml_value: toml::Value = match toml::from_str(frontmatter_str) {
            Ok(val) => val,
            Err(e) => {
                warn!("Failed to parse TOML frontmatter: {}", e);
                return Ok((HashMap::new(), content.to_string()));
            }
        };

        // Convert TOML to YAML value (via JSON as intermediate)
        let json_value = serde_json::to_value(toml_value)?;
        let yaml_value: serde_yaml::Value = serde_json::from_value(json_value)?;
        
        let frontmatter = self.yaml_value_to_hashmap(yaml_value);

        debug!("Extracted {} TOML frontmatter fields", frontmatter.len());
        Ok((frontmatter, markdown_content.to_string()))
    }

    /// Extract JSON frontmatter
    fn extract_json_frontmatter(
        &self,
        content: &str,
    ) -> Result<(HashMap<String, serde_yaml::Value>, String)> {
        let content = content.trim_start();
        
        // Try to find the end of JSON by looking for balanced braces
        // This is a simplified but more robust approach than manual parsing
        let mut brace_count = 0;
        let mut json_end = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for (i, ch) in content.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => {
                    brace_count += 1;
                }
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        json_end = i + ch.len_utf8();
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if json_end == 0 || brace_count != 0 {
            warn!("Invalid JSON frontmatter format");
            return Ok((HashMap::new(), content.to_string()));
        }
        
        let frontmatter_str = &content[..json_end];
        let markdown_content = content[json_end..].trim();
        
        // Parse JSON and convert to serde_yaml::Value
        let json_value: serde_json::Value = match serde_json::from_str(frontmatter_str) {
            Ok(val) => val,
            Err(e) => {
                warn!("Failed to parse JSON frontmatter: {}", e);
                return Ok((HashMap::new(), content.to_string()));
            }
        };

        // Convert JSON to YAML value
        let yaml_value: serde_yaml::Value = serde_json::from_value(json_value)?;
        let frontmatter = self.yaml_value_to_hashmap(yaml_value);

        debug!("Extracted {} JSON frontmatter fields", frontmatter.len());
        Ok((frontmatter, markdown_content.to_string()))
    }

    /// Helper function to convert serde_yaml::Value to HashMap
    fn yaml_value_to_hashmap(&self, value: serde_yaml::Value) -> HashMap<String, serde_yaml::Value> {
        match value {
            serde_yaml::Value::Mapping(map) => {
                let mut hashmap = HashMap::new();
                for (k, v) in map {
                    if let serde_yaml::Value::String(key) = k {
                        hashmap.insert(key, v);
                    }
                }
                hashmap
            }
            _ => HashMap::new(),
        }
    }

    /// Convert markdown content to HTML
    pub fn markdown_to_html(&self, markdown: &str) -> Result<String> {
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
    #[allow(dead_code)]
    pub fn extract_frontmatter_field<T>(
        &self,
        frontmatter: &HashMap<String, serde_yaml::Value>,
        key: &str,
    ) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        frontmatter
            .get(key)
            .and_then(|value| T::deserialize(value.clone()).ok())
    }

    /// Extract title from frontmatter or generate from content
    #[allow(dead_code)]
    pub fn extract_title(
        &self,
        frontmatter: &HashMap<String, serde_yaml::Value>,
        content: &str,
    ) -> String {
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
    #[allow(dead_code)]
    pub fn extract_tags(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> Vec<String> {
        self.extract_frontmatter_field::<Vec<String>>(frontmatter, "tags")
            .unwrap_or_default()
    }

    /// Extract category from frontmatter
    #[allow(dead_code)]
    pub fn extract_category(
        &self,
        frontmatter: &HashMap<String, serde_yaml::Value>,
    ) -> Option<String> {
        self.extract_frontmatter_field::<String>(frontmatter, "category")
    }

    /// Extract published status from frontmatter (defaults to true)
    #[allow(dead_code)]
    pub fn extract_published(&self, frontmatter: &HashMap<String, serde_yaml::Value>) -> bool {
        self.extract_frontmatter_field::<bool>(frontmatter, "published")
            .unwrap_or(true)
    }

    /// Extract author from frontmatter
    #[allow(dead_code)]
    pub fn extract_author(
        &self,
        frontmatter: &HashMap<String, serde_yaml::Value>,
    ) -> Option<String> {
        self.extract_frontmatter_field::<String>(frontmatter, "author")
    }

    /// Extract excerpt from frontmatter
    #[allow(dead_code)]
    pub fn extract_excerpt(
        &self,
        frontmatter: &HashMap<String, serde_yaml::Value>,
    ) -> Option<String> {
        self.extract_frontmatter_field::<String>(frontmatter, "excerpt")
    }

    /// Generate excerpt from content if not provided in frontmatter
    #[allow(dead_code)]
    pub fn generate_excerpt(&self, content: &str, max_words: usize) -> String {
        let words: Vec<&str> = content.split_whitespace().take(max_words).collect();

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

        assert_eq!(
            result.frontmatter.get("title").unwrap().as_str().unwrap(),
            "Test Post"
        );
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
        frontmatter.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test Title".to_string()),
        );

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

    // 新しいテスト: TOMLフロントマター対応
    #[test]
    fn test_parse_markdown_with_toml_frontmatter() {
        let service = MarkdownService::new();
        let content = r#"+++
title = "TOML Test Post"
tags = ["rust", "toml"]
published = true
+++

# TOML記事

TOMLフロントマターのテスト記事です。"#;

        let result = service.parse_markdown(content).unwrap();

        assert_eq!(
            result.frontmatter.get("title").unwrap().as_str().unwrap(),
            "TOML Test Post"
        );
        assert!(result.html.contains("<h1>TOML記事</h1>"));
        assert!(result.html.contains("<p>TOMLフロントマターのテスト記事です。</p>"));
    }

    // 新しいテスト: JSONフロントマター対応
    #[test]
    fn test_parse_markdown_with_json_frontmatter() {
        let service = MarkdownService::new();
        let content = r#"{
  "title": "JSON Test Post",
  "tags": ["rust", "json"],
  "published": true
}

# JSON記事

JSONフロントマターのテスト記事です。"#;

        let result = service.parse_markdown(content).unwrap();

        assert_eq!(
            result.frontmatter.get("title").unwrap().as_str().unwrap(),
            "JSON Test Post"
        );
        assert!(result.html.contains("<h1>JSON記事</h1>"));
        assert!(result.html.contains("<p>JSONフロントマターのテスト記事です。</p>"));
    }

    // 新しいテスト: カスタムフィールドの保持
    #[test]
    fn test_preserve_custom_fields() {
        let service = MarkdownService::new();
        let content = r#"---
title: "Custom Fields Test"
custom_field: "custom value"
nested:
  field: "nested value"
array_field: [1, 2, 3]
---

# Custom Fields"#;

        let result = service.parse_markdown(content).unwrap();

        assert_eq!(
            result.frontmatter.get("custom_field").unwrap().as_str().unwrap(),
            "custom value"
        );
        assert!(result.frontmatter.contains_key("nested"));
        assert!(result.frontmatter.contains_key("array_field"));
    }

    // 新しいテスト: 無効なフロントマターの優雅な処理
    #[test]
    fn test_parse_markdown_with_invalid_frontmatter() {
        let service = MarkdownService::new();
        let content = r#"---
invalid: yaml: syntax
---

# Content

本文です。"#;

        // 無効なフロントマターの場合、フロントマターなしとして扱う
        let result = service.parse_markdown(content).unwrap();

        // フロントマターが空であることを確認
        assert!(result.frontmatter.is_empty());

        // コンテンツ全体が本文として扱われることを確認
        // pulldown-cmarkは '---' を <hr /> に変換し、'invalid: yaml: syntax' を H2見出しに変換する
        assert!(result.html.contains("<hr />"));
        assert!(result.html.contains("<h2>invalid: yaml: syntax</h2>"));
        assert!(result.html.contains("<h1>Content</h1>"));
        assert!(result.html.contains("<p>本文です。</p>"));
    }
}
