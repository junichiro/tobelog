# 個人ブログシステム仕様書

## プロジェクト概要
Dropboxをメインストレージとして活用する、Rust製の個人ブログシステムを構築する。

## システム要件

### 基本要件
- **利用者**: 個人（執筆者1名）
- **月間記事数**: 10-30記事
- **記事形式**: Markdown
- **ホスティング**: 自宅サーバー
- **ストレージ**: Dropbox（Essential契約済み）
- **データベース**: SQLite（ローカル）またはSupabase（必要に応じて）

### 機能要件

#### 必須機能
1. Markdown形式での記事管理
2. LLMで生成した記事の入稿対応
3. Obsidian/DropboxのMarkdownファイル取り込み
4. レスポンシブデザイン
5. カスタムデザイン対応
6. Dropboxをメインストレージとして使用
7. 画像・メディアファイルのDropbox管理

#### オプション機能
1. カテゴリ・タグによる記事分類
2. バージョン管理（記事の変更履歴）

#### 不要な機能
- コメント機能
- 予約投稿
- 複数ユーザー管理
- 高度なSEO対策

## 技術スタック

### バックエンド
- **言語**: Rust
- **Webフレームワーク**: Axum
- **Markdown処理**: pulldown-cmark
- **テンプレートエンジン**: Tera
- **Dropbox連携**: HTTP APIを直接実装

### フロントエンド
- **HTML/CSS/JavaScript**（シンプルな構成）
- **CSSフレームワーク**: TailwindCSS（CDN版で開始）

### データ管理
- **メタデータ**: SQLite（ローカルファイル）
- **記事本体**: Dropbox上のMarkdownファイル
- **メディア**: Dropbox上の画像・動画ファイル

## Dropboxフォルダ構造

```
/BlogStorage/
├── /posts/                    # 公開記事
│   ├── /2024/
│   │   ├── 01-first-post.md
│   │   └── meta.json         # 年ごとのメタデータ
│   └── /2025/
│       └── 01-new-year-post.md
├── /media/                    # メディアファイル
│   ├── /images/
│   │   ├── /2024/
│   │   └── /2025/
│   └── /videos/
├── /drafts/                   # 下書き
│   └── draft-post.md
├── /templates/                # デザインテンプレート
│   ├── style.css
│   └── custom-components.css
└── /config/                   # 設定ファイル
    └── blog-config.json
```

## APIエンドポイント設計

```
GET    /                       # ホームページ（記事一覧）
GET    /posts/{year}/{slug}    # 個別記事表示
GET    /category/{category}    # カテゴリ別記事一覧
GET    /tag/{tag}             # タグ別記事一覧

GET    /api/posts             # 記事一覧API
GET    /api/posts/{slug}      # 個別記事API
POST   /api/posts             # 記事作成
PUT    /api/posts/{slug}      # 記事更新
DELETE /api/posts/{slug}      # 記事削除

POST   /api/sync/dropbox      # Dropbox同期
POST   /api/import/markdown   # Markdown一括インポート

GET    /admin                 # 管理画面（簡易版）
GET    /admin/new             # 新規記事作成
GET    /admin/edit/{slug}     # 記事編集
```

## 記事メタデータ構造

```json
{
  "slug": "first-post",
  "title": "初めての投稿",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z",
  "category": "tech",
  "tags": ["rust", "blog"],
  "published": true,
  "dropbox_path": "/posts/2024/01-first-post.md",
  "media": [
    "/media/images/2024/example.jpg"
  ],
  "version": 1
}
```

## 実装フェーズ

### Phase 1: 基本機能（MVP）
1. Dropbox API連携の実装
2. Markdownファイルの読み込み・パース
3. 記事一覧・個別記事表示
4. 基本的なレスポンシブデザイン

### Phase 2: 記事管理機能
1. 記事の作成・更新・削除API
2. 簡易管理画面
3. LLM生成記事の入稿機能
4. メディアファイル管理

### Phase 3: 拡張機能
1. カテゴリ・タグ機能
2. バージョン管理
3. カスタムデザイン強化
4. パフォーマンス最適化

## 開発環境セットアップ

### 必要な環境変数
```env
DROPBOX_ACCESS_TOKEN=your_token_here
DATABASE_URL=sqlite://blog.db
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
BLOG_TITLE=My Personal Blog
```

### Dropbox App設定
1. Dropbox App Consoleで新規アプリ作成
2. Permission: files.content.read, files.content.write
3. Access Token取得（長期トークンまたはリフレッシュトークン）

## デプロイ構成

### Docker構成
```dockerfile
FROM rust:1.70 as builder
# ... ビルド処理

FROM debian:bookworm-slim
# ... 実行環境
```

### systemdサービス設定
```ini
[Unit]
Description=Personal Blog System
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/blog-server
Environment="DATABASE_URL=sqlite:///var/lib/blog/blog.db"
Restart=always

[Install]
WantedBy=multi-user.target
```

## 注意事項
1. Dropbox APIのレート制限に注意（1アプリあたり500リクエスト/分）
2. 記事キャッシュの実装を検討（頻繁なDropboxアクセスを避ける）
3. バックアップはDropboxの機能に依存
4. SSL証明書はLet's Encryptで取得

## 参考リンク
- [Dropbox API Documentation](https://www.dropbox.com/developers/documentation/http/overview)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)

## プロジェクト構造案

```
blog-system/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── post.rs
│   │   └── metadata.rs
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── posts.rs
│   │   ├── admin.rs
│   │   └── api.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── dropbox.rs
│   │   ├── markdown.rs
│   │   └── cache.rs
│   └── templates/
│       ├── base.html
│       ├── index.html
│       ├── post.html
│       └── admin/
├── static/
│   ├── css/
│   └── js/
├── migrations/
├── Dockerfile
├── docker-compose.yml
└── README.md
```

このシステムは、シンプルさと拡張性のバランスを重視し、個人ブログとして必要十分な機能を提供します。Dropboxの信頼性とRustの高パフォーマンスを活かし、運用コストを最小限に抑えながら、快適なブログ執筆環境を実現します。

