# cc-discord-bot - Claude Code用Discord Botスキル

Claude CodeをDiscord Botとして機能させるためのスキル開発プロジェクト。

## 目的

Claude CodeからDiscord Bot APIを直接操作できるようにし、以下を実現する：

- Discordでチャットできる
- スケジュール起動ができる
- エージェントから話しかけてくれる

## 調査結果

### 既存のソリューション

#### 1. zebbern/claude-code-discord（GitHub: 93⭐）
- **URL**: https://github.com/zebbern/claude-code-discord
- **概要**: Claude CodeをDiscord bot化するTypeScript製bot
- **技術スタック**: Docker, Deno, TypeScript
- **機能**:
  - `/git`, `/shell`, `/claude` コマンド
  - ブランチ管理
  - ロール制御
  - 45+のコマンド
- **注意**: これは「Claude CodeをDiscordから操作するbot」であって、「Claude Code用スキル」ではない

#### 2. ComposioのDiscordbot MCP
- **URL**: https://composio.dev/toolkits/discordbot/framework/claude-code
- **概要**: MCP（Model Context Protocol）経由でDiscord Bot APIにアクセス
- **機能**:
  - メッセージ送受信
  - チャンネル管理
  - モデレーション
  - ロール管理
  - Webhook管理
  - イベント管理
- **注意**: MCPサーバーなので、Claude CodeからはMCPクライアントとして接続

#### 3. agent-discord-skills（GitHub: Nice-Wolf-Studio）
- **URL**: https://github.com/Nice-Wolf-Studio/agent-discord-skills
- **概要**: Claude Code用のDiscord APIスキルセット
- **技術スタック**: Discord API v10, 環境変数で認証
- **機能**:
  - `discord-send-message`: メッセージ送信（embed対応）
  - `discord-get-messages`: メッセージ取得（ページネーション）
  - `discord-create-channel`: チャンネル作成
  - `discord-list-channels`: チャンネル一覧
  - `discord-manage-channel`: チャンネル管理（更新・削除）
- **スキル構造**:
  ```
  skill-name/
  ├── SKILL.md       # メインのスキル定義
  └── examples.md    # 使用例
  ```
- **これが一番「Claude Code用スキル」として近い**

#### 4. OpenClawのDiscord連携
- **概要**: OpenClaw自体がDiscordに対応
- **機能**:
  - メッセージ送受信
  - チャンネルアクション
  - リアクション
  - スレッド操作
  - ロール管理
  - イベント管理
  - モデレーション
- **CLI**: `openclaw message` コマンド
- **注意**: OpenClaw依存になる

### 実装アプローチの比較

| アプローチ | メリット | デメリット |
|-----------|---------|-----------|
| **OpenClawのmessageツールを使う** | 既存機能を使うだけ、実装が簡単 | OpenClaw依存になる |
| **独自にdiscord.pyを使う** | Claude Code単体で動作、柔軟性が高い | 実装が複雑、bot tokenの管理が必要 |

### 技術的実現可能性

✅ **十分に可能**

理由：
1. OpenClawは既にDiscordに対応している
2. Claude Codeのスキル仕様が明確（SKILL.md + スクリプト）
3. 参考実装が存在する（agent-discord-skills）
4. Python/Node.jsでDiscord botを動かせる

### 次のステップ

1. **agent-discord-skillsの構造を詳しく調査**
2. **OpenClawのmessageツールの仕様を確認**
3. **プロトタイプを作成**
4. **テスト環境を構築**

## 参考リソース

- [Discord Developer Portal](https://discord.com/developers/applications)
- [Discord API Documentation](https://discord.com/developers/docs/reference)
- [Claude Code Skills Documentation](https://docs.claude.com/en/docs/claude-code/skills)
- [OpenClaw Docs](https://docs.openclaw.ai)

## ライセンス

MIT License

## 作者

ainoobmenpg
