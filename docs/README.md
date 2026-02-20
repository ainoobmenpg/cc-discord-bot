# cc-discord-bot ドキュメント

DiscordでGLM-4.7と連携するAIボットの包括的なドキュメントです。

---

## 📚 ドキュメント一覧

### 初めての方

1. **[クイックスタート・環境設定](configuration.md)** - インストールから起動まで
2. **[Slash Commands](slash-commands.md)** - Discordコマンドの使い方

### 機能詳細

3. **[システムアーキテクチャ](architecture.md)** - 全体構成とモジュール
4. **[ツール仕様](tools.md)** - ファイル操作、Web取得、メモリ等
5. **[セッション・メモリ](session-and-memory.md)** - 会話履歴と長期記憶
6. **[スケジューラー・HTTP API](scheduler-and-api.md)** - 定期実行と外部連携
7. **[権限システム](permission-system.md)** - アクセス制御

---

## 🚀 30秒で始める

```bash
# 1. .env 作成
echo "DISCORD_BOT_TOKEN=xxx
GLM_API_KEY=xxx
API_KEY=xxx
ALLOWED_ORIGINS=http://localhost:3000
SUPER_USER_IDS=あなたのDiscordID" > .env

# 2. 起動
./run.sh

# 3. Discordで
/ask こんにちは！
```

---

## 🎯 使用例

### 基本的な質問

```
/ask Pythonでフィボナッチ数列を書いて
```

### ファイル操作

```
/ask srcフォルダ内の全てのTODOコメントを探して
/ask 結果を output/todos.txt に保存して
```

### Web情報取得

```
/ask https://example.com の記事を要約して
```

### 定期実行

```
/schedule add "0 9 * * *" おはよう！今日の予定を確認して
```

### 長期記憶

```
/ask 私の誕生日は3月15日です。覚えておいて
/ask 私の誕生日はいつ？
```

---

## 📊 機能概要

| 機能 | 説明 |
|------|------|
| **GLM-4.7連携** | 高性能LLMとの対話 |
| **セッション管理** | 会話履歴の保持 |
| **ファイル操作** | 読み取り、書き込み、検索 |
| **Web取得** | 記事の要約・抽出 |
| **メモリ** | 長期記憶の保存・検索 |
| **スケジューラー** | 定期実行（Cron） |
| **HTTP API** | 外部からの操作 |
| **権限管理** | ユーザー毎のアクセス制御 |

---

## 🔗 関連リンク

- [Discord Developer Portal](https://discord.com/developers/applications)
- [Zhipu AI (GLM)](https://open.bigmodel.cn/)
- [GitHub Repository](https://github.com/your-repo/cc-discord-bot)
