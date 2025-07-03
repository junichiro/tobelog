use crate::handlers::api::{
    ContentQualityResult, DuplicateCheckResult, LLMArticlePreview, LLMFormat,
    MetadataCompletenessResult, ObsidianIntegration, QualityCheckResults, SimilarPost,
};
use crate::services::{DatabaseService, MarkdownService};
use anyhow::Result;
use regex::Regex;
use tracing::info;

/// Service for processing LLM-generated content
#[derive(Clone)]
pub struct LLMProcessorService {
    database: DatabaseService,
    markdown: MarkdownService,
}

impl LLMProcessorService {
    pub fn new(database: DatabaseService, markdown: MarkdownService) -> Self {
        Self { database, markdown }
    }

    /// Process LLM content and return a preview with extracted metadata
    pub async fn process_llm_content(
        &self,
        content: &str,
        format: &LLMFormat,
        obsidian_integration: Option<&ObsidianIntegration>,
        _auto_structure: bool,
        auto_metadata: bool,
    ) -> Result<LLMArticlePreview> {
        info!("Processing LLM content with format: {:?}", format);

        // Step 1: Clean and structure the content based on format
        let structured_content = self.structure_content(content, format, obsidian_integration)?;

        // Step 2: Extract metadata if auto_metadata is enabled
        let (extracted_title, extracted_category, extracted_tags) = if auto_metadata {
            self.extract_metadata(&structured_content).await?
        } else {
            (None, None, Vec::new())
        };

        // Step 3: Generate excerpt
        let excerpt = self.generate_excerpt(&structured_content, 200);

        // Step 4: Calculate reading metrics
        let word_count = self.count_words(&structured_content);
        let estimated_reading_time = self.estimate_reading_time(word_count);

        Ok(LLMArticlePreview {
            structured_content,
            extracted_title,
            extracted_category,
            extracted_tags,
            excerpt,
            word_count,
            estimated_reading_time,
        })
    }

    /// Perform quality checks on the content
    pub async fn quality_check(
        &self,
        content: &str,
        title: Option<&str>,
        category: Option<&str>,
        tags: &[String],
    ) -> Result<QualityCheckResults> {
        info!("Performing quality checks on content");

        // Duplicate check
        let duplicate_check = self.check_for_duplicates(content, title).await?;

        // Content quality check
        let content_quality = self.assess_content_quality(content);

        // Metadata completeness check
        let metadata_completeness = self.assess_metadata_completeness(title, category, tags);

        Ok(QualityCheckResults {
            duplicate_check,
            content_quality,
            metadata_completeness,
        })
    }

    /// Structure content based on LLM format
    fn structure_content(
        &self,
        content: &str,
        format: &LLMFormat,
        obsidian_integration: Option<&ObsidianIntegration>,
    ) -> Result<String> {
        let mut structured = content.to_string();

        match format {
            LLMFormat::ChatGPT => {
                structured = self.clean_chatgpt_format(&structured)?;
            }
            LLMFormat::Claude => {
                structured = self.clean_claude_format(&structured)?;
            }
            LLMFormat::PlainText => {
                structured = self.structure_plain_text(&structured)?;
            }
            LLMFormat::Obsidian => {
                structured = self.process_obsidian_format(&structured, obsidian_integration)?;
            }
        }

        // Apply auto-structuring if needed
        structured = self.auto_structure_content(&structured)?;

        Ok(structured)
    }

    /// Clean ChatGPT-specific formatting
    fn clean_chatgpt_format(&self, content: &str) -> Result<String> {
        let mut cleaned = content.to_string();

        // Remove common ChatGPT artifacts
        cleaned = cleaned.replace("ChatGPT:", "");
        cleaned = cleaned.replace("Assistant:", "");
        cleaned = cleaned.replace("AI:", "");

        // Remove conversation markers
        let conversation_re = Regex::new(r"(?m)^(User|Human|You):\s*.*$")?;
        cleaned = conversation_re.replace_all(&cleaned, "").to_string();

        // Clean up extra whitespace
        cleaned = self.normalize_whitespace(&cleaned);

        Ok(cleaned)
    }

