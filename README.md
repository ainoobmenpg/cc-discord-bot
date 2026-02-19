# CC-Discord-Bot

Claude CodeをDiscordから使えるようにするボット。

## 目標

1. ✅ Discordでチャットできる
2. ⏳ スケジュール起動ができる
3. ⏳ エージェントから話しかけてくれる

## 技術スタック

- **Discord Bot**: Rust + Serenity
- **LLM**: GLM-4.7 Flash (Z.ai)
- **HTTP Client**: reqwest

## 使い方

### 1. 環境変数を設定

```bash
export GLM_API_KEY="your-glm-api-key"
export DISCORD_BOT_TOKEN="your-discord-bot-token"
```

### 2. ボットを起動

```bash
cd cc-bot
cargo run
```

または

```bash
./run.sh
```

### 3. Discordで使用

```
!ask こんにちは
!ask 2 + 2は？
!ask Rustの特徴は？
```

## ファイル構造

```
cc-bot/
├── Cargo.toml
├── src/
│   ├── main.rs      # Discord Bot
│   └── glm.rs       # GLM APIクライアント
├── run.sh
└── bot.log
```

## 進捗

- ✅ GLM-4.7 API統合
- ✅ Discord Bot実装
- ✅ `!ask` コマンド
- ⏳ スケジューラー
- ⏳ エージェントからの通知

## トラブルシューティング

### MESSAGE CONTENT INTENTが有効になっていない

[Discord Developer Portal](https://discord.com/developers/applications) で:
1. ボットを選択
2. 「Bot」タブ
3. 「MESSAGE CONTENT INTENT」をONにする

### APIキーのエラー

Z.aiコンソールで残高を確認してください。

## 参考リソース

- [GLM-4.7 ドキュメント](https://docs.z.ai/guides/llm/glm-4.7)
- [Serenity ドキュメント](https://docs.rs/serenity/)
