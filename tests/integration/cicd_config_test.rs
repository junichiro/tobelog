use std::fs;
use std::path::Path;

#[test]
fn test_github_actions_workflow_存在確認() {
    let workflow_path = ".github/workflows/ci-cd.yml";
    assert!(
        Path::new(workflow_path).exists(),
        "GitHub Actions workflowファイルが存在しません: {}",
        workflow_path
    );
}

#[test]
fn test_github_actions_workflow_内容確認() {
    let workflow_path = ".github/workflows/ci-cd.yml";
    let workflow_content =
        fs::read_to_string(workflow_path).expect("GitHub Actions workflowファイルを読み込めません");

    // 必須要素の確認
    assert!(
        workflow_content.contains("name: CI/CD Pipeline"),
        "ワークフロー名が設定されていません"
    );
    assert!(
        workflow_content.contains("on:"),
        "トリガー設定が見つかりません"
    );
    assert!(
        workflow_content.contains("jobs:"),
        "ジョブ設定が見つかりません"
    );
    assert!(
        workflow_content.contains("cargo test"),
        "テスト実行が設定されていません"
    );
    assert!(
        workflow_content.contains("cargo clippy"),
        "Clippy実行が設定されていません"
    );
    assert!(
        workflow_content.contains("cargo fmt"),
        "フォーマット確認が設定されていません"
    );
}

#[test]
fn test_security_workflow_存在確認() {
    let security_workflow_path = ".github/workflows/security.yml";
    assert!(
        Path::new(security_workflow_path).exists(),
        "セキュリティワークフローファイルが存在しません: {}",
        security_workflow_path
    );
}

#[test]
fn test_readme_存在確認() {
    let readme_path = "README.md";
    assert!(
        Path::new(readme_path).exists(),
        "README.mdファイルが存在しません: {}",
        readme_path
    );
}

#[test]
fn test_readme_内容確認() {
    let readme_path = "README.md";
    let readme_content =
        fs::read_to_string(readme_path).expect("README.mdファイルを読み込めません");

    // 必須セクションの確認
    assert!(
        readme_content.contains("# tobelog"),
        "プロジェクト名が設定されていません"
    );
    assert!(
        readme_content.contains("## 概要"),
        "概要セクションが見つかりません"
    );
    assert!(
        readme_content.contains("## セットアップ"),
        "セットアップセクションが見つかりません"
    );
    assert!(
        readme_content.contains("## 使用方法"),
        "使用方法セクションが見つかりません"
    );
    assert!(
        readme_content.contains("## API仕様"),
        "API仕様セクションが見つかりません"
    );
}

#[test]
fn test_api_spec_存在確認() {
    let api_spec_path = "docs/api-specification.md";
    assert!(
        Path::new(api_spec_path).exists(),
        "API仕様書が存在しません: {}",
        api_spec_path
    );
}

#[test]
fn test_gitignore_適切な設定確認() {
    let gitignore_path = ".gitignore";
    let gitignore_content =
        fs::read_to_string(gitignore_path).expect(".gitignoreファイルを読み込めません");

    // 必須の除外設定確認
    assert!(
        gitignore_content.contains("target/"),
        "Rustビルドディレクトリが除外されていません"
    );
    assert!(
        gitignore_content.contains("*.env"),
        "環境変数ファイルが除外されていません"
    );
    assert!(
        gitignore_content.contains("*.db"),
        "データベースファイルが除外されていません"
    );
}

#[test]
fn test_env_example_存在確認() {
    let env_example_path = ".env.example";
    assert!(
        Path::new(env_example_path).exists(),
        "環境変数テンプレートファイルが存在しません: {}",
        env_example_path
    );
}
