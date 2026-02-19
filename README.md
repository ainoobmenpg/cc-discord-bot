# CC-Discord-Bot

Claude CodeのようにDiscordで使えるAIボット。

**現在の実装**: Rust + Serenity + GLM-4.7 Flash

## 目標

1. ✅ Discordでチャットできる
2. ⏳ スケジュール起動ができる（未実装）
3. ⏳ エージェントから話しかけてくれる（未実装）

## 技術スタック

- **Discord Bot**: Rust + Serenity
- **LLM**: GLM-4.7 Flash (Z.ai)
- **HTTP Client**: reqwest
- **非同期ランタイム**: tokio

## セットアップ

### 1. 環境変数を設定

`.env.example` をコピーして `.env` を作成:

```bash
cp .env.example .env
```

`.env` を編集して認証情報を設定:

```bash
# GLM API Configuration
GLM_API_KEY=your-glm-api-key-here
GLM_MODEL=glm-4.7-flash

# Discord Bot Configuration
DISCORD_BOT_TOKEN=your-discord-bot-token-here
```

### 2. ボットを起動

```bash
./run.sh
```

**注意**: `cc-bot/` ディレクトリ内で `cargo run` を実行しないでください（環境変数が読み込まれません）。

### 3. Discordで使用

```
!ask こんにちは
!ask 2 + 2は？
!ask Rustの特徴は？
```

## ファイル構造

```
cc-discord-bot/
├── .env                 # 環境変数（Git管理外・セキュリティ）
├── .env.example         # 環境変数のテンプレート（Git管理）
├── run.sh               # ボット起動スクリプト
├── cc-bot/
│   ├── Cargo.toml       # Rust依存関係
│   └── src/
│       ├── main.rs      # Discord Bot本体
│       └── glm.rs       # GLM APIクライアント
└── README.md
```

## 進捗

- ✅ GLM-4.7 API統合
- ✅ Discord Bot実装
- ✅ `!ask` コマンド
- ✅ エラーハンドリングとロギング
- ✅ セキュリティ対策（.env方式）
- ✅ コードレビュー完了
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

### ボットが起動しない

1. `.env` ファイルが存在するか確認
2. `run.sh` に実行権限があるか確認: `chmod +x run.sh`
3. ログを確認: `tail -f bot.log`

### コンパイルエラー

```bash
cd cc-bot
cargo clean
cargo build
```

## 開発

### 依存関係の追加

```bash
cd cc-bot
cargo add <crate-name>
```

### テスト

```bash
cd cc-bot
cargo test
```

### ログの確認

```bash
tail -f cc-bot/bot.log
```

## アーキテクチャの変更履歴

### 2026-02-19: Rust + GLM API に変更

**変更前の計画**:
- Rust + Node.js (Claude Code API)
- Claude Agent SDKを使用

**変更後の実装**:
- Rustのみ
- GLM-4.7 APIを直接使用

**理由**:
- Claude Agent SDKにAnthropic APIキーが必要
- GLM-4.7 Flash（無料版）を既に使用可能
- シンプルなアーキテクチャを優先

## セキュリティ

- ✅ `.env` ファイルで機密情報を管理
- ✅ `.gitignore` で `.env` を除外
- ✅ `.env.example` でテンプレートを提供
- ⚠️ 決して `.env` ファイルをコミットしないでください

## 参考リソース

- [GLM-4.7 ドキュメント](https://docs.z.ai/guides/llm/glm-4)
- [Serenity ドキュメント](https://docs.rs/serenity/)
- [Discord Developer Portal](https://discord.com/developers/applications)

## ライセンス

MIT License

## 作者

ainoobmenpg
