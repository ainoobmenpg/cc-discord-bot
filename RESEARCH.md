# 調査メモ - cc-discord-bot

## 調査日
2026-02-19

## 調査方法
- Exa AIで「Claude Code Discord bot integration custom skill agent」を検索
- GitHubリポジトリのドキュメントを確認
- OpenClawのローカルドキュメントを確認

## 既存実装の詳細

### agent-discord-skillsのスキル構造

```
skill-name/
├── SKILL.md       # メインのスキル定義（YAML frontmatter + Markdown）
└── examples.md    # 使用例とシナリオ
```

**SKILL.mdの形式**:
- YAML frontmatterでメタデータを定義
- Markdownでスキルの説明
- Claude Codeへの指示を記述

**環境変数**:
```bash
export DISCORD_BOT_TOKEN="your-bot-token-here"
```

**必要なBot権限**:
- Send Messages (2048)
- Read Message History (65536)
- Manage Channels (16)
- View Channels (1024)

### OpenClawのmessageツール

**CLIコマンド**:
```bash
openclaw message <subcommand> [flags]
```

**主なサブコマンド**:
- `send`: メッセージ送信
- `poll`: 投票作成
- `react`: リアクション追加
- `read`: メッセージ読み取り
- `edit`: メッセージ編集
- `delete`: メッセージ削除
- `pin`/`unpin`: ピン留め
- `thread create`/`thread list`/`thread reply`: スレッド操作
- `emoji list`/`emoji upload`: 絵文字管理
- `channel list`/`channel info`: チャンネル情報
- `member info`: メンバー情報
- `event create`/`event list`: イベント管理
- `timeout`/`kick`/`ban`: モデレーション

**ターゲット形式（Discord）**:
- `channel:<id>`: チャンネル指定
- `user:<id>`: ユーザー指定
- `@mention`: メンション形式
- 生のID: チャンネルとして扱う

**使用例**:
```bash
# メッセージ送信
openclaw message send --channel discord \
  --target channel:123 --message "hi" --reply-to 456

# 投票作成
openclaw message poll --channel discord \
  --target channel:123 \
  --poll-question "Snack?" \
  --poll-option Pizza --poll-option Sushi \
  --poll-multi --poll-duration-hours 48

# スレッド作成
openclaw message thread create --channel discord \
  --thread-name "Discussion" --target channel:123 \
  --message "Let's discuss!"
```

## 実装のポイント

### スキル形式（Claude Code）

1. **SKILL.md**:
   - YAML frontmatterでカテゴリ、タグ、説明を定義
   - Claude Codeへの具体的な指示を記述
   - 必要なツールや環境変数を明記

2. **スクリプト**:
   - Python or Node.jsで実装
   - Discord APIを直接叩くか、OpenClawのmessageツールを使う

### 認証方法

1. **直接discord.pyを使う場合**:
   - 環境変数 `DISCORD_BOT_TOKEN` を使用
   - bot tokenを安全に管理

2. **OpenClawのmessageツールを使う場合**:
   - OpenClawの設定ファイルでDiscordを設定済みであること
   - `exec` ツールで `openclaw message` コマンドを実行

### 機能要件

1. **メッセージ送受信**:
   - チャンネルへのメッセージ送信
   - メッセージの読み取り（履歴）

2. **スケジュール起動**:
   - 定期的なタスク実行
   - cron式のスケジュール設定

3. **エージェントからの通知**:
   - 非同期的なメッセージ送信
   - イベントドリブンな通知

## 懸念点

1. **セキュリティ**:
   - bot tokenの管理
   - 権限の最小化
   - 外部スキルの信頼性

2. **依存関係**:
   - OpenClaw依存にするか、スタンドアロンにするか
   - MCPサーバーを併用するか

3. **メンテナンス**:
   - Discord APIの更新対応
   - エラーハンドリング
   - ログ出力

## 次のアクション

1. ✅ リポジトリ作成
2. ✅ 調査結果をドキュメント化
3. ⏳ agent-discord-skillsのソースコードを確認
4. ⏳ プロトタイプの設計
5. ⏳ テスト環境の構築
