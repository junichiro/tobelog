# Docker Deployment Guide

## 概要

このドキュメントは、tobelogアプリケーションのDocker化とデプロイメントに関する情報を提供します。

## 前提条件

- Docker Engine 20.10+
- Docker Compose 2.0+
- Dropbox API アクセストークン

## 環境設定

### 1. 環境変数の設定

```bash
# .env.example をコピーして環境変数を設定
cp .env.example .env

# 必要な値を設定（すべての環境変数が必須です）
vi .env
```

**重要**: `DROPBOX_ACCESS_TOKEN`と`API_KEY`は必須の環境変数です。設定されていない場合、コンテナの起動に失敗します。

### 2. Dropbox API トークン取得

1. [Dropbox App Console](https://www.dropbox.com/developers/apps) にアクセス
2. 新しいアプリを作成
3. 必要な権限を設定:
   - `files.content.read`
   - `files.content.write`
4. アクセストークンを取得し、`.env` ファイルに設定

## 開発環境

### 基本的な起動

```bash
# 開発環境での起動
docker-compose up

# または明示的に開発環境を指定
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
```

### 開発環境の特徴

- ソースコードのマウント（ホットリロード対応）
- デバッグポート（9229）の公開
- 詳細なログ出力
- 緩和されたセキュリティ設定

## ステージング環境

```bash
# ステージング環境での起動
docker-compose -f docker-compose.yml -f docker-compose.staging.yml up -d

# ログの確認
docker-compose -f docker-compose.yml -f docker-compose.staging.yml logs -f
```

### ステージング環境の特徴

- ポート 3001 で起動
- リソース制限設定
- 本番環境に近いセキュリティ設定

## 本番環境

```bash
# 本番環境での起動
docker-compose -f docker-compose.yml -f docker-compose.production.yml up -d

# ヘルスチェック
docker-compose -f docker-compose.yml -f docker-compose.production.yml ps
```

### 本番環境の特徴

- 外部ポート非公開（リバースプロキシ前提）
- 最小限のログ出力
- 最大限のセキュリティ設定
- リソース制限とログローテーション

## 便利スクリプト

### ビルドスクリプト

```bash
# 開発環境用ビルド
./docker-scripts/build.sh dev

# 本番環境用ビルド
./docker-scripts/build.sh production
```

### デプロイスクリプト

```bash
# 開発環境デプロイ
./docker-scripts/deploy.sh dev

# 本番環境デプロイ
./docker-scripts/deploy.sh production
```

## データ永続化

### ボリューム管理

```bash
# ボリュームの一覧
docker volume ls | grep tobelog

# ボリュームの詳細
docker volume inspect tobelog-blog-data

# バックアップ
docker run --rm -v tobelog-blog-data:/data -v $(pwd):/backup alpine tar czf /backup/blog-data-backup.tar.gz /data
```

### データベース管理

```bash
# データベースへの接続
docker-compose exec tobelog sqlite3 /home/app/data/blog.db

# マイグレーション実行
docker-compose exec tobelog /usr/local/bin/tobelog migrate
```

## セキュリティ

### セキュリティ設定の概要

- 非rootユーザーでの実行 (UID: 1001)
- 最小限の権限設定（すべての capabilities を削除）
- `no-new-privileges` フラグ
- 読み取り専用ルートファイルシステム（マウントされたボリュームは書き込み可能）

### セキュリティスキャン

```bash
# Dockerイメージのセキュリティスキャン
docker scan tobelog:latest

# 脆弱性チェック
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd):/app aquasec/trivy image tobelog:latest
```

## 監視とログ

### ヘルスチェック

アプリケーションは `/health` エンドポイントでヘルスチェックを提供します。

```bash
# ヘルスチェック状態の確認
docker-compose ps

# 手動でヘルスチェック
curl -f http://localhost:3000/health
```

### ログ管理

```bash
# ログの確認
docker-compose logs -f tobelog

# ログの検索
docker-compose logs tobelog | grep ERROR
```

## トラブルシューティング

### よくある問題

1. **Dropbox API エラー**
   - アクセストークンの確認
   - API制限の確認 (500req/min)

2. **データベース接続エラー**
   - ボリュームマウントの確認
   - ファイル権限の確認

3. **ポート競合**
   - 他のサービスとのポート競合
   - docker-compose.override.yml の確認

### デバッグ

```bash
# コンテナ内でのデバッグ
docker-compose exec tobelog /bin/bash

# ログレベルの変更
docker-compose -f docker-compose.yml -f docker-compose.dev.yml \
  run -e RUST_LOG=debug tobelog
```

## バックアップとリストア

### データベースバックアップ

```bash
# バックアップの作成
docker-compose exec tobelog sqlite3 /home/app/data/blog.db ".backup /home/app/data/backup.db"

# バックアップファイルの取得
docker cp $(docker-compose ps -q tobelog):/home/app/data/backup.db ./backup.db
```

### Dropboxデータ

Dropboxはメインストレージとして使用されるため、Dropboxの機能を使用してバックアップを管理してください。

## 本番環境での推奨事項

1. **リバースプロキシ**
   - Nginx または Traefik の使用
   - SSL/TLS 終端の設定

2. **監視**
   - Prometheus + Grafana
   - ログ集約システム

3. **バックアップ**
   - 定期的なデータベースバックアップ
   - Dropboxデータの監視

4. **セキュリティ**
   - 定期的なセキュリティスキャン
   - アクセストークンの定期的な更新

## 参考資料

- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Dropbox API Documentation](https://www.dropbox.com/developers/documentation/http/overview)