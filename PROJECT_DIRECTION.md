# プロジェクト方針 - cc-discord-bot

更新日: 2026-02-19

---

## 目標

**OpenClawのように動くClaude Codeを作る**

Claude CodeをDiscord Botとして機能させ、Discordから操作できるようにする。

---

## 実現したい機能

### 1. Discordでチャットできる
- Discordからメッセージを送る → Claude Codeが応答
- Claude Codeがメッセージを送る → Discordに表示
- 双方向のコミュニケーション

### 2. スケジュール起動ができる
- 定期的にClaude Codeにタスクを実行させる
- cron式のスケジュール設定
- 結果をDiscordに通知

### 3. エージェントから話しかけてくれる
- Claude Codeが自発的にメッセージを送ってくる
- 非同期な通知
- イベントドリブンな通知

---

## 技術スタック

### Rust + Serenity

**選択理由**:
1. **型安全性**: AIが書くコードのバグを防ぐ
2. **パフォーマンス**: 高速な実行、メモリ効率が良い
3. **エラーハンドリング**: `Result<T, E>` で明示的なエラー処理
4. **並列処理**: 並列処理が得意

**Discordライブラリ**:
- **Serenity**: 機能豊富なDiscordライブラリ
- **Poise**: Serenityの上に構築されたフレームワーク（スラッシュコマンドが簡単）

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    Discord                              │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│              Rust Discord Bot                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Serenity / Poise                                  │ │
│  │  - メッセージ受信                                   │ │
│  │  - コマンド処理                                     │ │
│  │  - レスポンス送信                                   │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↓                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  HTTP Client                                        │ │
│  │  - Claude Code APIを呼び出し                        │ │
│  │  - ストリーミングレスポンス                          │ │
│  └────────────────────────────────────────────────────┘ │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│          Claude Code API / SDK                         │
│  （Node.js/TypeScriptで実装）                          │
└─────────────────────────────────────────────────────────┘
```

---

## プロジェクト構成

```
cc-discord-bot/
├── rust-bot/              # Rust Discord Bot
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── discord/       # Discordモジュール
│       ├── claude/        # Claude Code APIモジュール
│       └── scheduler/     # スケジューラーモジュール
│
├── claude-api/            # Claude Code API（Node.js）
│   ├── package.json
│   ├── src/
│   │   ├── server.ts      # HTTPサーバー
│   │   └── claude.ts      # Claude Code SDKラッパー
│   └── tsconfig.json
│
├── investigation/         # 調査結果（既存）
│   ├── agent-discord-skills-analysis.md
│   ├── claude-code-discord-analysis.md
│   ├── implementation-patterns.md
│   ├── api-endpoints.md
│   └── architecture-comparison.md
│
├── PROJECT_DIRECTION.md   # このファイル
├── README.md
└── .gitignore
```

---

## 実装のステップ

### Phase 1: Claude Code API（Node.js）

**目的**: Claude Code SDKをHTTP APIとしてラップする

**タスク**:
1. Claude Code SDKの調査
   - 何ができるか確認
   - APIの仕様を理解

2. HTTPサーバーの実装
   - `POST /chat`: メッセージ送信
   - `GET /session`: セッション情報
   - `DELETE /session`: セッション削除
   - `POST /schedule`: スケジュール登録

3. テスト
   - curlで動作確認

**依存関係**:
```json
{
  "dependencies": {
    "@anthropic-ai/claude-agent-sdk": "latest",
    "express": "^4.18.0",
    "typescript": "^5.0.0"
  }
}
```

---

### Phase 2: Rust Discord Bot

**目的**: DiscordからClaude Code APIを呼び出すボットを実装

**タスク**:
1. プロジェクト作成
   ```bash
   cargo new cc-discord-bot --bin
   ```

2. 依存関係追加
   ```toml
   [dependencies]
   serenity = "0.12"
   tokio = { version = "1.0", features = ["full"] }
   reqwest = { version = "0.11", features = ["json"] }
   chrono = "0.4"
   ```

3. 基本実装
   - Discord Botの起動
   - メッセージ受信
   - Claude Code API呼び出し
   - レスポンス送信

4. コマンド実装
   - `/chat`: Claude Codeにチャット
   - `/status`: セッション状態
   - `/schedule`: スケジュール管理

---

### Phase 3: スケジューラー

**目的**: 定期的にClaude Codeを実行してDiscordに通知

**タスク**:
1. cronライブラリ選定
   - **cron**: Rustのcronライブラリ
   - **tokio-cron-scheduler**: Tokioベースのスケジューラー

2. 実装
   - スケジュールの登録
   - 定期実行
   - Discord通知

3. コマンド
   - `/schedule add`: スケジュール追加
   - `/schedule list`: スケジュール一覧
   - `/schedule remove`: スケジュール削除

---

### Phase 4: エージェント通知

**目的**: Claude Codeから自発的に通知を送る

**タスク**:
1. WebSocket実装
   - Claude Code APIからWebSocketで通知を受信
   - Discordに転送

2. イベントハンドリング
   - タスク完了通知
   - エラー通知
   - ステータス更新

---

## Claude Code SDKとの統合

### 問題点
- **Claude Code SDKはNode.js/TypeScriptのみ**
- Rustから直接呼び出すのは難しい

### 解決策
1. **HTTP APIとしてClaude Code SDKをラップ**
   - Node.jsでREST APIサーバーを作る
   - RustからHTTPで呼び出す

2. **WebSocketでストリーミング**
   - Claude CodeのストリーミングレスポンスをWebSocketで転送
   - Rustで受信してDiscordに送信

---

## Claude Code APIの仕様（予定）

### POST /chat

**リクエスト**:
```json
{
  "message": "Hello, Claude!",
  "session_id": "optional-session-id",
  "options": {
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 4096,
    "temperature": 0.7
  }
}
```

**レスポンス**:
```json
{
  "session_id": "session-id",
  "message": "Hi! How can I help you?",
  "tokens_used": 100,
  "model": "claude-3-5-sonnet-20241022"
}
```

### GET /session

**リクエスト**:
```
GET /session?session_id=session-id
```

**レスポンス**:
```json
{
  "session_id": "session-id",
  "created_at": "2026-02-19T12:00:00Z",
  "last_activity": "2026-02-19T12:05:00Z",
  "message_count": 10
}
```

### DELETE /session

**リクエスト**:
```
DELETE /session?session_id=session-id
```

**レスポンス**:
```json
{
  "success": true
}
```

---

## 依存関係

### Rust

```toml
[dependencies]
# Discord
serenity = "0.12"
poise = "0.6"

