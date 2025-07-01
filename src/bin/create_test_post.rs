use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tobelog::{BlogStorageService, Config, DropboxClient};
use tobelog::services::blog_storage::{BlogPost, BlogPostMetadata};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    dotenv::dotenv().ok();

    info!("🧪 Creating test blog post...");

    let config = Config::from_env()?;
    let dropbox_client = Arc::new(DropboxClient::new(config.dropbox_access_token));
    let blog_storage = Arc::new(BlogStorageService::new(dropbox_client));

    // Create test post metadata
    let metadata = BlogPostMetadata {
        slug: "first-post".to_string(),
        title: "初めての投稿".to_string(),
        published: true,
        category: Some("tech".to_string()),
        tags: vec!["rust".to_string(), "blog".to_string(), "markdown".to_string()],
        author: Some("Tobe Junichiro".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        excerpt: Some("tobelogでの初めての投稿です。Rustで作ったブログシステムの動作テストを行います。".to_string()),
    };

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

    let blog_post = BlogPost {
        metadata,
        content: content.to_string(),
        dropbox_path: "/BlogStorage/posts/first-post.md".to_string(),
        file_metadata: None,
    };

    // Save the post
    info!("📝 Saving test post to Dropbox...");
    blog_storage.save_post(&blog_post, false).await?;

    info!("✅ Test post created successfully!");
    info!("🌐 You can now view it at: http://localhost:3000/");
    info!("📖 Direct link: http://localhost:3000/posts/{}/first-post", Utc::now().format("%Y"));

    Ok(())
}