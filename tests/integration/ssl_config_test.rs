use std::fs;
use std::path::Path;

#[test]
fn test_nginx_config_存在確認() {
    let nginx_config_path = "nginx/nginx.conf";
    assert!(
        Path::new(nginx_config_path).exists(),
        "nginx設定ファイルが存在しません: {}",
        nginx_config_path
    );
}

#[test]
fn test_nginx_config_ssl設定確認() {
    let nginx_config_path = "nginx/nginx.conf";
    let config_content =
        fs::read_to_string(nginx_config_path).expect("nginx設定ファイルを読み込めません");

    // SSL設定の必須項目をチェック
    assert!(
        config_content.contains("ssl_certificate"),
        "SSL証明書設定が見つかりません"
    );
    assert!(
        config_content.contains("ssl_certificate_key"),
        "SSL秘密鍵設定が見つかりません"
    );
    assert!(
        config_content.contains("listen 443 ssl"),
        "SSL接続設定が見つかりません"
    );
    assert!(
        config_content.contains("return 301 https://"),
        "HTTP→HTTPSリダイレクト設定が見つかりません"
    );
}

#[test]
fn test_nginx_config_セキュリティヘッダー確認() {
    let nginx_config_path = "nginx/nginx.conf";
    let config_content =
        fs::read_to_string(nginx_config_path).expect("nginx設定ファイルを読み込めません");

    // セキュリティヘッダーの確認
    assert!(
        config_content.contains("X-Frame-Options"),
        "X-Frame-Optionsヘッダーが設定されていません"
    );
    assert!(
        config_content.contains("X-Content-Type-Options"),
        "X-Content-Type-Optionsヘッダーが設定されていません"
    );
    assert!(
        config_content.contains("X-XSS-Protection"),
        "X-XSS-Protectionヘッダーが設定されていません"
    );
}

#[test]
fn test_ssl_renewal_script_存在確認() {
    let ssl_script_path = "scripts/ssl-renewal.sh";
    assert!(
        Path::new(ssl_script_path).exists(),
        "SSL証明書更新スクリプトが存在しません: {}",
        ssl_script_path
    );
}

#[test]
fn test_docker_compose_ssl対応確認() {
    let docker_compose_path = "docker-compose.production.yml";
    let compose_content = fs::read_to_string(docker_compose_path)
        .expect("docker-compose.production.ymlを読み込めません");

    // SSL関連の設定確認
    assert!(
        compose_content.contains("nginx"),
        "nginxサービスが設定されていません"
    );
    assert!(
        compose_content.contains("443:443"),
        "HTTPSポート設定が見つかりません"
    );
    assert!(
        compose_content.contains("letsencrypt"),
        "Let's Encrypt設定が見つかりません"
    );
}
