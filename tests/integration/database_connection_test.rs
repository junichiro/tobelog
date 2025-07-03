use std::path::Path;

#[tokio::test]
async fn test_sqliteファイルベース接続が正常に動作する() {
    // ファイルベースSQLite接続をテスト
    let test_db_path = "test_database.db";
    let database_url = format!("sqlite:{}", test_db_path);
    
    // テスト前にファイルを削除
    if Path::new(test_db_path).exists() {
        std::fs::remove_file(test_db_path).unwrap();
    }
    
    // DatabaseServiceを初期化
    let result = tobelog::services::DatabaseService::new(&database_url).await;
    
    assert!(
        result.is_ok(),
        "ファイルベースSQLite接続の初期化に失敗しました: {:?}",
        result.err()
    );
    
    let database = result.unwrap();
    
    // データベースファイルが作成されていることを確認
    assert!(
        Path::new(test_db_path).exists(),
        "データベースファイルが作成されていません"
    );
    
    // 基本的なCRUD操作をテスト
    let create_post = tobelog::models::CreatePost {
        slug: "test-article".to_string(),
        title: "テスト記事".to_string(),
        content: "これはテスト記事です。".to_string(),
        html_content: "<p>これはテスト記事です。</p>".to_string(),
        excerpt: Some("テスト要約".to_string()),
        category: Some("test".to_string()),
        tags: vec!["test".to_string()],
        published: true,
        featured: false,
        author: Some("テストユーザー".to_string()),
        dropbox_path: "/test/article.md".to_string(),
    };
    
    // 記事を作成
    let post_result = database.create_post(create_post).await;
    assert!(
        post_result.is_ok(),
        "記事の作成に失敗しました: {:?}",
        post_result.err()
    );
    
    let created_post = post_result.unwrap();
    
    // 記事を取得
    let retrieved_post = database.get_post_by_slug(&created_post.slug).await;
    assert!(
        retrieved_post.is_ok(),
        "記事の取得に失敗しました: {:?}",
        retrieved_post.err()
    );
    
    let post_option = retrieved_post.unwrap();
    assert!(
        post_option.is_some(),
        "作成した記事が見つかりません"
    );
    
    let post = post_option.unwrap();
    assert_eq!(post.title, "テスト記事");
    assert_eq!(post.content, "これはテスト記事です。");
    
    // テスト後のクリーンアップ
    if Path::new(test_db_path).exists() {
        std::fs::remove_file(test_db_path).unwrap();
    }
}

#[tokio::test]
async fn test_データベースディレクトリ作成機能() {
    // ネストしたディレクトリパスでのデータベース作成をテスト
    let test_dir = "test_data/nested/database";
    let test_db_path = format!("{}/test.db", test_dir);
    let database_url = format!("sqlite:{}", test_db_path);
    
    // テスト前にディレクトリを削除
    if Path::new("test_data").exists() {
        std::fs::remove_dir_all("test_data").unwrap();
    }
    
    // DatabaseServiceを初期化（ディレクトリが自動作成されるべき）
    let result = tobelog::services::DatabaseService::new(&database_url).await;
    
    assert!(
        result.is_ok(),
        "ネストしたディレクトリでのデータベース初期化に失敗しました: {:?}",
        result.err()
    );
    
    // ディレクトリとファイルが作成されていることを確認
    assert!(
        Path::new(test_dir).exists(),
        "データベースディレクトリが作成されていません"
    );
    
    assert!(
        Path::new(&test_db_path).exists(),
        "データベースファイルが作成されていません"
    );
    
    // テスト後のクリーンアップ
    if Path::new("test_data").exists() {
        std::fs::remove_dir_all("test_data").unwrap();
    }
}

#[tokio::test]
async fn test_メインサーバー起動時のデータベース接続() {
    use tobelog::config::Config;
    
    // テスト用の環境変数を設定
    std::env::set_var("DATABASE_URL", "sqlite://test_server.db");
    std::env::set_var("SERVER_HOST", "127.0.0.1");
    std::env::set_var("SERVER_PORT", "3001");
    std::env::set_var("DROPBOX_ACCESS_TOKEN", "test_token");
    std::env::set_var("API_KEY", "test_api_key");
    std::env::set_var("BLOG_TITLE", "Test Blog");
    
    let test_db_path = "test_server.db";
    
    // テスト前にファイルを削除
    if Path::new(test_db_path).exists() {
        std::fs::remove_file(test_db_path).unwrap();
    }
    
    // 設定を読み込み
    let config_result = Config::from_env();
    assert!(
        config_result.is_ok(),
        "設定の読み込みに失敗しました: {:?}",
        config_result.err()
    );
    
    let config = config_result.unwrap();
    
    // メインアプリケーションと同じ方法でDatabaseServiceを初期化
    let database_result = tobelog::services::DatabaseService::new(&config.database_url).await;
    assert!(
        database_result.is_ok(),
        "メインサーバー起動時のデータベース接続に失敗しました: {:?}",
        database_result.err()
    );
    
    // データベースファイルが作成されていることを確認
    assert!(
        Path::new(test_db_path).exists(),
        "メインサーバー用データベースファイルが作成されていません"
    );
    
    // テスト後のクリーンアップ
    if Path::new(test_db_path).exists() {
        std::fs::remove_file(test_db_path).unwrap();
    }
}