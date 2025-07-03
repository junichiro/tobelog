use anyhow::Result;
use std::sync::Arc;
use tobelog::models::CreatePost;
use tobelog::services::{DatabaseService, MarkdownService};
use tobelog::Config;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    dotenv::dotenv().ok();

    info!("🧪 Creating test blog post with database sync...");

    let config = Config::from_env()?;

    // Initialize services
    let database = Arc::new(DatabaseService::new(&config.database_url).await?);
    let markdown_service = MarkdownService::new();

    // Create test post content
    let content = r#"# 初めての投稿

tobelogブログシステムへようこそ！

## システムについて

このブログシステムは以下の技術で構築されています：

- **Backend**: Rust + Axum
- **Storage**: Dropbox API
- **Database**: SQLite
- **Template**: Tera
- **Frontend**: TailwindCSS

## 特徴

- Dropboxをメインストレージとして使用
- Markdownファイルでの記事管理
- レスポンシブデザイン
- 高速なRustバックエンド

## 次のステップ

今後は以下の機能を追加予定です：

1. 記事作成・編集API
2. 管理画面UI
3. メディアファイル管理
4. カテゴリ・タグ機能

記事の作成が正常に動作していることを確認できました！"#;

    // Convert markdown to HTML
    let html_content = markdown_service.markdown_to_html(content)?;

    // Create test post for database
    let create_post = CreatePost {
        slug: "first-post".to_string(),
        title: "初めての投稿".to_string(),
        content: content.to_string(),
        html_content,
        excerpt: Some(
            "tobelogでの初めての投稿です。Rustで作ったブログシステムの動作テストを行います。"
                .to_string(),
        ),
        category: Some("tech".to_string()),
        tags: vec![
            "rust".to_string(),
            "blog".to_string(),
            "markdown".to_string(),
        ],
        published: true,
        featured: false,
        author: Some("Tobe Junichiro".to_string()),
        dropbox_path: "/BlogStorage/posts/first-post.md".to_string(),
    };

    // Save to database
    info!("💾 Saving test post to database...");
    let post = database.create_post(create_post).await?;

    info!("✅ Test post created successfully!");
    info!("🆔 Post ID: {}", post.id);
    info!("🔗 Slug: {}", post.slug);
    info!("🌐 You can now view it at: http://localhost:3000/");
    info!(
        "📖 Direct link: http://localhost:3000/posts/{}/first-post",
        chrono::Utc::now().format("%Y")
    );

    Ok(())
}
