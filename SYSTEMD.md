# Systemd Service Configuration and Operation Manual

## 概要

tobelogアプリケーションのsystemdサービス化により、自宅サーバーでの安定した運用を実現します。この文書では、インストール、設定、運用、トラブルシューティングの手順を説明します。

## 前提条件

### システム要件
- Linux システム（systemd使用）
- SQLite 3.x
- curl（ヘルスチェック用）
- bc（監視スクリプト用）
- mail（アラート送信用、オプション）

### ユーザー権限
- インストール時: root権限が必要
- 運用時: tobelogユーザーで実行

## インストール手順

### 1. バイナリのビルド

```bash
# プロジェクトディレクトリで
cargo build --release

# バイナリを適切な場所にコピー（インストールスクリプト実行前に必要）
sudo cp target/release/tobelog /usr/local/bin/tobelog
sudo chmod +x /usr/local/bin/tobelog
```

### 2. systemdサービスのインストール

```bash
# インストールスクリプトの実行（バイナリが/usr/local/bin/tobelogにある場合）
sudo ./scripts/install-systemd.sh

# カスタムバイナリパスを指定する場合
sudo ./scripts/install-systemd.sh --binary-path /opt/tobelog/bin/tobelog

# 注意: バイナリが/usr/local/bin/tobelog以外の場所にある場合は--binary-pathオプションを使用してください
```

### 3. 環境設定

```bash
# 環境ファイルの編集
sudo nano /etc/tobelog/environment

# 必須項目の設定
DROPBOX_ACCESS_TOKEN=your_dropbox_access_token_here
API_KEY=your_api_key_here
```

### 4. サービス開始

```bash
# サービス開始
sudo systemctl start tobelog

# 自動起動有効化
sudo systemctl enable tobelog

# 状態確認
sudo systemctl status tobelog
```

## ファイルシステム構成

```
/usr/local/bin/
└── tobelog                    # 実行ファイル

/etc/tobelog/
├── environment               # 環境変数設定
└── config.toml              # 設定ファイル（オプション）

/var/lib/tobelog/
├── database/
│   └── blog.db              # SQLiteデータベース
└── cache/                   # キャッシュファイル

/var/log/tobelog/
├── tobelog.log              # アプリケーションログ
└── archive/                 # ログアーカイブ

/var/cache/tobelog/
└── temp/                    # 一時ファイル

/etc/systemd/system/
├── tobelog.service          # メインサービス
├── tobelog-monitor.service  # 監視サービス
├── tobelog-monitor.timer    # 監視タイマー
├── tobelog-backup.service   # バックアップサービス
└── tobelog-backup.timer     # バックアップタイマー
```

## サービス管理

### 基本操作

```bash
# サービス管理スクリプトの使用
./scripts/manage-service.sh [COMMAND]

# 利用可能なコマンド
./scripts/manage-service.sh start      # サービス開始
./scripts/manage-service.sh stop       # サービス停止
./scripts/manage-service.sh restart    # サービス再起動
./scripts/manage-service.sh status     # 状態確認
./scripts/manage-service.sh logs       # ログ表示
./scripts/manage-service.sh enable     # 自動起動有効化
./scripts/manage-service.sh disable    # 自動起動無効化
```

### 直接のsystemctl操作

```bash
# サービス操作
sudo systemctl start tobelog
sudo systemctl stop tobelog
sudo systemctl restart tobelog
sudo systemctl reload tobelog

# 状態確認
sudo systemctl status tobelog
sudo systemctl is-active tobelog
sudo systemctl is-enabled tobelog

# ログ確認
sudo journalctl -u tobelog
sudo journalctl -u tobelog -f          # フォロー
sudo journalctl -u tobelog --since "1 hour ago"
```

## 設定管理

### 環境変数

```bash
# /etc/tobelog/environment
DROPBOX_ACCESS_TOKEN=your_token
API_KEY=your_api_key
RUST_LOG=info
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
DATABASE_URL=sqlite:///var/lib/tobelog/database/blog.db
```

### サービス設定の変更