# 非同期ランタイム
tokio = { version = "1.0", features = ["full"] }

# HTTPクライアント
reqwest = { version = "0.11", features = ["json"] }

# シリアライゼーション
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 日付・時刻
chrono = "0.4"

# スケジューラー
tokio-cron-scheduler = "0.9"

# 環境変数
dotenv = "0.15"

# ロギング
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Node.js

```json
{
  "dependencies": {
    "@anthropic-ai/claude-agent-sdk": "latest",
    "express": "^4.18.0",
    "typescript": "^5.0.0",
    "@types/express": "^4.17.0",
    "cors": "^2.8.5",
    "dotenv": "^16.0.0"
  }
}
```

---

## 環境変数

### Rust Bot

```bash
# Discord
DISCORD_BOT_TOKEN=your_bot_token_here
DISCORD_APPLICATION_ID=your_application_id_here

# Claude Code API
CLAUDE_API_URL=http://localhost:3000
CLAUDE_API_KEY=optional_api_key

# スケジューラー
SCHEDULER_ENABLED=true

# ロギング
RUST_LOG=info
```

### Claude Code API

```bash
# Anthropic
ANTHROPIC_API_KEY=sk-ant-...

# サーバー
PORT=3000
CLAUDE_API_KEY=optional_api_key

# Claude Code
CLAUDE_CODE_DEFAULT_MODEL=claude-3-5-sonnet-20241022
CLAUDE_CODE_MAX_TOKENS=4096
```

---

## 開発環境

### 必要なツール

1. **Rustツールチェーン**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js & npm**
   ```bash
   # macOS
   brew install node

   # またはnvm
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install node
   ```

3. **Discord Developer Account**
   - https://discord.com/developers/applications

---

## テスト計画

### Unit Test
- Rustの各モジュールのテスト
- Node.jsの各関数のテスト

### Integration Test
- Discord Bot ↔ Claude Code APIの通信テスト
- スケジューラーのテスト

### Manual Test
- 実際のDiscordサーバーでテスト
- エラーハンドリングの確認

---

## リリース計画

### v0.1.0 - MVP
- Discordでチャットできる
- 基本的なコマンド

### v0.2.0 - スケジューラー
- スケジュール実行
- cron式のサポート

### v0.3.0 - エージェント通知
- 自発的な通知
- WebSocket対応

---

## 参考リソース

### Rust
- [Rust公式ドキュメント](https://doc.rust-lang.org/)
- [Serenity](https://github.com/serenity-rs/serenity)
- [Poise](https://github.com/kangalioo/poise)

### Claude Code SDK
- [Anthropic SDK](https://github.com/anthropics/anthropic-sdk-typescript)
- [Claude Code Docs](https://docs.claude.com/en/docs/claude-code)

### Discord
- [Discord Developer Portal](https://discord.com/developers/applications)
- [Discord API Documentation](https://discord.com/developers/docs/reference)

---

## 次のステップ

1. **Claude Code SDKの調査**
   - 公式ドキュメントを確認
   - サンプルコードを試す
   - API仕様を理解

2. **Claude Code APIの実装**
   - Node.jsでHTTPサーバー
   - SDKのラッパー

3. **Rust Botのプロトタイプ**
   - シンプルなDiscord Bot
   - 動作確認

---

## ライセンス

MIT License

---

## 作者

ainoobmenpg
