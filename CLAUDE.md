# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

DiscordでGLM-4.7と連携するAIボット。Rust + Serenityで実装。

**現在のバージョン**: v0.7.1 (Slash Commands完了)
**次期バージョン**: v0.8.0 (CLI/HTTP API)

## 開発コマンド

```bash
# ボット起動（推奨 - 環境変数を読み込む）
./run.sh

# ビルド
cd cc-bot && cargo build

# テスト実行
cd cc-bot && cargo test

# リリースビルド
cd cc-bot && cargo build --release
```

**注意**: `cc-bot/` 内で `cargo run` を直接実行しないこと（`.env` が読み込まれない）。

## アーキテクチャ

```
Discord → Serenity (main.rs) → Slash Commands → GLM Client (glm.rs) → GLM-4.7 API
                                      ↓
                              Tool Manager (tool.rs)
                                      ↓
                              Tools: read_file, write_file, list_files, remember, recall
```

### 主要ファイル

| ファイル | 役割 |
|----------|------|
| `cc-bot/src/main.rs` | Discord Bot本体、Slash Commands処理 |
| `cc-bot/src/glm.rs` | GLM-4.7 APIクライアント |
| `cc-bot/src/commands/` | Slash Commands実装 |
| `cc-bot/src/session.rs` | セッション管理（SQLite永続化） |
| `cc-bot/src/scheduler.rs` | Cronベーススケジューラー |
| `cc-bot/src/memory_store.rs` | メモリ永続化（SQLite） |
| `cc-bot/src/tool.rs` | Tool trait定義 |
| `cc-bot/src/tools/` | 個別ツール実装 |

### 依存関係

- **serenity 0.12**: Discord API (Slash Commands対応)
- **tokio 1**: 非同期ランタイム
- **reqwest 0.12**: HTTPクライアント
- **rusqlite 0.32**: SQLiteデータベース
- **uuid 1.21**: UUID生成
- **chrono 0.4**: 日時処理
- **cron 0.15**: スケジュール管理
- **thiserror 2**: エラーハンドリング
- **tracing**: ロギング

## 環境変数

`.env` ファイルで管理：

```bash
GLM_API_KEY=your-glm-api-key
GLM_MODEL=glm-4.7
DISCORD_BOT_TOKEN=your-discord-token
ADMIN_USER_IDS=123,456
```

## 利用可能なSlash Commands

```
/ask <question>                     # GLM-4.7に質問
/clear                              # セッション履歴クリア
/tools                              # ツール一覧
/schedule add/list/remove           # スケジュール管理
/permission list/grant/revoke       # パーミッション
/memory add/list/search/delete      # メモリ操作
/admin status/reload                # 管理者コマンド
```

## 完了済みバージョン

| バージョン | 機能 |
|-----------|------|
| v0.1.0 | MVP (GLM連携) |
| v0.2.0 | セッション管理 |
| v0.3.0 | ツール実行 |
| v0.4.0 | スケジューラー |
| v0.5.0 | セキュリティ |
| v0.6.0 | メモリシステム |
| v0.7.0 | Slash Commands基盤 |
| v0.7.1 | Slash Commands実装完了 |
| v0.9.0 | Windows対応 |

## 次期機能（v0.8.0）

- HTTP API基盤（axum）
- CLIツール

## Discord Bot設定

Discord Developer Portal で以下を有効にする：
- **MESSAGE CONTENT INTENT**
- **Slash Commands** (自動登録)

## ドキュメント構成

| ファイル | 目的 |
|----------|------|
| `Plans.md` | ロードマップとタスク定義 |
| `CLAUDE.md` | 本ファイル（開発ガイド） |
| `README.md` | ユーザー向け説明 |
