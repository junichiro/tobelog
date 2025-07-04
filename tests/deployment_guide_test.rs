use std::fs;

/// Tests for unified deployment guide documentation completeness
mod deployment_guide_tests {
    use super::*;

    #[test]
    fn test_deployment_md_exists_and_has_content() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should exist and be readable");
        
        assert!(!deployment_content.is_empty(), "DEPLOYMENT.md should not be empty");
        assert!(deployment_content.contains("本番デプロイ方法の統一ガイド"), 
                "DEPLOYMENT.md should contain the main title");
    }

    #[test]
    fn test_deployment_method_comparison_table() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // デプロイ方法の比較表が存在することを確認
        assert!(deployment_content.contains("## デプロイ方法の比較"), 
                "Should contain deployment method comparison section");
        assert!(deployment_content.contains("| 方法 | 適用場面 | 難易度 | 特徴 |"), 
                "Should contain comparison table headers");
        
        // 各デプロイ方法が比較表に含まれていることを確認
        let deployment_methods = vec![
            "Docker Compose",
            "systemd",
            "CI/CD"
        ];
        
        for method in deployment_methods {
            assert!(deployment_content.contains(method), 
                    "Comparison table should include: {}", method);
        }
    }

    #[test]
    fn test_decision_flowchart_exists() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 意思決定フローチャートが存在することを確認
        assert!(deployment_content.contains("## デプロイ方法の選択"), 
                "Should contain decision flowchart section");
        assert!(deployment_content.contains("```mermaid"), 
                "Should contain Mermaid flowchart");
        assert!(deployment_content.contains("個人・小規模"), 
                "Flowchart should include personal/small scale option");
        assert!(deployment_content.contains("企業・チーム"), 
                "Flowchart should include enterprise/team option");
    }

    #[test]
    fn test_quick_start_guide_exists() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 15分クイックスタートガイドが存在することを確認
        assert!(deployment_content.contains("## 🚀 15分クイックスタート"), 
                "Should contain 15-minute quick start guide");
        assert!(deployment_content.contains("### 前提条件"), 
                "Quick start should include prerequisites");
        assert!(deployment_content.contains("### 手順"), 
                "Quick start should include step-by-step instructions");
        assert!(deployment_content.contains("docker-compose"), 
                "Quick start should mention docker-compose commands");
    }

    #[test]
    fn test_staged_deployment_guide_exists() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 段階的セットアップガイドが存在することを確認
        assert!(deployment_content.contains("## 📚 段階的セットアップ"), 
                "Should contain staged setup guide");
        
        let stages = vec![
            "Stage 1: 基本動作確認",
            "Stage 2: 本番環境セットアップ", 
            "Stage 3: 運用設定"
        ];
        
        for stage in stages {
            assert!(deployment_content.contains(stage), 
                    "Staged setup should include: {}", stage);
        }
    }

    #[test]
    fn test_target_audience_documentation() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 読者層別案内が存在することを確認
        assert!(deployment_content.contains("## 👥 読者層別ガイド"), 
                "Should contain target audience guide");
        
        let audience_levels = vec![
            "初心者",
            "中級者", 
            "上級者"
        ];
        
        for level in audience_levels {
            assert!(deployment_content.contains(level), 
                    "Should include guidance for: {}", level);
        }
    }

    #[test]
    fn test_links_to_detailed_docs() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 詳細ドキュメントへのリンクが存在することを確認
        let detailed_docs = vec![
            "DOCKER.md",
            "SYSTEMD.md",
            "DEVELOPMENT.md"
        ];
        
        for doc in detailed_docs {
            assert!(deployment_content.contains(doc), 
                    "Should link to detailed documentation: {}", doc);
        }
    }

    #[test] 
    fn test_environment_specific_sections() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 環境別セクションが存在することを確認
        let environments = vec![
            "個人ブログ",
            "小規模チーム",
            "企業環境"
        ];
        
        for env in environments {
            assert!(deployment_content.contains(env), 
                    "Should include environment-specific guidance for: {}", env);
        }
    }

    #[test]
    fn test_troubleshooting_integration() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 統合トラブルシューティングが存在することを確認
        assert!(deployment_content.contains("## 🔧 トラブルシューティング"), 
                "Should contain troubleshooting section");
        assert!(deployment_content.contains("よくある問題"), 
                "Should include common issues");
        assert!(deployment_content.contains("症状別解決方法"), 
                "Should include symptom-based solutions");
    }

    #[test]
    fn test_maintenance_and_operations_guide() {
        let deployment_content = fs::read_to_string("DEPLOYMENT.md")
            .expect("DEPLOYMENT.md should be readable");

        // 運用・保守ガイドが存在することを確認
        assert!(deployment_content.contains("## 🔄 運用・保守"), 
                "Should contain operations and maintenance guide");
        assert!(deployment_content.contains("ログ確認"), 
                "Should include log checking procedures");
        assert!(deployment_content.contains("バックアップ"), 
                "Should include backup procedures");
        assert!(deployment_content.contains("アップデート"), 
                "Should include update procedures");
    }
}