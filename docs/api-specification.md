# tobelog API仕様書

## 概要

tobelog APIは、個人ブログシステムの記事管理、メディア管理、システム監視機能を提供するRESTful APIです。

## 認証

API Key認証を使用します。認証が必要なエンドポイントでは、リクエストヘッダーに以下を含めてください：

```
Authorization: Bearer YOUR_API_KEY
```

## ベースURL

- 開発環境: `http://localhost:3000`
- 本番環境: `https://your-domain.com`

## 共通レスポンス形式

### 成功時

```json
{
  "success": true,
  "data": {},
  "message": "操作が正常に完了しました"
}
```

### エラー時

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "エラーメッセージ",
    "details": {}
  }
}
```

## HTTPステータスコード

| ステータス | 説明 |
|-----------|------|
| 200 | 成功 |
| 201 | 作成成功 |
| 400 | リクエストエラー |
| 401 | 認証エラー |
| 403 | 権限不足 |
| 404 | リソースが見つからない |
| 429 | レート制限超過 |
| 500 | サーバーエラー |

---

## エンドポイント一覧

### 1. 記事API

#### GET /api/posts
記事一覧を取得します。

**パラメータ（クエリ）:**
- `page` (int): ページ番号（デフォルト: 1）
- `limit` (int): 1ページあたりの記事数（デフォルト: 10、最大: 100）
- `category` (string): カテゴリ名でフィルタ
- `tag` (string): タグでフィルタ
- `published` (bool): 公開状態でフィルタ
- `search` (string): 検索クエリ（タイトル・本文を検索）

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "posts": [
      {
        "slug": "first-post",
        "title": "初めての投稿",
        "excerpt": "記事の要約...",
        "category": "tech",
        "tags": ["rust", "blog"],
        "published": true,
        "featured": false,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "author": "junichiro",
        "read_time": 5
      }
    ],
    "pagination": {
      "current_page": 1,
      "total_pages": 10,
      "total_posts": 95,
      "per_page": 10
    }
  }
}
```

#### GET /api/posts/{slug}
個別記事を取得します。

**パラメータ（パス）:**
- `slug` (string): 記事のスラッグ

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "slug": "first-post",
    "title": "初めての投稿",
    "content": "記事の本文...",
    "content_html": "<p>記事の本文...</p>",
    "excerpt": "記事の要約...",
    "category": "tech",
    "tags": ["rust", "blog"],
    "published": true,
    "featured": false,
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z",
    "author": "junichiro",
    "read_time": 5,
    "version": 1,
    "dropbox_path": "/posts/2024/01-first-post.md"
  }
}
```

#### POST /api/posts
新規記事を作成します。

**認証:** 必要

**リクエストボディ:**
```json
{
  "title": "新しい記事",
  "content": "記事の本文（Markdown形式）",
  "category": "tech",
  "tags": ["rust", "blog"],
  "published": true,
  "featured": false
}
```

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "slug": "new-article",
    "title": "新しい記事",
    "created_at": "2024-01-01T00:00:00Z"
  },
  "message": "記事が正常に作成されました"
}
```

#### PUT /api/posts/{slug}
記事を更新します。

**認証:** 必要

**パラメータ（パス）:**
- `slug` (string): 記事のスラッグ

**リクエストボディ:**
```json
{
  "title": "更新された記事タイトル",
  "content": "更新された記事の本文",
  "category": "tech",
  "tags": ["rust", "web"],
  "published": true,
  "featured": true
}
```

#### DELETE /api/posts/{slug}
記事を削除します。

**認証:** 必要

**パラメータ（パス）:**
- `slug` (string): 記事のスラッグ

### 2. カテゴリ・タグAPI

#### GET /api/categories
全カテゴリ一覧を取得します。

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "categories": [
      {
        "name": "tech",
        "display_name": "技術",
        "post_count": 15,
        "description": "技術関連の記事"
      }
    ]
  }
}
```

#### GET /api/tags
全タグ一覧を取得します。

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "tags": [
      {
        "name": "rust",
        "post_count": 8
      },
      {
        "name": "blog",
        "post_count": 12
      }
    ]
  }
}
```

