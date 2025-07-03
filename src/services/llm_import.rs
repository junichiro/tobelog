use chrono::Utc;
use regex::Regex;
use tracing::{debug, warn};

use crate::models::{
    BatchImportRequest, BatchImportResponse, CreatePost, ImportError, ImportSummary,
    LLMArticleImportRequest, LLMArticleImportResponse, LLMSuggestedMetadata,
};
use crate::services::{DatabaseService, MarkdownService};

/// LLM記事インポート処理サービス
#[derive(Clone)]
pub struct LLMImportService {
    markdown_service: MarkdownService,
    database_service: DatabaseService,
}

impl LLMImportService {
    pub fn new(markdown_service: MarkdownService, database_service: DatabaseService) -> Self {
        Self {
            markdown_service,
            database_service,
        }
    }

    /// 単一の記事をインポート処理
    pub async fn process_single_article(
        &self,
        request: LLMArticleImportRequest,
    ) -> Result<LLMArticleImportResponse, Box<dyn std::error::Error + Send + Sync>> {
        debug!("LLMインポート処理開始: source={}", request.source);

        // 1. タイトルの自動抽出
        let title = self.extract_title(&request.content, request.suggested_title.as_deref())?;

        // 2. コンテンツの構造化処理
        let formatted_content = self.structure_content(&request.content)?;

        // 3. HTMLに変換
        let html_content = self.markdown_service.markdown_to_html(&formatted_content)?;

        // 4. 抜粋の生成
        let excerpt = self.generate_excerpt(&formatted_content);

        // 5. スラグの生成
        let slug = self.generate_slug(&title).await?;

        // 6. カテゴリ・タグの提案
        let suggested_category =
            self.suggest_category(&request.content, request.category_hint.as_deref());
        let suggested_tags = self.suggest_tags(&request.content, request.tags_hint.as_ref());

        // 7. Dropboxパスの生成
        let dropbox_path = self.generate_dropbox_path(&slug);

        // 8. メタデータの構築
        let suggested_metadata = LLMSuggestedMetadata {
            title: title.clone(),
            excerpt,
            category: suggested_category,
            tags: suggested_tags,
            author: Some("AI Generated".to_string()),
            source: request.source.clone(),
        };

        // 9. プレビューURLの生成
        let preview_url = format!("/posts/{}/{}", Utc::now().format("%Y"), slug);

        Ok(LLMArticleImportResponse {
            slug,
            suggested_metadata,
            formatted_content,
            html_content,
            preview_url,
            dropbox_path,
        })
    }

    /// バッチインポート処理
    pub async fn process_batch_import(&self, request: BatchImportRequest) -> BatchImportResponse {
        let total_attempted = request.articles.len();
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut duplicates_detected = 0;

        for article in request.articles {
            let content_preview = article.content.chars().take(100).collect::<String>();

            // 重複チェック
            if self.check_duplicate_content(&article.content).await {
                duplicates_detected += 1;
                failed.push(ImportError {
                    content_preview,
                    error_message: "重複するコンテンツが検出されました".to_string(),
                    source: article.source.clone(),
                });
                continue;
            }

            match self.process_single_article(article).await {
                Ok(result) => successful.push(result),
                Err(e) => {
                    failed.push(ImportError {
                        content_preview,
                        error_message: e.to_string(),
                        source: "unknown".to_string(),
                    });
                }
            }
        }

        let summary = ImportSummary {
            total_attempted,
            successful: successful.len(),
            failed: failed.len(),
            duplicates_detected,
        };

        BatchImportResponse {
            successful,
            failed,
            summary,
        }
    }

