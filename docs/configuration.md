# 環境設定・クイックスタート

## クイックスタート

### 1. 環境変数設定

`.env` ファイルを作成：

```bash
# 必須
DISCORD_BOT_TOKEN=your-discord-bot-token
GLM_API_KEY=your-glm-api-key
API_KEY=your-secure-api-key
ALLOWED_ORIGINS=http://localhost:3000

# 推奨（自分をスーパーユーザーに）
SUPER_USER_IDS=あなたのDiscordID
```

### 2. 起動

```bash
./run.sh
```

### 3. Discordで使用

```
/ask こんにちは！
```

---

## 環境変数一覧

### 必須

| 変数 | 説明 | 例 |
|------|------|-----|
| `DISCORD_BOT_TOKEN` | Discordボットトークン | `OTIxODI3...` |
| `GLM_API_KEY` | GLM-4.7 APIキー | `zhipuai-...` |
| `API_KEY` | HTTP API認証キー | `my-secret-key-123` |
| `ALLOWED_ORIGINS` | CORS許可オリジン | `http://localhost:3000,https://example.com` |

### オプション

| 変数 | デフォルト | 説明 |
|------|-----------|------|
| `GLM_MODEL` | `glm-4.7` | GLMモデル名 |
| `ADMIN_USER_IDS` | - | 管理者ユーザーID（カンマ区切り） |
| `SUPER_USER_IDS` | - | スーパーユーザーID（カンマ区切り） |
| `API_PORT` | `3000` | HTTP APIポート |
| `BASE_OUTPUT_DIR` | `/tmp/cc-bot` | ファイル出力先 |
| `MCP_CONFIG_PATH` | - | MCP設定ファイルパス |

---

## Discord設定

### ボット作成

1. [Discord Developer Portal](https://discord.com/developers/applications) にアクセス
2. 「New Application」をクリック
3. ボット名を入力して作成
4. 「Bot」タブ →「Add Bot」
5. トークンをコピー（`DISCORD_BOT_TOKEN`に設定）

### Intents設定

「Bot」タブで以下を有効化：
- ✅ **MESSAGE CONTENT INTENT**

### サーバーに招待

「OAuth2」タブ →「URL Generator」：
- **Scopes**: `bot`, `applications.commands`
- **Permissions**: `Send Messages`, `Use Slash Commands`

生成されたURLでサーバーに招待。

---

## GLM API設定

### APIキー取得

1. [Zhipu AI](https://open.bigmodel.cn/) にアクセス
2. アカウント作成・ログイン
3. API キーを取得

### モデル

| モデル | 説明 |
|--------|------|
| `glm-4.7` | デフォルト（推奨） |
| `glm-4` | 標準モデル |

---

## ファイル構成

```
cc-discord-bot/
├── .env                    # 環境変数（Git管理外）
├── run.sh                  # 起動スクリプト
├── cc-bot/
│   ├── Cargo.toml         # Rust設定
│   ├── src/               # ソースコード
│   └── data/              # データベース等
│       ├── sessions.db    # SQLite DB
│       └── permissions.json
├── docs/                   # ドキュメント
└── output/                 # ファイル出力先
    └── YYYY-MM-DD/
        └── user_{id}/
```

---

## トラブルシューティング

### ボットが応答しない

1. `DISCORD_BOT_TOKEN` が正しいか確認
2. MESSAGE CONTENT INTENT が有効か確認
3. ログを確認：
   ```bash
   ./run.sh 2>&1 | grep -i error
   ```

### HTTP API にアクセスできない

1. `API_KEY` が設定されているか確認
2. ポートが使用可能か確認：
   ```bash
   lsof -i :3000
   ```

### 権限エラー

1. `SUPER_USER_IDS` に自分のDiscordIDを設定
2. `/permission list` で現在の権限を確認

### ファイル操作エラー

1. `BASE_OUTPUT_DIR` が書き込み可能か確認
2. ディレクトリを作成：
   ```bash
   mkdir -p /tmp/cc-bot
   ```

---

## ログレベル調整

```bash
# デバッグログ有効
RUST_LOG=debug ./run.sh

# 特定モジュールのみ
RUST_LOG=cc_bot::api=debug ./run.sh
```

---

## 本番環境設定例

```bash
# .env（本番）
DISCORD_BOT_TOKEN=xxx
GLM_API_KEY=xxx
GLM_MODEL=glm-4.7

# セキュリティ
API_KEY=$(openssl rand -hex 32)
SUPER_USER_IDS=123456789
ALLOWED_ORIGINS=https://your-domain.com

# パス設定
BASE_OUTPUT_DIR=/var/lib/cc-bot/output
MCP_CONFIG_PATH=/etc/cc-bot/mcp-config.json
```