### 3. メディアAPI

#### POST /api/media/upload
メディアファイルをアップロードします。

**認証:** 必要

**リクエスト:** multipart/form-data
- `file`: アップロードするファイル
- `alt_text`: 代替テキスト（オプション）

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "filename": "image-20240101-123456.jpg",
    "url": "/media/images/2024/image-20240101-123456.jpg",
    "size": 1024000,
    "mime_type": "image/jpeg",
    "alt_text": "サンプル画像"
  }
}
```

#### GET /api/media
メディアファイル一覧を取得します。

**認証:** 必要

**パラメータ（クエリ）:**
- `page` (int): ページ番号
- `limit` (int): 1ページあたりのファイル数
- `type` (string): ファイルタイプ（image, video, document）

### 4. 統計API

#### GET /api/stats
ブログの統計情報を取得します。

**認証:** 必要

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "posts": {
      "total": 95,
      "published": 87,
      "draft": 8,
      "featured": 5
    },
    "categories": [
      {
        "name": "tech",
        "count": 45
      }
    ],
    "tags": [
      {
        "name": "rust",
        "count": 15
      }
    ],
    "media": {
      "total_files": 120,
      "total_size": 50000000
    }
  }
}
```

### 5. 同期API

#### POST /api/sync/dropbox
Dropboxとデータベースの同期を実行します。

**認証:** 必要

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "synced_posts": 5,
    "new_posts": 2,
    "updated_posts": 1,
    "deleted_posts": 0
  },
  "message": "Dropbox同期が完了しました"
}
```

### 6. システムAPI

#### GET /health
システムのヘルスチェックを行います。

**レスポンス例:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0",
  "services": {
    "database": "connected",
    "dropbox": "connected",
    "cache": "active"
  },
  "uptime": 86400
}
```

#### GET /api/version
API及びアプリケーションのバージョン情報を取得します。

**レスポンス例:**
```json
{
  "success": true,
  "data": {
    "api_version": "1.0.0",
    "app_version": "0.1.0",
    "rust_version": "1.70.0",
    "build_date": "2024-01-01T00:00:00Z"
  }
}
```

---

## エラーコード一覧

| エラーコード | 説明 |
|------------|------|
| INVALID_REQUEST | リクエスト形式が不正です |
| UNAUTHORIZED | 認証が必要です |
| FORBIDDEN | この操作を実行する権限がありません |
| NOT_FOUND | 指定されたリソースが見つかりません |
| VALIDATION_ERROR | 入力値の検証でエラーが発生しました |
| DUPLICATE_SLUG | 同じスラッグの記事が既に存在します |
| DROPBOX_ERROR | Dropbox APIでエラーが発生しました |
| DATABASE_ERROR | データベースエラーが発生しました |
| INTERNAL_ERROR | 内部サーバーエラーが発生しました |

---

## レート制限

APIリクエストには以下のレート制限が適用されます：

- **一般API**: 100リクエスト/分
- **管理API**: 10リクエスト/分
- **アップロードAPI**: 5リクエスト/分

レート制限に達した場合、HTTP 429ステータスとともに以下のレスポンスが返されます：

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "レート制限を超過しました。しばらく時間をおいてから再試行してください。",
    "retry_after": 60
  }
}
```

---

## クライアントライブラリ例

### cURL

```bash
# 記事一覧取得
curl -X GET "http://localhost:3000/api/posts?limit=5"

# 認証が必要な操作
curl -X POST "http://localhost:3000/api/posts" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"title": "新しい記事", "content": "記事の内容"}'
```

### JavaScript (fetch)

```javascript
// 記事一覧取得
const response = await fetch('/api/posts');
const data = await response.json();

// 記事作成
const createResponse = await fetch('/api/posts', {
  method: 'POST',
  headers: {
    'Authorization': 'Bearer YOUR_API_KEY',
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    title: '新しい記事',
    content: '記事の内容'
  })
});
```

---

## 変更履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 1.0.0 | 2024-01-01 | 初期リリース |