    /// コンテンツからタイトルを抽出
    fn extract_title(
        &self,
        content: &str,
        suggested_title: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 提案されたタイトルがあればそれを優先
        if let Some(title) = suggested_title {
            if !title.trim().is_empty() {
                return Ok(title.trim().to_string());
            }
        }

        // Markdownのheader（# タイトル）を探す
        let header_regex = Regex::new(r"^#\s+(.+)$")?;
        for line in content.lines() {
            if let Some(captures) = header_regex.captures(line) {
                if let Some(title) = captures.get(1) {
                    return Ok(title.as_str().trim().to_string());
                }
            }
        }

        // 最初の段落を使用（最大50文字）
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let title = if line.len() > 50 {
                    format!("{}...", &line[..47])
                } else {
                    line.to_string()
                };
                return Ok(title);
            }
        }

        // フォールバック
        Ok("Untitled Article".to_string())
    }

    /// コンテンツの構造化処理
    fn structure_content(
        &self,
        content: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut structured = String::new();
        let mut in_code_block = false;
        let mut current_paragraph = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // コードブロックの検出
            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                if !current_paragraph.is_empty() {
                    structured.push_str(&current_paragraph);
                    structured.push_str("\n\n");
                    current_paragraph.clear();
                }
                structured.push_str(line);
                structured.push('\n');
                continue;
            }

            if in_code_block {
                structured.push_str(line);
                structured.push('\n');
                continue;
            }

            // 見出しの検出と整形
            if trimmed.starts_with('#') {
                if !current_paragraph.is_empty() {
                    structured.push_str(&current_paragraph);
                    structured.push_str("\n\n");
                    current_paragraph.clear();
                }
                structured.push_str(&self.format_heading(trimmed));
                structured.push_str("\n\n");
            } else if trimmed.is_empty() {
                if !current_paragraph.is_empty() {
                    structured.push_str(&current_paragraph);
                    structured.push_str("\n\n");
                    current_paragraph.clear();
                }
            } else {
                if !current_paragraph.is_empty() {
                    current_paragraph.push(' ');
                }
                current_paragraph.push_str(trimmed);
            }
        }

        // 残りの段落を追加
        if !current_paragraph.is_empty() {
            structured.push_str(&current_paragraph);
        }

        Ok(structured)
    }

    /// 見出しの形式を整える
    fn format_heading(&self, heading: &str) -> String {
        let level = heading.chars().take_while(|&c| c == '#').count();
        let title = heading.trim_start_matches('#').trim();

        // 適切なレベルに調整（最大H3まで）
        let adjusted_level = level.min(3);
        format!("{} {}", "#".repeat(adjusted_level), title)
    }

    /// 抜粋を生成
    fn generate_excerpt(&self, content: &str) -> Option<String> {
        // 最初の段落または最初の200文字を抜粋として使用
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') && !line.starts_with("```") {
                if line.len() > 200 {
                    return Some(format!("{}...", &line[..197]));
                } else {
                    return Some(line.to_string());
                }
            }
        }
        None
    }

    /// スラグを生成
    async fn generate_slug(
        &self,
        title: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 基本的なスラグ生成
        let mut slug = title
            .to_lowercase()
            .chars()
            .map(|c| {
                match c {
                    'a'..='z' | '0'..='9' => c,
                    ' ' | '-' | '_' => '-',
                    _ => '-', // 日本語や特殊文字は'-'に変換
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        // 長すぎる場合は短縮
        if slug.len() > 50 {
            slug = slug[..50].to_string();
            if let Some(pos) = slug.rfind('-') {
                slug = slug[..pos].to_string();
            }
        }

        // 空の場合のフォールバック
        if slug.is_empty() {
            slug = format!("article-{}", Utc::now().timestamp());
        }

        // 重複チェックしてユニークなスラグを生成
        let mut final_slug = slug.clone();
        let mut counter = 1;

        while self
            .database_service
            .get_post_by_slug(&final_slug)
            .await?
            .is_some()
        {
            final_slug = format!("{}-{}", slug, counter);
            counter += 1;
        }

        Ok(final_slug)
    }

    /// カテゴリを提案
    fn suggest_category(&self, content: &str, hint: Option<&str>) -> Option<String> {
        if let Some(category) = hint {
            return Some(category.to_string());
        }

        // コンテンツから技術的なキーワードを検出
        let tech_keywords = [
            "rust",
            "javascript",
            "python",
            "react",
            "vue",
            "api",
            "database",
            "web",
            "frontend",
            "backend",
            "programming",
            "code",
            "development",
        ];

        let content_lower = content.to_lowercase();
        for keyword in &tech_keywords {
            if content_lower.contains(keyword) {
                return Some("tech".to_string());
            }
        }

        // デフォルトカテゴリ
        Some("general".to_string())
    }

    /// タグを提案
    fn suggest_tags(&self, content: &str, hint: Option<&Vec<String>>) -> Vec<String> {
        if let Some(tags) = hint {
            return tags.clone();
        }

        let mut suggested_tags = Vec::new();
        let content_lower = content.to_lowercase();

        // 技術タグの検出
        let tech_tags = [
            ("rust", "Rust"),
            ("javascript", "JavaScript"),
            ("typescript", "TypeScript"),
            ("python", "Python"),
            ("react", "React"),
            ("vue", "Vue.js"),
            ("api", "API"),
            ("database", "Database"),
            ("web", "Web"),
            ("ai", "AI"),
            ("llm", "LLM"),
        ];

        for (keyword, tag) in &tech_tags {
            if content_lower.contains(keyword) {
                suggested_tags.push(tag.to_string());
            }
        }

        // 最大5個までに制限
        suggested_tags.truncate(5);

        if suggested_tags.is_empty() {
            suggested_tags.push("article".to_string());
        }

        suggested_tags
    }

    /// Dropboxパスを生成
    fn generate_dropbox_path(&self, slug: &str) -> String {
        let year = Utc::now().format("%Y");
        format!("/posts/{}/{}.md", year, slug)
    }

    /// 重複コンテンツをチェック
    async fn check_duplicate_content(&self, content: &str) -> bool {
        // 簡単な重複チェック（実際の実装では内容のハッシュ値を使用することも可能）
        let content_hash = content.len(); // 簡易的な実装

        // 実際の実装では、データベースにハッシュ値を保存して比較する
        warn!(
            "重複チェック機能は簡易実装です: content_length={}",
            content_hash
        );
        false // 現在は常にfalseを返す
    }

    /// CreatePostを生成してデータベースに保存
    pub async fn save_imported_article(
        &self,
        import_response: LLMArticleImportResponse,
        published: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let create_post = CreatePost {
            slug: import_response.slug,
            title: import_response.suggested_metadata.title,
            content: import_response.formatted_content,
            html_content: import_response.html_content,
            excerpt: import_response.suggested_metadata.excerpt,
            category: import_response.suggested_metadata.category,
            tags: import_response.suggested_metadata.tags,
            published,
            featured: false,
            author: import_response.suggested_metadata.author,
            dropbox_path: import_response.dropbox_path,
        };

        self.database_service.create_post(create_post).await?;
        Ok(())
    }
}
