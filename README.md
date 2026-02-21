# CC-Discord-Bot

Discordで使えるAIボット。GLM-4.7と連携して、チャットやスケジュールタスクを実行します。

## 主な機能

- **チャット**: `/ask` コマンドでGLM-4.7に質問 ✅
- **セッション管理**: チャット履歴を保持 ✅
- **ツール実行**: ファイル読み書き、メモリ機能 ✅
- **スケジュール実行**: 定期的にタスクを実行 ✅
- **メモリシステム**: 情報を記憶・検索 ✅

---

## クイックスタート

### 1. 環境変数を設定

`.env.example` をコピーして `.env` を作成:

```bash
cp .env.example .env
```

`.env` を編集して認証情報を設定:

```bash
# GLM API Configuration
GLM_API_KEY=your-glm-api-key-here
GLM_MODEL=glm-4.7

# Discord Bot Configuration
DISCORD_BOT_TOKEN=your-discord-bot-token-here

# Admin User IDs (optional, comma-separated)
ADMIN_USER_IDS=123456789,987654321
```

### 2. ボットを起動

**macOS/Linux:**
```bash
./run.sh
```

**Windows (CMD):**
```cmd
run.bat
```

**Windows (PowerShell):**
```powershell
.\run.ps1
```

**注意**: `cc-bot/` ディレクトリ内で `cargo run` を実行しないでください（環境変数が読み込まれません）。

### 3. Discordで使用

ボットをサーバーに招待した後、Slash Commandsを使用：

```
/ask こんにちは
/ask 2 + 2は？
/tools
/schedule list
/memory list
```

---

## 利用可能なコマンド

| コマンド | 説明 |
|----------|------|
| `/ask <質問>` | GLM-4.7に質問 |
| `/clear` | セッション履歴をクリア |
| `/tools` | 利用可能なツール一覧 |
| `/schedule add/list/remove` | スケジュール管理 |
| `/memory add/list/search/delete` | メモリ操作 |
| `/permission list/grant/revoke` | パーミッション管理 |
| `/admin status/reload` | 管理者コマンド |

---

## 技術スタック

- **言語**: Rust
- **Discord Bot**: Serenity 0.12 (Slash Commands)
- **LLM**: GLM-4.7 (Z.ai)
- **データベース**: SQLite (rusqlite)
- **スケジューラー**: cron
- **非同期ランタイム**: tokio 1.x

---

## ファイル構造

```
cc-discord-bot/
├── .env                 # 環境変数（Git管理外）
├── .env.example         # 環境変数テンプレート
├── run.sh               # 起動スクリプト (macOS/Linux)
├── run.bat              # 起動スクリプト (Windows CMD)
├── run.ps1              # 起動スクリプト (Windows PowerShell)
├── README.md            # このファイル
├── CLAUDE.md            # 開発ガイド
├── Plans.md             # ロードマップ
│
└── cc-bot/              # Rust Discord Bot
    ├── Cargo.toml       # 依存関係
    └── src/
        ├── main.rs      # Bot本体
        ├── glm.rs       # GLM APIクライアント
        ├── session.rs   # セッション管理
        ├── scheduler.rs # スケジューラー
        ├── memory_store.rs # メモリ永続化
        ├── tool.rs      # Tool trait
        ├── commands/    # Slash Commands
        └── tools/       # ツール実装
```

---

## 進捗

| バージョン | 機能 | 状態 |
|-----------|------|------|
| v0.1.0 | MVP (GLM連携) | ✅ |
| v0.2.0 | セッション管理 | ✅ |
| v0.3.0 | ツール実行 | ✅ |
| v0.4.0 | スケジューラー | ✅ |
| v0.5.0 | セキュリティ | ✅ |
| v0.6.0 | メモリシステム | ✅ |
| v0.7.1 | Slash Commands | ✅ |
| v0.8.0 | HTTP API | ✅ |
| v0.9.0 | Windows対応 | ✅ |

---

## トラブルシューティング

### ボットが応答しない

1. Discord Developer Portal で **MESSAGE CONTENT INTENT** が有効か確認
2. ボットがサーバーに招待されているか確認
3. トークンが正しいか確認

### 認証エラー

Discord Developer Portal でトークンを再生成してください。

### ボットが起動しない

1. `.env` ファイルが存在するか確認
2. `run.sh` に実行権限があるか確認: `chmod +x run.sh`
3. Rustがインストールされているか確認

---

## 開発

### テスト実行

```bash
cd cc-bot && cargo test
```

### ビルド

```bash
cd cc-bot && cargo build --release
```

---

## セキュリティ

- ✅ `.env` ファイルで機密情報を管理
- ✅ `.gitignore` で `.env` を除外
- ✅ `.env.example` でテンプレートを提供
- ⚠️ 決して `.env` ファイルをコミットしないでください

---

## 参考リソース

- [GLM-4.7 ドキュメント](https://docs.z.ai/guides/llm/glm-4)
- [Serenity ドキュメント](https://docs.rs/serenity/)
- [Discord Developer Portal](https://discord.com/developers/applications)

---

## ライセンス

MIT License

---

## 作者

ainoobmenpg
