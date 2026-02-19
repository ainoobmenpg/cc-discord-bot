# アーキテクチャ比較

## 概要

agent-discord-skillsとclaude-code-discordのアーキテクチャを比較する。

---

## 1. 目的の違い

### agent-discord-skills
- **目的**: Claude Code用のDiscord APIスキルセット
- **対象**: Claude Codeユーザー
- **使用方法**: Claude Codeから直接呼び出す
- **機能**: Discord APIの基本的な操作（メッセージ、チャンネル）

### claude-code-discord
- **目的**: DiscordからClaude Codeを操作するボット
- **対象**: Discordユーザー
- **使用方法**: Discordのスラッシュコマンドで操作
- **機能**: Claude Codeの完全な操作（Git、Shell、設定、AIエージェント）

---

## 2. 技術スタック

### agent-discord-skills
- **言語**: なし（ドキュメントのみ）
- **実装**: ユーザーが自分で実装（curl、Python、Node.jsなど）
- **依存関係**: なし（Discord APIのみ）

### claude-code-discord
- **言語**: TypeScript
- **ランタイム**: Deno
- **ライブラリ**:
  - discord.js（Discord API）
  - @anthropic-ai/claude-agent-sdk（Claude Code SDK）
  - その他多数
- **依存関係**: 多数のnpmパッケージ

---

## 3. アーキテクチャ

### agent-discord-skills

```
┌─────────────────────────────────────────────────────────┐
│                    Claude Code                          │
│                                                         │
│  ┌────────────────────────────────────────────────────┐ │
│  │              discord-send-message                  │ │
│  │  ┌──────────────────────────────────────────────┐ │ │
│  │  │  SKILL.md + examples.md                      │ │ │
│  │  │  - 使用タイミング                              │ │ │
│  │  │  - APIリクエスト（curl）                      │ │ │
│  │  │  - エラーハンドリング                          │ │ │
│  │  └──────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↓                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Discord API                           │ │
│  │  POST /channels/{id}/messages                     │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**特徴**:
1. ドキュメント主導（SKILL.md + examples.md）
2. Claude Codeがcurlを実行
3. シンプルで理解しやすい
4. ユーザーが実装を自由に選べる

### claude-code-discord

```
┌─────────────────────────────────────────────────────────┐
│                    Discord                              │
│                                                         │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Discord Bot                           │ │
│  │  ┌──────────────────────────────────────────────┐ │ │
│  │  │  Bot Factory                                 │ │ │
│  │  │  - ProcessManager                            │ │ │
│  │  │  - WorktreeManager                           │ │ │
│  │  │  - CrashHandler                              │ │ │
│  │  └──────────────────────────────────────────────┘ │ │
│  │                          ↓                           │ │
│  │  ┌──────────────────────────────────────────────┐ │ │
│  │  │  Handler Registry                            │ │ │
│  │  │  - Command Handlers                          │ │ │
│  │  │  - Button Handlers                           │ │ │
│  │  │  - RBAC                                      │ │ │
│  │  └──────────────────────────────────────────────┘ │ │
│  │                          ↓                           │ │
│  │  ┌──────────────────────────────────────────────┐ │ │
│  │  │  Claude Code Integration                     │ │ │
│  │  │  - Enhanced Client                           │ │ │
│  │  │  - Session Manager                           │ │ │
│  │  │  - MCP Server Manager                        │ │ │
│  │  │  - Model Discovery                           │ │ │
│  │  └──────────────────────────────────────────────┘ │ │
│  │                          ↓                           │ │
│  │  ┌──────────────────────────────────────────────┐ │ │
│  │  │  Git / Shell Integration                     │ │ │
│  │  │  - Shell Manager                             │ │ │
│  │  │  - Git Manager                               │ │ │
│  │  │  - Worktree Manager                          │ │ │
│  │  └──────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↓                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Claude Code SDK                      │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**特徴**:
1. モジュラー設計（core、discord、claude、git、shell）
2. インベントハンドラー（コマンド、ボタン、モーダル）
3. セッション管理
4. RBAC
5. MCPサーバー統合
6. エージェントシステム

---

## 4. 認証・認可

### agent-discord-skills

**認証**:
- ボットトークンのみ（`DISCORD_BOT_TOKEN`）
- 環境変数で管理

**認可**:
- なし
- ボットの権限で操作

**セキュリティ**:
- トークン保護（環境変数のみ）
- ユーザー入力のバリデーション

### claude-code-discord