    /// Clean Claude-specific formatting
    fn clean_claude_format(&self, content: &str) -> Result<String> {
        let mut cleaned = content.to_string();

        // Remove Claude artifacts
        cleaned = cleaned.replace("Claude:", "");
        cleaned = cleaned.replace("Assistant:", "");

        // Remove thinking tags if present
        let thinking_re = Regex::new(r"<thinking>.*?</thinking>")?;
        cleaned = thinking_re.replace_all(&cleaned, "").to_string();

        // Remove artifact references
        let artifact_re = Regex::new(r"I'll.*?artifact.*?\.")?;
        cleaned = artifact_re.replace_all(&cleaned, "").to_string();

        cleaned = self.normalize_whitespace(&cleaned);

        Ok(cleaned)
    }

    /// Structure plain text into markdown
    fn structure_plain_text(&self, content: &str) -> Result<String> {
        let mut structured = content.to_string();

        // Auto-detect and add headings
        structured = self.auto_add_headings(&structured)?;

        // Format lists
        structured = self.format_lists(&structured)?;

        // Add paragraph breaks
        structured = self.add_paragraph_breaks(&structured)?;

        Ok(structured)
    }

    /// Process Obsidian-specific formatting
    fn process_obsidian_format(
        &self,
        content: &str,
        integration: Option<&ObsidianIntegration>,
    ) -> Result<String> {
        let mut processed = content.to_string();

        if let Some(settings) = integration {
            // Convert WikiLinks if requested
            if settings.convert_wikilinks.unwrap_or(true) {
                processed = self.convert_wikilinks(&processed)?;
            }

            // Preserve Obsidian tags if requested
            if settings.preserve_tags.unwrap_or(true) {
                processed = self.preserve_obsidian_tags(&processed)?;
            }
        }

        Ok(processed)
    }

    /// Convert Obsidian WikiLinks to standard markdown links
    fn convert_wikilinks(&self, content: &str) -> Result<String> {
        let wikilink_re = Regex::new(r"\[\[([^\]]+)\]\]")?;
        let converted = wikilink_re.replace_all(content, |caps: &regex::Captures| {
            let link_text = &caps[1];
            if link_text.contains('|') {
                let parts: Vec<&str> = link_text.split('|').collect();
                if parts.len() == 2 {
                    format!("[{}]({})", parts[1], parts[0])
                } else {
                    format!("[{}]({})", link_text, link_text)
                }
            } else {
                format!("[{}]({})", link_text, link_text)
            }
        });
        Ok(converted.to_string())
    }

    /// Preserve Obsidian tags in content
    fn preserve_obsidian_tags(&self, content: &str) -> Result<String> {
        // Obsidian tags are preserved as-is since they're compatible with most markdown processors
        Ok(content.to_string())
    }

    /// Auto-structure content by adding headings and formatting
    fn auto_structure_content(&self, content: &str) -> Result<String> {
        let mut structured = content.to_string();

        // Detect and format headings based on content patterns
        structured = self.auto_add_headings(&structured)?;

        // Format code blocks
        structured = self.format_code_blocks(&structured)?;

        // Format lists
        structured = self.format_lists(&structured)?;

        // Add proper spacing
        structured = self.normalize_spacing(&structured);

        Ok(structured)
    }

    /// Auto-detect and add markdown headings
    fn auto_add_headings(&self, content: &str) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip if already a heading
            if trimmed.starts_with('#') {
                result.push(line.to_string());
                continue;
            }