```bash
# サービスファイルの編集
sudo systemctl edit tobelog

# 設定変更後の再読み込み
sudo systemctl daemon-reload
sudo systemctl restart tobelog
```

## ログ管理

### ログ確認

```bash
# systemdジャーナル
sudo journalctl -u tobelog

# ファイルログ
sudo tail -f /var/log/tobelog/tobelog.log

# ログレベル変更
sudo systemctl edit tobelog
# [Service]
# Environment="RUST_LOG=debug"
```

### ログローテーション

```bash
# 手動でログローテーション実行
sudo logrotate -f /etc/logrotate.d/tobelog

# 設定確認
sudo logrotate -d /etc/logrotate.d/tobelog
```

## バックアップ・復旧

### 自動バックアップ

```bash
# バックアップタイマーの確認
sudo systemctl status tobelog-backup.timer

# 手動バックアップ実行
./scripts/backup.sh --compress --retention 30

# カスタムバックアップ
./scripts/backup.sh --destination /home/backup --compress
```

### 復旧手順

```bash
# 1. サービス停止
sudo systemctl stop tobelog

# 2. データ復旧（バックアップディレクトリから復旧）
# バックアップファイルの場所を確認
ls -la /var/backups/tobelog/
# 特定のタイムスタンプのバックアップから復旧
sudo cp /var/backups/tobelog/YYYYMMDD_HHMMSS/database/blog.db /var/lib/tobelog/database/
sudo chown tobelog:tobelog /var/lib/tobelog/database/blog.db

# 3. サービス開始
sudo systemctl start tobelog

# 4. 動作確認
sudo systemctl status tobelog
curl -f http://localhost:3000/health
```

## 監視・アラート

### 監視スクリプト

```bash
# 手動監視実行
./scripts/monitor.sh

# アラート有効化
./scripts/monitor.sh --alerts --email admin@example.com

# 閾値カスタマイズ
./scripts/monitor.sh --threshold-cpu 90 --threshold-disk 95
```

### 自動監視

```bash
# 監視タイマーの確認
sudo systemctl status tobelog-monitor.timer

# 監視ログの確認
sudo journalctl -u tobelog-monitor
```

### 監視項目

- **サービス状態**: 起動状態、自動起動設定
- **システムリソース**: CPU、メモリ、ディスク使用量
- **ヘルスエンドポイント**: HTTP応答性、レスポンス時間
- **ログ分析**: エラー発生率、ログファイルサイズ
- **データベース**: 整合性、アクセス性、サイズ
- **Dropbox API**: 接続性、エラー率
- **ファイル権限**: ディレクトリ・ファイルの権限設定

## セキュリティ設定

### システムハードニング

```bash
# サービスユーザーの権限確認
id tobelog

# ファイル権限の確認
sudo ls -la /etc/tobelog/
sudo ls -la /var/lib/tobelog/

# systemdセキュリティ設定の確認
sudo systemctl show tobelog | grep -E "(NoNewPrivileges|ProtectSystem|PrivateTmp)"
```

### ファイアウォール設定

```bash
# ufwを使用する場合
sudo ufw allow 3000/tcp comment "Tobelog HTTP"

# iptablesを使用する場合
sudo iptables -A INPUT -p tcp --dport 3000 -j ACCEPT
sudo iptables-save > /etc/iptables/rules.v4
```

## アップデート手順

### アプリケーションアップデート

```bash
# 1. 新しいバイナリのビルド
cargo build --release

# 2. サービス停止
sudo systemctl stop tobelog

# 3. バックアップ作成
./scripts/backup.sh --compress

# 4. バイナリ更新
sudo cp target/release/tobelog /usr/local/bin/tobelog

# 5. 権限設定
sudo chmod +x /usr/local/bin/tobelog

# 6. サービス開始
sudo systemctl start tobelog

# 7. 動作確認
sudo systemctl status tobelog
curl -f http://localhost:3000/health
```

### systemd設定アップデート

