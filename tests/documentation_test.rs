use std::fs;
use std::path::Path;

#[cfg(test)]
mod documentation_tests {
    use super::*;

    #[test]
    fn test_readme_exists_and_has_content() {
        let readme_path = "README.md";
        assert!(Path::new(readme_path).exists(), "README.md should exist");
        
        let content = fs::read_to_string(readme_path)
            .expect("README.md should be readable");
        
        // ドキュメントの最小要件をチェック
        assert!(content.len() > 1000, "README.md should have substantial content (found {} chars)", content.len());
        assert!(content.contains("# tobelog"), "README.md should have main title");
    }

    #[test]
    fn test_article_update_methods_documented() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // 各記事更新方法がドキュメント化されているかをチェック
        let required_sections = vec![
            "## 記事更新方法",
            "## 1. Dropbox直接編集",
            "## 2. 管理画面（Admin UI）", 
            "## 3. API経由",
            "## 4. LLM生成記事入稿",
            "## 5. Obsidian連携",
        ];

        for section in required_sections {
            assert!(
                readme_content.contains(section),
                "README.md should contain section: {}",
                section
            );
        }
    }

    #[test]
    fn test_comparison_table_exists() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // 比較表が存在することをチェック
        assert!(
            readme_content.contains("方法別比較表"),
            "README.md should contain comparison table"
        );
        
        // 比較項目が含まれていることをチェック
        let comparison_items = vec![
            "技術レベル",
            "作業場所", 
            "一括処理",
            "オフライン",
            "推奨用途",
        ];

        for item in comparison_items {
            assert!(
                readme_content.contains(item),
                "Comparison table should include: {}",
                item
            );
        }
    }

    #[test]
    fn test_api_documentation_completeness() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // API関連のドキュメントがあることをチェック
        let api_sections = vec![
            "API エンドポイント",
            "認証方法",
            "リクエスト例",
            "レスポンス形式",
        ];

        for section in api_sections {
            assert!(
                readme_content.contains(section),
                "API documentation should include: {}",
                section
            );
        }

        // 主要なエンドポイントがドキュメント化されているかチェック
        let endpoints = vec![
            "GET /api/posts",
            "POST /api/posts", 
            "PUT /api/posts/{slug}",
            "DELETE /api/posts/{slug}",
            "POST /api/sync/dropbox",
            "/api/import/llm-article", // URLパターンで検索
        ];

        for endpoint in endpoints {
            assert!(
                readme_content.contains(endpoint),
                "API documentation should include endpoint: {}",
                endpoint
            );
        }
    }

    #[test]
    fn test_faq_section_exists() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // FAQ section exists
        assert!(
            readme_content.contains("よくある質問（FAQ）"),
            "README.md should contain FAQ section"
        );

        // Basic FAQ questions are covered
        let faq_questions = vec![
            "どの更新方法が一番おすすめですか？",
            "記事が反映されません",
            "画像が表示されません",
        ];

        for question in faq_questions {
            assert!(
                readme_content.contains(question),
                "FAQ should include question: {}",
                question
            );
        }
    }

    #[test]
    fn test_troubleshooting_section_exists() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // Troubleshooting section exists
        assert!(
            readme_content.contains("トラブルシューティング"),
            "README.md should contain troubleshooting section"
        );

        // Common issues are documented
        let issues = vec![
            "記事が表示されない",
            "画像が表示されない", 
            "APIが動作しない",
        ];

        for issue in issues {
            assert!(
                readme_content.contains(issue),
                "Troubleshooting should cover: {}",
                issue
            );
        }
    }

    #[test]
    fn test_code_examples_exist() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // Check that code examples are present
        assert!(
            readme_content.contains("```bash"),
            "README.md should contain bash examples"
        );
        
        assert!(
            readme_content.contains("```python"),
            "README.md should contain Python examples"
        );
        
        assert!(
            readme_content.contains("```javascript"),
            "README.md should contain JavaScript examples"
        );
        
        assert!(
            readme_content.contains("```markdown"),
            "README.md should contain Markdown examples"
        );
    }

    #[test]
    fn test_metadata_format_documented() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // Check that metadata format is documented
        assert!(
            readme_content.contains("メタデータの詳細説明"),
            "README.md should document metadata format"
        );

        // Check for required metadata fields
        let metadata_fields = vec![
            "title",
            "category",
            "tags",
            "published",
            "featured",
            "author",
        ];

        for field in metadata_fields {
            assert!(
                readme_content.contains(&format!("`{}`", field)),
                "Metadata documentation should include field: {}",
                field
            );
        }
    }

    #[test]
    fn test_folder_structure_documented() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // Check folder structure documentation
        assert!(
            readme_content.contains("フォルダ構造"),
            "README.md should document folder structure"
        );

        // Check for key folders
        let folders = vec![
            "/posts/",
            "/media/",
            "/drafts/",
        ];

        for folder in folders {
            assert!(
                readme_content.contains(folder),
                "Folder structure should include: {}",
                folder
            );
        }
    }

    #[test]
    fn test_flowchart_exists() {
        let readme_content = fs::read_to_string("README.md")
            .expect("README.md should be readable");
        
        // Check that decision flowchart exists
        assert!(
            readme_content.contains("どの方法を選ぶべきか？"),
            "README.md should contain decision flowchart"
        );
        
        assert!(
            readme_content.contains("```mermaid"),
            "README.md should contain mermaid flowchart"
        );
    }
}