            // Detect potential headings (short lines followed by content)
            if trimmed.len() > 0 && trimmed.len() < 80 && !trimmed.ends_with('.') {
                if i + 1 < lines.len() && !lines[i + 1].trim().is_empty() {
                    // Check if this looks like a heading
                    if self.looks_like_heading(trimmed) {
                        result.push(format!("## {}", trimmed));
                    } else {
                        result.push(line.to_string());
                    }
                } else {
                    result.push(line.to_string());
                }
            } else {
                result.push(line.to_string());
            }
        }

        Ok(result.join("\n"))
    }

    /// Check if a line looks like a heading
    fn looks_like_heading(&self, line: &str) -> bool {
        // Simple heuristics for heading detection
        let has_title_case = line.chars().next().map_or(false, |c| c.is_uppercase());
        let has_no_punctuation = !line.contains('.') && !line.contains(',');
        let reasonable_length = line.len() >= 3 && line.len() <= 60;

        has_title_case && has_no_punctuation && reasonable_length
    }

    /// Format code blocks
    fn format_code_blocks(&self, content: &str) -> Result<String> {
        // This is a simple implementation - in practice, you might want more sophisticated code detection
        let code_re = Regex::new(r"```(.*?)```")?;
        let formatted = code_re.replace_all(content, |caps: &regex::Captures| {
            let code = &caps[1];
            format!("```\n{}\n```", code.trim())
        });
        Ok(formatted.to_string())
    }

    /// Format lists
    fn format_lists(&self, content: &str) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        for line in lines {
            let trimmed = line.trim();

            // Detect list items
            if trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
            {
                result.push(line.to_string());
            } else if trimmed.len() > 2 && (trimmed.starts_with("• ") || trimmed.starts_with("◦ "))
            {
                // Convert bullet points to markdown lists
                result.push(format!("- {}", &trimmed[2..]));
            } else {
                result.push(line.to_string());
            }
        }

        Ok(result.join("\n"))
    }

    /// Add proper paragraph breaks
    fn add_paragraph_breaks(&self, content: &str) -> Result<String> {
        let paragraph_re = Regex::new(r"\n{3,}")?;
        let formatted = paragraph_re.replace_all(content, "\n\n");
        Ok(formatted.to_string())
    }

    /// Normalize whitespace
    fn normalize_whitespace(&self, content: &str) -> String {
        content
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Normalize spacing between elements
    fn normalize_spacing(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut prev_was_empty = false;

        for line in lines {
            let is_empty = line.trim().is_empty();

            if is_empty && prev_was_empty {
                // Skip multiple empty lines
                continue;
            }

            result.push(line.to_string());
            prev_was_empty = is_empty;
        }

        result.join("\n")
    }

    /// Extract metadata from content
    async fn extract_metadata(
        &self,
        content: &str,
    ) -> Result<(Option<String>, Option<String>, Vec<String>)> {
        let title = self.extract_title(content);
        let category = self.extract_category(content).await?;
        let tags = self.extract_tags(content);

        Ok((title, category, tags))
    }

    /// Extract title from content
    fn extract_title(&self, content: &str) -> Option<String> {
        // Look for first heading
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                return Some(trimmed.trim_start_matches("# ").to_string());
            }
        }

        // Look for title-like first line
        let first_line = content.lines().next()?.trim();
        if first_line.len() > 3 && first_line.len() < 100 && !first_line.ends_with('.') {
            Some(first_line.to_string())
        } else {
            None
        }
    }

    /// Extract category from content using simple keyword matching
    async fn extract_category(&self, content: &str) -> Result<Option<String>> {
        // Get existing categories from database
        let stats = self.database.get_post_stats().await?;
        let existing_categories: Vec<String> =
            stats.categories.iter().map(|c| c.name.clone()).collect();

        // Simple keyword matching
        let content_lower = content.to_lowercase();
        for category in existing_categories {
            if content_lower.contains(&category.to_lowercase()) {
                return Ok(Some(category));
            }
        }

        // Default categories based on content analysis
        if content_lower.contains("tech")
            || content_lower.contains("programming")
            || content_lower.contains("code")
            || content_lower.contains("software")
        {
            Ok(Some("tech".to_string()))
        } else if content_lower.contains("tutorial")
            || content_lower.contains("how to")
            || content_lower.contains("guide")
        {
            Ok(Some("tutorial".to_string()))
        } else {
            Ok(None)
        }
    }

    /// Extract tags from content
    fn extract_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // Look for Obsidian-style tags
        let tag_re = Regex::new(r"#(\w+)").unwrap();
        for cap in tag_re.captures_iter(content) {
            if let Some(tag) = cap.get(1) {
                tags.push(tag.as_str().to_string());
            }
        }

        // Look for common keywords
        let content_lower = content.to_lowercase();
        let keyword_tags = [
            ("rust", "rust"),
            ("javascript", "javascript"),
            ("python", "python"),
            ("tutorial", "tutorial"),
            ("api", "api"),
            ("database", "database"),
            ("web", "web"),
            ("frontend", "frontend"),
            ("backend", "backend"),
        ];

        for (keyword, tag) in keyword_tags {
            if content_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        tags
    }

    /// Generate excerpt from content
    fn generate_excerpt(&self, content: &str, max_length: usize) -> String {
        let text = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join(" ");

        if text.len() <= max_length {
            text
        } else {
            format!("{}...", &text[..max_length])
        }
    }

    /// Count words in content
    fn count_words(&self, content: &str) -> usize {
        content.split_whitespace().count()
    }

    /// Estimate reading time based on word count
    fn estimate_reading_time(&self, word_count: usize) -> usize {
        // Average reading speed: 200 words per minute
        (word_count as f32 / 200.0).ceil() as usize
    }

    /// Check for duplicate content
    async fn check_for_duplicates(
        &self,
        content: &str,
        title: Option<&str>,
    ) -> Result<DuplicateCheckResult> {
        // Get all existing posts
        let posts = self.database.list_posts(Default::default()).await?;

        let mut similar_posts = Vec::new();
        let mut max_similarity: f32 = 0.0;

        for post in posts {
            let similarity =
                self.calculate_similarity(content, &post.content, title, Some(&post.title));

            if similarity > 0.7 {
                similar_posts.push(SimilarPost {
                    slug: post.slug,
                    title: post.title,
                    similarity_score: similarity,
                });
                max_similarity = max_similarity.max(similarity);
            }
        }

        // Sort by similarity score
        similar_posts.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        Ok(DuplicateCheckResult {
            is_duplicate: max_similarity > 0.9,
            similar_posts,
            similarity_score: if max_similarity > 0.0 {
                Some(max_similarity)
            } else {
                None
            },
        })
    }

    /// Calculate similarity between two pieces of content
    fn calculate_similarity(
        &self,
        content1: &str,
        content2: &str,
        title1: Option<&str>,
        title2: Option<&str>,
    ) -> f32 {
        // Simple similarity calculation based on common words
        let words1: std::collections::HashSet<&str> = content1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = content2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        let content_similarity = if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        };

        // Factor in title similarity if both titles exist
        let title_similarity = if let (Some(t1), Some(t2)) = (title1, title2) {
            if t1.to_lowercase() == t2.to_lowercase() {
                1.0
            } else {
                let t1_words: std::collections::HashSet<&str> = t1.split_whitespace().collect();
                let t2_words: std::collections::HashSet<&str> = t2.split_whitespace().collect();
                let t_intersection = t1_words.intersection(&t2_words).count();
                let t_union = t1_words.union(&t2_words).count();

                if t_union > 0 {
                    t_intersection as f32 / t_union as f32
                } else {
                    0.0
                }
            }
        } else {
            0.0
        };

        // Weighted combination
        (content_similarity * 0.7) + (title_similarity * 0.3)
    }

    /// Assess content quality
    fn assess_content_quality(&self, content: &str) -> ContentQualityResult {
        let word_count = self.count_words(content);
        let has_headings = content.contains('#');
        let has_images = content.contains("![") || content.contains("<img");
        let mut issues = Vec::new();

        // Check for common issues
        if word_count < 100 {
            issues.push("Content is very short (less than 100 words)".to_string());
        }

        if !has_headings {
            issues.push("No headings found - consider adding section headers".to_string());
        }

        if content.lines().count() < 5 {
            issues.push("Content has very few paragraphs".to_string());
        }

        // Simple readability score (placeholder - could be more sophisticated)
        let readability_score = if word_count > 0 {
            let avg_sentence_length = content.split('.').count() as f32 / word_count as f32;
            Some((100.0 - (avg_sentence_length * 10.0)).max(0.0).min(100.0))
        } else {
            None
        };

        ContentQualityResult {
            word_count,
            has_headings,
            has_images,
            readability_score,
            issues,
        }
    }

    /// Assess metadata completeness
    fn assess_metadata_completeness(
        &self,
        title: Option<&str>,
        category: Option<&str>,
        tags: &[String],
    ) -> MetadataCompletenessResult {
        let has_title = title.is_some();
        let has_category = category.is_some();
        let has_tags = !tags.is_empty();

        let score_components = [has_title, has_category, has_tags];
        let completeness_score = score_components.iter().filter(|&&x| x).count() as f32 / 3.0;

        let mut suggestions = Vec::new();
        if !has_title {
            suggestions.push("Consider adding a descriptive title".to_string());
        }
        if !has_category {
            suggestions.push("Adding a category will help organize your content".to_string());
        }
        if !has_tags {
            suggestions.push("Tags will help readers discover your content".to_string());
        }

        MetadataCompletenessResult {
            has_title,
            has_category,
            has_tags,
            completeness_score,
            suggestions,
        }
    }
}