```bash
# 1. 設定ファイル更新
sudo cp systemd/tobelog.service /etc/systemd/system/

# 2. systemd再読み込み
sudo systemctl daemon-reload

# 3. サービス再起動
sudo systemctl restart tobelog
```

## トラブルシューティング

### サービス起動失敗

```bash
# 詳細なエラー情報確認
sudo systemctl status tobelog -l
sudo journalctl -u tobelog --since "10 minutes ago"

# 設定ファイル検証
sudo systemd-analyze verify /etc/systemd/system/tobelog.service

# 環境変数確認
sudo systemctl show tobelog | grep Environment
```

### 一般的な問題と解決策

#### 1. 環境変数未設定

```bash
# 症状: "environment variable not found" エラー
# 解決: /etc/tobelog/environment ファイルの設定確認
sudo nano /etc/tobelog/environment
```

#### 2. データベース接続エラー

```bash
# 症状: "unable to open database file" エラー
# 解決: ファイル権限とパスの確認
sudo ls -la /var/lib/tobelog/database/
sudo chown -R tobelog:tobelog /var/lib/tobelog/
```

#### 3. ポート競合

```bash
# 症状: "address already in use" エラー
# 解決: ポート使用状況確認
sudo ss -tlnp | grep :3000
sudo fuser -k 3000/tcp  # プロセス強制終了
```

#### 4. Dropbox API接続エラー

```bash
# 症状: Dropbox API関連エラー
# 解決: ネットワーク接続とAPIトークン確認
curl -H "Authorization: Bearer YOUR_TOKEN" https://api.dropboxapi.com/2/users/get_current_account
```

### ログレベル変更によるデバッグ

```bash
# デバッグモード有効化
sudo systemctl edit tobelog
# [Service]
# Environment="RUST_LOG=debug"

sudo systemctl restart tobelog
sudo journalctl -u tobelog -f
```

## パフォーマンス調整

### リソース制限

```bash
# CPU制限
sudo systemctl edit tobelog
# [Service]
# CPUQuota=50%

# メモリ制限
# MemoryLimit=512M

sudo systemctl daemon-reload
sudo systemctl restart tobelog
```

### データベース最適化

```bash
# データベースVACUUM実行
sudo -u tobelog sqlite3 /var/lib/tobelog/database/blog.db "VACUUM;"

# データベース統計更新
sudo -u tobelog sqlite3 /var/lib/tobelog/database/blog.db "ANALYZE;"
```

## 高可用性設定

### 自動再起動設定

```bash
# サービスファイルの設定確認
sudo systemctl show tobelog | grep -E "(Restart|StartLimit)"

# 設定変更例
sudo systemctl edit tobelog
# [Service]
# Restart=always
# RestartSec=30
# StartLimitBurst=5
```

### ヘルスチェック監視

```bash
# 外部監視ツールの設定例
# /etc/cron.d/tobelog-health
# */5 * * * * root curl -f http://localhost:3000/health || systemctl restart tobelog
```

## 参考情報

### 有用なコマンド

```bash
# システム情報
sudo systemctl --version
sudo systemctl list-units | grep tobelog
sudo systemctl list-timers | grep tobelog

# リソース使用量
sudo systemctl status tobelog | grep -E "(Memory|CPU)"
sudo ps aux | grep tobelog

# ネットワーク
sudo ss -tlnp | grep tobelog
sudo netstat -tlnp | grep :3000
```

### 関連ファイル

- **サービスファイル**: `/etc/systemd/system/tobelog.service`
- **環境設定**: `/etc/tobelog/environment`
- **ログ設定**: `/etc/logrotate.d/tobelog`
- **rsyslog設定**: `/etc/rsyslog.d/49-tobelog.conf`

### 外部リンク

- [systemd.service man page](https://www.freedesktop.org/software/systemd/man/systemd.service.html)
- [systemd.timer man page](https://www.freedesktop.org/software/systemd/man/systemd.timer.html)
- [SQLite Documentation](https://www.sqlite.org/docs.html)
- [Dropbox API Documentation](https://www.dropbox.com/developers/documentation/http/overview)