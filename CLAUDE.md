# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

DiscordでGLM-4.7と連携し、OpenClaw同等の作業ができるAIアシスタント。Rust + Serenityで実装。

**現在のバージョン**: v0.1.0 (MVP完了)
**目標**: v0.6.0 (メモリシステム)まで段階的に実装

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

# ログ確認
tail -f cc-bot/bot.log
```

**注意**: `cc-bot/` 内で `cargo run` を直接実行しないこと（`.env` が読み込まれない）。

## アーキテクチャ

```
Discord → Serenity (main.rs) → GLM Client (glm.rs) → GLM-4.7 API
```

### 主要ファイル

| ファイル | 役割 |
|----------|------|
| `cc-bot/src/main.rs` | Discord Bot本体、`!ask` コマンド処理 |
| `cc-bot/src/glm.rs` | GLM-4.7 APIクライアント、エラー型定義 |
| `run.sh` | 起動スクリプト（`.env` 読み込み） |
| `.env` | 環境変数（Git管理外） |

### 依存関係

- **serenity 0.12**: Discord API
- **tokio 1**: 非同期ランタイム
- **reqwest 0.12**: HTTPクライアント
- **thiserror 2**: エラーハンドリング
- **tracing**: ロギング

## 環境変数

`.env` ファイルで管理：

```bash
GLM_API_KEY=your-glm-api-key
GLM_MODEL=glm-4.7-flash
DISCORD_BOT_TOKEN=your-discord-token
```

## ロードマップ

| Phase | 機能 | 状態 |
|-------|------|------|
| v0.1.0 | MVP（チャット基本機能） | ✅ 完了 |
| v0.2.0 | セッション管理 | ⏳ 計画中 |
| v0.3.0 | ツール実行機能 | 📋 未着手 |
| v0.4.0 | スケジューラー | 📋 未着手 |
| v0.5.0 | セキュリティ | 📋 未着手 |
| v0.6.0 | メモリシステム | 📋 未着手 |

## Phase別開発ガイド

### v0.2.0 セッション管理

```bash
# 実装開始
/work で「v0.2.0から始めて」と指示

# 追加予定ファイル
cc-bot/src/session.rs    # セッション管理
cc-bot/src/history.rs    # 履歴管理
```

### v0.3.0 ツール実行機能

```bash
# 実装開始
/work で「v0.3.0から始めて」と指示

# 追加予定ファイル
cc-bot/src/tool.rs       # Tool trait
cc-bot/src/tools/        # 個別ツール実装
```

### v0.4.0 スケジューラー

```bash
# 実装開始
/work で「v0.4.0から始めて」と指示

# 追加予定依存関係
tokio-cron-scheduler または cron
```

## Discord Bot設定

Discord Developer Portal で **MESSAGE CONTENT INTENT** を有効にする必要がある。

## ドキュメント構成

| ファイル | 目的 |
|----------|------|
| `Plans.md` | ロードマップとタスク定義 |
| `decisions.md` | アーキテクチャ決定記録（ADR） |
| `patterns.md` | 設計・実装パターン |
| `AGENTS.md` | エージェント連携設定 |
