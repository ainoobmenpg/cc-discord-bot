# システムアーキテクチャ

## 概要

cc-discord-bot は、Discord上でGLM-4.7（LLM）と連携するAIボットです。Rust + Serenityで実装されています。

---

## 全体構成

```
┌─────────────────────────────────────────────────────────────────┐
│                         Discord                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Serenity Gateway                             │
│                   (Discord API Client)                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       main.rs                                    │
│                    (EventHandler)                                │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐      │
│  │   Handler   │ RateLimiter │ Permission  │  Scheduler  │      │
│  │             │             │  Manager    │             │      │
│  └─────────────┴─────────────┴─────────────┴─────────────┘      │
└─────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ Slash Commands  │ │   HTTP API      │ │   LLM Client    │
│   (/ask等)      │ │   (axum)        │ │   (GLM-4.7)     │
└─────────────────┘ └─────────────────┘ └─────────────────┘
          │                                       │
          └───────────────────┬───────────────────┘
                              ▼
                    ┌─────────────────┐
                    │  Tool Manager   │
                    │    (Tools)      │
                    └─────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
    ┌───────────┐      ┌───────────┐      ┌───────────┐
    │File Tools │      │ Web Tool  │      │Memory Tool│
    │read/write │      │ web_fetch │      │remember   │
    └───────────┘      └───────────┘      └───────────┘
```

---

## モジュール一覧

### コアモジュール

| ファイル | 役割 |
|----------|------|
| `main.rs` | エントリーポイント、Discordイベント処理 |
| `glm.rs` | レガシーGLMクライアント（互換性維持） |
| `api.rs` | HTTP APIサーバー（axum） |

### LLM関連

| ファイル | 役割 |
|----------|------|
| `llm/mod.rs` | LLMクライアントtrait定義 |
| `llm/glm.rs` | GLM-4.7 APIクライアント実装 |
| `llm/mock.rs` | テスト用モッククライアント |

### 機能モジュール

| ファイル | 役割 |
|----------|------|
| `session.rs` | セッション管理（会話履歴） |
| `scheduler.rs` | Cronベースのスケジューラー |
| `memory_store.rs` | メモリ永続化（SQLite） |
| `permission.rs` | 権限管理システム |
| `rate_limiter.rs` | レートリミッター（DoS防止） |

### ツール（Tools）

| ファイル | 役割 |
|----------|------|
| `tools/read_file.rs` | ファイル読み取り |
| `tools/write_file.rs` | ファイル書き込み |
| `tools/edit.rs` | ファイル部分編集 |
| `tools/list_files.rs` | ファイル一覧 |
| `tools/glob.rs` | パターンマッチファイル検索 |
| `tools/grep.rs` | ファイル内容検索 |
| `tools/bash.rs` | シェルコマンド実行 |
| `tools/web_fetch.rs` | Webコンテンツ取得 |
| `tools/remember.rs` | メモリ保存 |
| `tools/mcp.rs` | MCPツール統合 |

### コマンド（Slash Commands）

| ファイル | 役割 |
|----------|------|
| `commands/ask.rs` | `/ask` - GLM-4.7に質問 |
| `commands/clear.rs` | `/clear` - セッション履歴クリア |
| `commands/tools.rs` | `/tools` - ツール一覧表示 |
| `commands/schedule.rs` | `/schedule` - スケジュール管理 |
| `commands/permission.rs` | `/permission` - 権限管理 |
| `commands/memory_cmd.rs` | `/memory` - メモリ操作 |
| `commands/admin.rs` | `/admin` - 管理者コマンド |
| `commands/settings.rs` | `/settings` - ユーザー設定 |

### セキュリティ

| ファイル | 役割 |
|----------|------|
| `security/logging.rs` | ログマスキング（機密情報保護） |
| `validation.rs` | 入力バリデーション（XSS/パストラバーサル対策） |

---

## データフロー

### `/ask` コマンドの処理フロー

```
1. ユーザーがDiscordで /ask と入力
        ↓
2. SerenityがInteractionイベントを受信
        ↓
3. main.rs:handle_slash_command() で処理
        ↓
4. レートリミットチェック
        ↓
5. commands/ask.rs:run() を実行
        ↓
6. セッションから履歴を取得
        ↓
7. LLM Client (GLM-4.7) にリクエスト送信
        ↓
8. LLMがツール実行を要求した場合
   ├─ Tool Manager がツールを実行
   └─ 結果をLLMに返して処理継続
        ↓
9. 最終応答をDiscordに送信
        ↓
10. セッション履歴を更新・永続化
```

---

## 永続化

### データベース（SQLite）

| ファイル | 内容 |
|----------|------|
| `data/sessions.db` | セッション履歴、メモリ、スケジュール |

### JSONファイル

| ファイル | 内容 |
|----------|------|
| `data/permissions.json` | 権限設定 |
| `data/role_config.json` | ロール設定 |
| `data/user_settings/` | ユーザー毎設定 |

### 出力ディレクトリ

| ディレクトリ | 内容 |
|-------------|------|
| `output/YYYY-MM-DD/user_{id}/` | ユーザー毎のファイル出力 |

---

## 依存関係

### 主要クレート

| クレート | 用途 |
|----------|------|
| `serenity` 0.12 | Discord API |
| `tokio` 1 | 非同期ランタイム |
| `axum` 0.8 | HTTP API |
| `reqwest` 0.12 | HTTPクライアント |
| `rusqlite` 0.32 | SQLiteデータベース |
| `serde` / `serde_json` | JSONシリアライズ |
| `tracing` | ロギング |
| `cron` 0.15 | スケジュール管理 |
| `subtle` 2 | 定数時間比較（セキュリティ） |
| `regex` 1 | 正規表現 |
| `legible` 0.4 | Web本文抽出 |

---

## 環境変数

| 変数 | 必須 | 説明 |
|------|:----:|------|
| `DISCORD_BOT_TOKEN` | ✅ | Discordボットトークン |
| `GLM_API_KEY` | ✅ | GLM-4.7 APIキー |
| `GLM_MODEL` | | モデル名（デフォルト: glm-4.7） |
| `ADMIN_USER_IDS` | | 管理者ユーザーID（カンマ区切り） |
| `SUPER_USER_IDS` | | スーパーユーザーID（カンマ区切り） |
| `API_KEY` | ✅ | HTTP API認証キー |
| `API_PORT` | | HTTP APIポート（デフォルト: 3000） |
| `ALLOWED_ORIGINS` | ✅ | CORS許可オリジン |
| `BASE_OUTPUT_DIR` | | 出力ディレクトリ（デフォルト: /tmp/cc-bot） |
| `MCP_CONFIG_PATH` | | MCP設定ファイルパス |