**認証**:
- ボットトークン（`DISCORD_TOKEN`）
- アプリケーションID（`APPLICATION_ID`）
- オプション: Anthropic APIキー（`ANTHROPIC_API_KEY`）

**認可**:
- RBAC（ロールベースアクセス制御）
- `ADMIN_ROLE_IDS`: 管理者ロール
- `ADMIN_USER_IDS`: 管理者ユーザー
- 破壊的コマンド（`/shell`、`/git`）へのアクセス制御

**セキュリティ**:
- トークン保護（環境変数のみ）
- RBAC
- サンドボックス（ネットワークルール、ファイルシステムACL）
- 監査ログ（チャンネル履歴）

---

## 5. メッセージフロー

### agent-discord-skills

```
ユーザー → Claude Code → curl → Discord API
                    ↓
              SKILL.mdの指示に従う
                    ↓
              エラーハンドリング
                    ↓
              ユーザーフィードバック
```

**特徴**:
1. ユーザーがClaude Codeに直接指示
2. Claude CodeがSKILL.mdを読んで実行
3. シンプルなフロー

### claude-code-discord

```
Discordユーザー → スラッシュコマンド
                    ↓
            Discord Bot
                    ↓
            Bot Factory（マネージャー作成）
                    ↓
            Handler Registry（ハンドラー選択）
                    ↓
            RBACチェック（管理者権限）
                    ↓
            Claude Code Integration
                    ↓
            Claude Code SDK
                    ↓
            Git / Shell Integration
                    ↓
            結果をDiscordに送信
                    ↓
            ボタンインタラクション（確認、権限）
```

**特徴**:
1. Discordから操作
2. 複数のマネージャーが連携
3. RBACでアクセス制御
4. ボタンでインタラクティブ操作

---

## 6. エラーハンドリング

### agent-discord-skills

**アプローチ**:
- ドキュメントベース
- SKILL.mdに一般的なエラーと対処法を記載
- examples.mdに具体的なエラーシナリオ

**例**:
```
403 Forbidden
Bot needs "Send Messages" permission in channel
Check channel permission overrides
```

**特徴**:
1. 静的（ドキュメント）
2. ユーザーが対処
3. 汎用的

### claude-code-discord

**アプローチ**:
- コードベース
- try-catchでエラーをキャッチ
- ユーザーフレンドリーなエラーメッセージ
- Discordインタラクションでエラーを通知

**例**:
```typescript
try {
  await commandHandler(interaction);
} catch (error) {
  await interaction.reply({
    content: `Error: ${getUserFriendlyErrorMessage(error)}`,
    ephemeral: true
  });
}
```

**特徴**:
1. 動的（コード）
2. ボットが対処
3. 具体的

---

## 7. 機能比較

| 機能 | agent-discord-skills | claude-code-discord |
|------|---------------------|---------------------|
| メッセージ送信 | ✅ | ✅ |
| メッセージ取得 | ✅ | ✅ |
| チャンネル作成 | ✅ | ✅ |
| チャンネル一覧 | ✅ | ✅ |
| チャンネル管理 | ✅ | ✅ |
| Git操作 | ❌ | ✅ |
| Shell実行 | ❌ | ✅ |
| 設定管理 | ❌ | ✅ |
| RBAC | ❌ | ✅ |
| MCPサーバー | ❌ | ✅ |
| AIエージェント | ❌ | ✅（7種類） |
| 思考モード | ❌ | ✅（4種類） |
| セッション管理 | ❌ | ✅ |
| スケジュール実行 | ❌ | ❌ |
| Webhook | ❌ | ❌ |

---

## 8. 拡張性

### agent-discord-skills

**拡張方法**:
1. 新しいスキルディレクトリを作成
2. SKILL.mdとexamples.mdを追加
3. Claude Codeが自動検出

**例**:
```
discord-webhook/
├── SKILL.md
└── examples.md
```

**特徴**:
1. ドキュメントを追加するだけ
2. 実装はユーザー次第
3. 柔軟性が高い

### claude-code-discord

**拡張方法**:
1. 新しいコマンドハンドラーを追加
2. Handler Registryに登録
3. 必要に応じてRBACを追加

**例**:
```typescript
// 新しいコマンドハンドラー
async function handleNewCommand(interaction: CommandInteraction) {
  // 実装
}

// 登録
commandHandlers['new-command'] = handleNewCommand;
```

**特徴**:
1. コードを追加する必要がある
2. モジュラー設計で拡張しやすい
3. 型安全性がある

---

## 9. メンテナンス

### agent-discord-skills

**メンテナンス項目**:
- Discord APIの更新対応
- エラーメッセージの更新
- ドキュメントの修正

**難易度**: 低
- ドキュメントの修正のみ
- コードの変更なし

### claude-code-discord

**メンテナンス項目**:
- Discord APIの更新対応
- discord.jsの更新
- Claude Code SDKの更新
- 依存パッケージの更新
- バグ修正
- 機能追加

**難易度**: 中〜高
- コードの変更が必要
- テストが必要
- デプロイが必要

---

## 10. デプロイ

### agent-discord-skills

**デプロイ方法**:
- なし（ユーザー環境にインストールのみ）

**手順**:
```bash
# Claude Codeのスキルディレクトリに配置
mkdir -p ~/.claude/skills
cp -r agent-discord-skills ~/.claude/skills/

# 環境変数を設定
export DISCORD_BOT_TOKEN="your-bot-token"
```

**特徴**:
1. シンプル
2. ユーザー環境に依存
3. サーバー不要

### claude-code-discord

**デプロイ方法**:
- Docker
- Deno
- PM2
- Systemd

**手順（Docker）**:
```bash
# Dockerイメージをビルド
docker build -t claude-code-discord .

# Docker Composeで起動
docker-compose up -d
```

**特徴**:
1. 複雑
2. サーバー必要
3. 永続的な稼働

---

## 11. ユースケース

### agent-discord-skills

**適したユースケース**:
- Claude CodeからDiscordに通知を送りたい
- Claude CodeからDiscordの情報を取得したい
- シンプルなDiscord操作で十分
- 自分で実装をコントロールしたい

**例**:
- 「Claude、この結果をDiscordに送って」
- 「Claude、Discordの最新のメッセージを取得して」
- 「Claude、新しいチャンネルを作って」

### claude-code-discord

**適したユースケース**:
- DiscordからClaude Codeを操作したい
- チームでClaude Codeを使いたい
- 高度な機能（Git、Shell、RBAC）が必要
- サーバーで常時稼働させたい

**例**:
- Discordで`/claude`コマンドを実行
- DiscordでGit操作
- DiscordでShell実行
- チーム_collaboration

---

## 12. 学習コスト

### agent-discord-skills

**学習コスト**: 低
- SKILL.mdの形式を学ぶだけ
- examples.mdを見て理解
- 実装は既存の知識でOK

**時間**: 1〜2時間

### claude-code-discord

**学習コスト**: 中〜高
- TypeScriptの知識が必要
- Discord APIの知識が必要
- Claude Code SDKの知識が必要
- アーキテクチャを理解する必要がある

**時間**: 数日〜数週間

---

## まとめ

### agent-discord-skillsのメリット
1. シンプルで理解しやすい
2. 学習コストが低い
3. 実装を自由に選べる
4. メンテナンスが簡単

### agent-discord-skillsのデメリット
1. 機能が限定的
2. 実装が必要
3. 常時稼働しない

### claude-code-discordのメリット
1. 機能が豊富
2. 常時稼働
3. チーム_collaborationが可能
4. 高度な機能（RBAC、MCP、エージェント）

### claude-code-discordのデメリット
1. 複雑で理解しにくい
2. 学習コストが高い
3. メンテナンスが大変
4. サーバーが必要

---

## 選択ガイド

### agent-discord-skillsを選ぶべき場合
- Claude CodeからDiscordを操作したい
- シンプルな機能で十分
- 自分で実装をコントロールしたい
- 学習コストを抑えたい

### claude-code-discordを選ぶべき場合
- DiscordからClaude Codeを操作したい
- 豊富な機能が必要
- チームで使いたい
- 常時稼働させたい

---

## cc-discord-botの方向性

sakuchichi さんのリポジトリ（cc-discord-bot）は、agent-discord-skillsのアプローチを取るべき理由：

1. **Claude Code用スキル**: Claude CodeからDiscordを操作
2. **シンプル**: 学習コストが低い
3. **柔軟性**: 実装を自由に選べる
4. **メンテナンス**: ドキュメントの修正のみ

最初はagent-discord-skillsの形式で始めて、必要に応じてclaude-code-discordの機能を取り入れていくのが良い。

---

## 次のステップ

1. **agent-discord-skillsの形式を採用**: SKILL.md + examples.md
2. **基本的なスキルを実装**: メッセージ送信・取得、チャンネル管理
3. **スケジュール機能を追加**: cron式のスケジュール
4. **エージェントからの通知を追加**: 非同期的なメッセージ送信
5. **必要に応じて高度な機能を追加**: RBAC、セッション管理など
