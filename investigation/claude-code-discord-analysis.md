# claude-code-discord 詳細分析

## 概要

claude-code-discord は、Deno + discord.js を使った Discord ボットで、`@anthropic-ai/claude-agent-sdk` を使用して Claude Code を Discord から操作できるようにするものです。ユーザーは Discord チャットから Claude Code に対話的に指示を出し、コードの生成、デバッグ、レビュー、Git 操作などを行うことができます。

**技術スタック**:
- Deno 2.2.0
- discord.js 14.14.1
- @anthropic-ai/claude-agent-sdk 0.2.45

---

## プロジェクト構造

```
claude-code-discord/
├── index.ts                    # メインエントリーポイント
├── deno.json                   # Deno 設定（依存関係、タスク定義）
├── .env                        # 環境変数（非コミット）
├── .claude/mcp.json            # MCP サーバー設定
│
├── claude/                     # Claude Agent SDK 統合レイヤー
│   ├── client.ts               # SDK クエリ実行、ストリーミング
│   ├── enhanced-client.ts      # モデル管理、セッションマネージャー
│   ├── command.ts              # /claude コマンドハンドラー
│   ├── message-converter.ts    # SDK JSON → Discord メッセージ変換
│   ├── model-fetcher.ts        # 動的モデル取得（API + CLI）
│   ├── query-manager.ts        # クエリライフサイクル管理
│   ├── user-question.ts        # AskUserQuestion ツール処理
│   ├── permission-request.ts   # インタラクティブ許可要求
│   ├── hooks.ts                # SDK フック統合
│   └── types.ts                # 型定義
│
├── core/                       # コアボットインフラ
│   ├── handler-registry.ts     # コマンドルーティング、オプションビルダー
│   ├── bot-factory.ts          # マネージャー生成（Shell, Git, Health 等）
│   ├── rbac.ts                 # ロールベースアクセス制御
│   ├── config-loader.ts        # 環境変数・CLI引数の解析
│   ├── signal-handler.ts       # グレースフルシャットダウン
│   ├── command-wrappers.ts     # コマンドラッパー
│   └── button-handlers.ts      # ボタンインタラクション
│
├── discord/                    # Discord 統合レイヤー
│   ├── bot.ts                  # Discord Bot 作成、コマンド登録
│   ├── sender.ts               # メッセージ送信、Embeds、ストリーミング
│   ├── pagination.ts           # ページネーション処理
│   ├── formatting.ts           # テキストフォーマット
│   ├── types.ts                # 型定義
│   └── utils.ts                # ユーティリティ
│
├── settings/                   # 設定管理
│   ├── unified-settings.ts     # 統合設定（UI 定義、永続化）
│   ├── unified-handlers.ts     # 設定コマンドハンドラー
│   ├── advanced-settings.ts    # 高度な設定
│   └── handlers.ts             # 設定ハンドラー
│
├── git/                        # Git 統合
│   ├── handler.ts              # Git コマンド実行
│   ├── command.ts              # Git スラッシュコマンド定義
│   ├── process-manager.ts      # Worktree Bot 管理
│   ├── types.ts                # 型定義
│   └── index.ts                # エクスポート
│
├── shell/                      # シェル実行
│   ├── handler.ts              # シェルコマンド実行
│   ├── command.ts              # シェルコマンド定義
│   ├── types.ts                # 型定義
│   └── index.ts                # エクスポート
│
├── agent/                      # エージェント管理
│   ├── index.ts                # カスタムエージェント定義
│   └── command.ts              # エージェントコマンド
│
├── process/                    # プロセス管理
│   ├── crash-handler.ts        # クラッシュハンドリング
│   ├── health-monitor.ts       # ヘルスモニタリング
│   └── index.ts                # エクスポート
│
├── screenshot/                 # スクリーンショット
│   ├── handler.ts              # スクリーンショット取得
│   ├── command.ts              # コマンド定義
│   └── types.ts                # 型定義
│
├── system/                     # システムコマンド
│   ├── commands.ts             # システムコマンド
│   └── index.ts                # エクスポート
│
├── help/                       # ヘルプ
│   ├── commands.ts             # ヘルプコマンド
│   └── index.ts                # エクスポート
│
├── util/                       # ユーティリティ
│   ├── version-check.ts        # バージョンチェック
│   ├── usage-tracker.ts        # API 使用量追跡
│   ├── persistence.ts          # 設定永続化
│   ├── process.ts              # プロセスユーティリティ
│   └── platform.ts             # プラットフォーム検出
│
├── types/                      # 共有型定義
│   ├── shared.ts               # 共通インターフェース
│   └── index.ts                # エクスポート
│
└── tests/                      # テストファイル
    ├── test-unified-settings.ts
    └── ...
```

---

## アーキテクチャ

### モジュラー設計

このプロジェクトは**モジュラー設計**を採用しており、各機能が独立したモジュールとして実装されています。

```typescript
// index.ts - エントリーポイントでの依存性注入パターン
export async function createClaudeCodeBot(config: BotConfig) {
  // マネージャー生成（bot-factory パターン）
  const managers: BotManagers = createBotManagers({
    config: { discordToken, applicationId, workDir, categoryName, userId },
    crashHandlerOptions: { maxRetries: 3, retryDelay: 5000, ... }
  });

  const { shellManager, worktreeBotManager, crashHandler,
          healthMonitor, claudeSessionManager } = managers;

  // ハンドラー生成（handler-registry パターン）
  const allHandlers: AllHandlers = createAllHandlers(deps, claudeSession, settings);

  // Discord Bot 作成
  bot = await createDiscordBot(config, handlers, buttonHandlers, dependencies);
}
```

### ハンドラーパターン

各コマンドは「定義（Command）」と「実装（Handler）」に分離されています：

```typescript
// コマンド定義（discord/command.ts）
export const claudeCommands: SlashCommandBuilder[] = [
  new SlashCommandBuilder()
    .setName('claude')
    .setDescription('Send a prompt to Claude Code')
    .addStringOption(opt => opt.setName('prompt').setRequired(true))
    .addStringOption(opt => opt.setName('session_id'))
];

// ハンドラー実装（claude/command.ts）
export function createClaudeHandlers(deps: ClaudeHandlerDeps) {
  return {
    async execute(ctx: InteractionContext) {
      const prompt = ctx.getString('prompt', true);
      const sessionId = ctx.getString('session_id');
      // Claude SDK クエリ実行...
    }
  };
}
```

### SDK統合

Claude Agent SDK を使用したストリーミング処理：

```typescript
// claude/client.ts
import { query as claudeQuery, type SDKMessage } from "@anthropic-ai/claude-agent-sdk";

const iterator = claudeQuery(queryOptions);

for await (const message of iterator) {
  if (controller.signal.aborted) break;

  // メッセージ処理...
  if (message.type === 'assistant' && message.message.content) {
    const textContent = message.message.content
      .filter(c => c.type === 'text')
      .map(c => c.text)
      .join('');
    if (onChunk) onChunk(textContent);
  }
}
```

### データフロー

```
Discord メッセージ (/claude "Hello")
    │
    ▼
discord/bot.ts (handleCommand)
    │  ├─ RBAC チェック
    │  └─ ハンドラー選択
    │
    ▼
core/handler-registry.ts (createAllHandlers)
    │  └─ getQueryOptions() で現在の設定を反映
    │
    ▼
claude/client.ts (sendToClaudeCode)
    │  ├─ MCP サーバー読み込み
    │  ├─ SDK query() 呼び出し
    │  └─ ストリーミング処理
    │
    ▼
claude-agent-sdk (AsyncGenerator)
    │  └─ SDKMessage を yield
    │
    ▼
claude/message-converter.ts
    │  └─ SDK JSON → ClaudeMessage 変換
    │
    ▼
discord/sender.ts
    │  └─ Discord Embed 更新
    │
    ▼
Discord チャンネル（リアルタイム更新）
```

---

## Discord API統合

### discord.js 14.14.1

discord.js を使用して Discord Gateway に接続し、スラッシュコマンドとボタンインタラクションを処理します。

```typescript
// discord/bot.ts
import {
  Client,
  GatewayIntentBits,
  Events,
  ChannelType,
  REST,
  Routes,
  CommandInteraction,
  ButtonInteraction,
  EmbedBuilder
} from "discord.js";

const client = new Client({
  intents: [GatewayIntentBits.Guilds],
});
```

### GatewayIntentBits

このボットは最小限の Intent で動作します：

| Intent | 用途 |
|--------|------|
| `Guilds` | サーバー/チャンネル情報の取得 |

**注記**: メッセージ内容の読み取りには `MessageContent` Intent は不要です（スラッシュコマンドのみ使用するため）。

### スラッシュコマンド登録

Discord REST API を使用してグローバルコマンドを登録します：

```typescript
// discord/bot.ts
const rest = new REST({ version: '10' }).setToken(discordToken);

await rest.put(
  Routes.applicationCommands(applicationId),
  { body: commands.map(cmd => cmd.toJSON()) }
);
```

### チャンネル自動作成

ボット起動時に、指定されたカテゴリとブランチ名に基づいてチャンネルを自動作成します：

```typescript
// discord/bot.ts
async function ensureChannelExists(guild): Promise<TextChannel> {
  const channelName = sanitizeChannelName(branchName);

  // カテゴリ確認/作成
  let category = guild.channels.cache.find(
    c => c.type === ChannelType.GuildCategory && c.name === actualCategoryName
  );
  if (!category) {
    category = await guild.channels.create({
      name: actualCategoryName,
      type: ChannelType.GuildCategory,
    });
  }

  // チャンネル確認/作成
  let channel = guild.channels.cache.find(
    c => c.type === ChannelType.GuildText &&
        c.name === channelName &&
        c.parentId === category.id
  );
  if (!channel) {
    channel = await guild.channels.create({
      name: channelName,
      type: ChannelType.GuildText,
      parent: category.id,
      topic: `Repository: ${repoName} | Branch: ${branchName}`,
    });
  }

  return channel;
}
```

---

## 認証・認可

### RBAC（ロールベースアクセス制御）

危険なコマンドに対してロールベースのアクセス制御を実装しています。

```typescript
// core/rbac.ts

// 制限対象コマンドの定義
const RESTRICTED_COMMANDS: Record<string, string[]> = {
  /** Full host access — highest risk */
  shell: ['shell', 'shell-input', 'shell-list', 'shell-kill'],
  /** Repository modifications */
  git: ['git', 'worktree', 'worktree-remove', 'worktree-bots', 'worktree-kill'],
  /** System information exposure */
  system: ['env-vars', 'port-scan', 'system-logs'],
  /** Bot lifecycle */
  admin: ['shutdown'],
};

// 環境変数からロール/ユーザー ID を読み込み
export function loadRBACConfig(): RBACConfig {
  const roleIdsRaw = Deno.env.get("ADMIN_ROLE_IDS") ?? "";
  const userIdsRaw = Deno.env.get("ADMIN_USER_IDS") ?? "";

  const allowedRoleIds = new Set(roleIdsRaw.split(",").map(id => id.trim()).filter(Boolean));
  const allowedUserIds = new Set(userIdsRaw.split(",").map(id => id.trim()).filter(Boolean));

  return { enabled: allowedRoleIds.size > 0 || allowedUserIds.size > 0,
           allowedRoleIds, allowedUserIds };
}

// 権限チェック
export async function checkCommandPermission(
  commandName: string,
  ctx: InteractionContext
): Promise<boolean> {
  if (!isRestrictedCommand(commandName)) return true;
  if (hasPermission(ctx)) return true;

  await ctx.reply({
    content: "🔒 **Access Denied** — You don't have permission.",
    ephemeral: true
  });

  return false;
}
```

### ADMIN_ROLE_IDS / ADMIN_USER_IDS

| 環境変数 | 説明 | 例 |
|----------|------|-----|
| `ADMIN_ROLE_IDS` | 管理者ロール ID（カンマ区切り） | `123456789,987654321` |
| `ADMIN_USER_IDS` | 管理者ユーザー ID（カンマ区切り） | `111222333` |

---

## Claude Agent SDK統合

### query() 関数

Claude Agent SDK の `query()` 関数を使用して Claude Code と対話します。

```typescript
// claude/client.ts
import { query as claudeQuery, type SDKMessage } from "@anthropic-ai/claude-agent-sdk";

export async function sendToClaudeCode(
  workDir: string,
  prompt: string,
  controller: AbortController,
  sessionId?: string,
  onChunk?: (text: string) => void,
  modelOptions?: ClaudeModelOptions
): Promise<{ response: string; sessionId?: string; cost?: number }> {

  const queryOptions = {
    prompt,
    abortController: controller,
    options: {
      cwd: workDir,
      permissionMode: modelOptions?.permissionMode || "dontAsk",
      systemPrompt: { type: 'preset', preset: 'claude_code' },
      settingSources: ['project', 'local'],

      // モデル設定
      ...(modelOptions?.model && { model: modelOptions.model }),
      ...(modelOptions?.thinking && { thinking: modelOptions.thinking }),
      ...(modelOptions?.effort && { effort: modelOptions.effort }),
      ...(modelOptions?.maxBudgetUsd && { maxBudgetUsd: modelOptions.maxBudgetUsd }),

      // セッション管理
      ...(continueMode && { continue: true }),
      ...(cleanedSessionId && !continueMode && { resume: cleanedSessionId }),

      // MCP サーバー
      ...(mcpServers && { mcpServers }),

      // ツール使用許可コールバック
      canUseTool: async (toolName: string, input: Record<string, unknown>) => {
        // AskUserQuestion の処理...
        // MCP ツールの自動許可...
        // インタラクティブ許可要求...
      },
    },
  };

  const iterator = claudeQuery(queryOptions);

  for await (const message of iterator) {
    if (controller.signal.aborted) break;

    // メッセージ処理...
    if (message.type === 'assistant' && message.message.content) {
      const textContent = message.message.content
        .filter(c => c.type === 'text')
        .map(c => c.text)
        .join('');
      if (onChunk) onChunk(textContent);
    }

    // セッション ID の保存
    if ('session_id' in message && message.session_id) {
      currentSessionId = message.session_id;
    }
  }

  return { response: fullResponse, sessionId: currentSessionId };
}
```

### MCP サーバー統合

`.claude/mcp.json` から MCP サーバー設定を動的に読み込みます：

```typescript
// claude/client.ts
async function loadMcpServers(workDir: string): Promise<Record<string, McpServerConfig> | undefined> {
  try {
    const mcpPath = path.join(workDir, ".claude", "mcp.json");
    const raw = await Deno.readTextFile(mcpPath);
    const parsed = JSON.parse(raw);
    const servers = parsed?.mcpServers;

    const result: Record<string, McpServerConfig> = {};
    for (const [name, cfg] of Object.entries(servers)) {
      const raw = cfg as any;
      // ${workspaceFolder:-.} プレースホルダーを解決
      const args = Array.isArray(raw.args)
        ? raw.args.map((a: string) => a.replace(/\$\{workspaceFolder:-\.?\}/g, workDir))
        : undefined;
      result[name] = {
        type: "stdio",
        command: raw.command,
        ...(args && { args }),
        ...(raw.env && { env: raw.env }),
      };
    }
    console.log(`[MCP] Loaded ${Object.keys(result).length} MCP server(s)`);
    return result;
  } catch {
    return undefined;
  }
}
```

### AskUserQuestion ハンドラー

Claude がユーザーに質問する必要がある場合、Discord ボタンで対話的に回答を収集します：

```typescript
// index.ts
function createAskUserDiscordHandler(bot: any): (input: AskUserQuestionInput) => Promise<Record<string, string>> {
  return async (input: AskUserQuestionInput): Promise<Record<string, string>> => {
    const channel = bot.getChannel();
    const answers: Record<string, string> = {};

    for (const q of input.questions) {
      // Embed で質問を表示
      const embed = new EmbedBuilder()
        .setColor(0xff9900)
        .setTitle(`❓ Claude needs your input — ${q.header}`)
        .setDescription(q.question)
        .setFooter({ text: 'Click an option to answer' });

      // 各オプションにボタンを作成
      const row = new ActionRowBuilder();
      for (const opt of q.options) {
        row.addComponents(
          new ButtonBuilder()
            .setCustomId(`ask-user:${qi}:${oi}`)
            .setLabel(opt.label)
            .setStyle(ButtonStyle.Primary)
        );
      }

      // ボタンクリックを待機
      const questionMsg = await channel.send({ embeds: [embed], components: [row] });
      const interaction = await questionMsg.awaitMessageComponent({
        componentType: ComponentType.Button,
      });

      // 回答を記録
      answers[q.question] = selectedOption.label;
    }

    return answers;
  };
}
```

### PermissionRequest ハンドラー

承認が必要なツール使用時に Allow/Deny ボタンを表示します：

```typescript
// index.ts
function createPermissionRequestHandler(bot: any): PermissionRequestCallback {
  return async (toolName: string, toolInput: Record<string, unknown>): Promise<boolean> => {
    const channel = bot.getChannel();

    const embed = new EmbedBuilder()
      .setColor(0xff9900)
      .setTitle(`🔐 Permission Request`)
      .setDescription(`Tool: **${toolName}**`)
      .addFields({ name: 'Input Preview', value: JSON.stringify(toolInput).slice(0, 1000) });

    const row = new ActionRowBuilder().addComponents(
      new ButtonBuilder().setCustomId(`perm-req:${nonce}:allow`).setLabel('✅ Allow').setStyle(ButtonStyle.Success),
      new ButtonBuilder().setCustomId(`perm-req:${nonce}:deny`).setLabel('❌ Deny').setStyle(ButtonStyle.Danger),
    );

    const msg = await channel.send({ embeds: [embed], components: [row] });
    const interaction = await msg.awaitMessageComponent({ componentType: ComponentType.Button });

    return parsePermissionButtonId(interaction.customId)?.allowed ?? false;
  };
}
```

---

## セッション管理

### ClaudeSessionManager

セッションのライフサイクルを管理するクラスです：

```typescript
// claude/enhanced-client.ts
export class ClaudeSessionManager {
  private sessions = new Map<string, ClaudeSession>();

  createSession(workDir: string, model?: string): ClaudeSession {
    const session: ClaudeSession = {
      id: `session_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      startTime: new Date(),
      lastActivity: new Date(),
      messageCount: 0,
      totalCost: 0,
      model: model || 'claude-3-5-sonnet-20241022',
      workDir
    };

    this.sessions.set(session.id, session);
    return session;
  }

  getSession(sessionId: string): ClaudeSession | undefined {
    return this.sessions.get(sessionId);
  }

  updateSession(sessionId: string, cost?: number): void {
    const session = this.sessions.get(sessionId);
    if (session) {
      session.lastActivity = new Date();
      session.messageCount++;
      if (cost) session.totalCost += cost;
    }
  }

  getActiveSessions(maxAge: number = 3600000): ClaudeSession[] {
    const cutoff = Date.now() - maxAge;
    return Array.from(this.sessions.values()).filter(
      session => session.lastActivity.getTime() > cutoff
    );
  }

  cleanup(maxAge: number = 24 * 3600000): number {
    const cutoff = Date.now() - maxAge;
    let deleted = 0;
    for (const [id, session] of this.sessions.entries()) {
      if (session.lastActivity.getTime() < cutoff) {
        this.sessions.delete(id);
        deleted++;
      }
    }
    return deleted;
  }
}
```

### セッション ID 管理

セッション ID は以下のフォーマットで管理されます：

```typescript
// セッション ID のクリーニング
export function cleanSessionId(sessionId: string): string {
  return sessionId
    .trim()
    .replace(/^`+|`+$/g, '')         // バッククォート除去
    .replace(/^```\n?|\n?```$/g, '') // コードブロック除去
    .replace(/[\r\n]/g, '')          // 改行除去
    .trim();
}
```

### Continue / Resume

```typescript
// Continue モード（最新の会話を継続）
if (continueMode) {
  queryOptions.options.continue = true;
}

// Resume モード（特定セッションを再開）
if (cleanedSessionId && !continueMode) {
  queryOptions.options.resume = cleanedSessionId;
}
```

### Rewind 機能

ファイル変更を特定ターンまで巻き戻す機能：

```typescript
// claude/info-commands.ts
async function handleRewind(ctx: InteractionContext, turn: number, dryRun: boolean) {
  const activeQuery = getActiveQuery();
  if (!activeQuery) {
    await ctx.reply({ content: 'No active session', ephemeral: true });
    return;
  }

  if (dryRun) {
    // 変更をプレビュー（適用しない）
    const preview = await activeQuery.rewindFiles(messageId, { dryRun: true });
    await ctx.reply({ embeds: [formatRewindPreview(preview)] });
  } else {
    // 変更を適用
    await activeQuery.rewindFiles(messageId);
    await ctx.reply({ content: `Rewound to turn ${turn}` });
  }
}
```

---

## コマンド実装

### コマンド数: 45以上

このボットは 45 以上のスラッシュコマンドを提供します。

### Claude Core コマンド (4)

| コマンド | 説明 |
|----------|------|
| `/claude` | Claude Code にプロンプトを送信 |
| `/resume` | 前の会話を再開 |
| `/claude-cancel` | 実行中の操作をキャンセル |
| `/fast` | Opus 4.6 Fast Mode をトグル |

### 拡張 Claude コマンド (7)

| コマンド | 説明 |
|----------|------|
| `/claude-explain` | コードや概念を説明 |
| `/claude-debug` | エラーやコードをデバッグ |
| `/claude-optimize` | コードを最適化 |
| `/claude-review` | コードレビュー |
| `/claude-generate` | コードを生成 |
| `/claude-refactor` | コードをリファクタリング |
| `/claude-learn` | トピックを学習 |

### 情報・制御コマンド (3)

| コマンド | 説明 |
|----------|------|
| `/claude-info` | アカウント情報、モデル一覧、MCP ステータス |
| `/rewind` | ファイル変更を巻き戻し |
| `/claude-control` | セッション中のモデル/権限変更 |

### 設定コマンド (2)

| コマンド | 説明 |
|----------|------|
| `/settings` | 統合設定ハブ（カテゴリ: show, bot, claude, modes, output, proxy, developer, reset） |
| `/quick-model` | モデルを素早く切り替え |

### Task & Agent Management (3)

| コマンド | 説明 |
|----------|------|
| `/todos` | タスク管理（list, add, complete, generate, prioritize） |
| `/mcp` | MCP サーバー管理（list, add, remove, test, status, toggle, reconnect） |
| `/agent` | 特殊 AI エージェント実行（7種類の組み込みエージェント） |

### Git コマンド (6)

| コマンド | 説明 |
|----------|------|
| `/git` | Git コマンドを実行 |
| `/worktree` | 新しい worktree を作成 |
| `/worktree-list` | Worktree 一覧 |
| `/worktree-remove` | Worktree を削除 |
| `/worktree-bots` | Worktree 別 Bot 管理 |
| `/worktree-kill` | Worktree Bot を停止 |

### Shell コマンド (4)

| コマンド | 説明 |
|----------|------|
| `/shell` | シェルコマンドを実行 |
| `/shell-input` | 実行中プロセスに入力 |
| `/shell-list` | プロセス一覧 |
| `/shell-kill` | プロセスを停止 |

### システムモニタリング (11)

| コマンド | 説明 |
|----------|------|
| `/system-info` | システム情報 |
| `/processes` | プロセス一覧 |
| `/system-resources` | CPU/メモリ/ディスク使用量 |
| `/network-info` | ネットワーク情報 |
| `/disk-usage` | ディスク使用量詳細 |
| `/env-vars` | 環境変数（フィルタリング済み） |
| `/system-logs` | システムログ |
| `/port-scan` | ポートスキャン |
| `/service-status` | サービスステータス |
| `/uptime` | 稼働時間 |
| `/screenshot` | スクリーンショット取得 |

### ユーティリティ (4)

| コマンド | 説明 |
|----------|------|
| `/status` | Bot ステータス |
| `/pwd` | 現在ディレクトリ |
| `/shutdown` | Bot 停止 |
| `/help` | ヘルプ表示 |

### コマンド実装パターン

```typescript
// 1. コマンド定義（command.ts）
export const shellCommands = [
  new SlashCommandBuilder()
    .setName('shell')
    .setDescription('Execute shell commands on the host')
    .addStringOption(opt =>
      opt.setName('command')
         .setDescription('Command to execute')
         .setRequired(true)
    ),
];

// 2. ハンドラー実装（handler.ts）
export function createShellHandlers(deps: ShellHandlerDeps) {
  const { shellManager } = deps;

  return {
    async execute(ctx: InteractionContext) {
      await ctx.deferReply();

      const command = ctx.getString('command', true);
      const result = await shellManager.execute(command);

      await ctx.editReply({
        embeds: [{
          color: result.exitCode === 0 ? 0x00ff00 : 0xff0000,
          title: 'Shell Execution',
          fields: [
            { name: 'Command', value: `\`${command}\`` },
            { name: 'Output', value: result.stdout.slice(0, 1000) },
          ],
        }],
      });
    }
  };
}

// 3. レジストリへの登録（handler-registry.ts）
const shellHandlers = createShellHandlers({ shellManager });
```

---

## 環境変数

### 必須環境変数

| 変数名 | 説明 | 例 |
|--------|------|-----|
| `DISCORD_TOKEN` | Discord Bot トークン | `OTk2...` |
| `APPLICATION_ID` | Discord アプリケーション ID | `123456789012345678` |

### オプション環境変数

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `ANTHROPIC_API_KEY` | Claude API キー（動的モデル取得用） | - |
| `USER_ID` / `DEFAULT_MENTION_USER_ID` | メンション対象ユーザー ID | - |
| `CATEGORY_NAME` | Discord チャンネルカテゴリ名 | リポジトリ名 |
| `WORK_DIR` | 作業ディレクトリ | カレントディレクトリ |
| `ADMIN_ROLE_IDS` | 管理者ロール ID（カンマ区切り） | - |
| `ADMIN_USER_IDS` | 管理者ユーザー ID（カンマ区切り） | - |

### .env ファイル自動読み込み

```typescript
// index.ts
async function loadEnvFile(): Promise<void> {
  try {
    const envPath = `${Deno.cwd()}/.env`;
    const content = await Deno.readTextFile(envPath);
    const lines = content.split('\n');

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;

      const eqIndex = trimmed.indexOf('=');
      if (eqIndex === -1) continue;

      const key = trimmed.substring(0, eqIndex).trim();
      let value = trimmed.substring(eqIndex + 1).trim();

      // クォート除去
      if ((value.startsWith('"') && value.endsWith('"')) ||
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1);
      }

      if (!Deno.env.get(key) && key && value) {
        Deno.env.set(key, value);
      }
    }

    console.log('Loaded configuration from .env file');
  } catch (error) {
    console.warn(`Could not load .env file: ${error.message}`);
  }
}
```

---

## まとめ

claude-code-discord は以下の特徴を持つ堅牢な Discord ボットです：

1. **モジュラー設計**: 各機能が独立したモジュールとして実装され、保守性が高い
2. **SDK 統合**: Claude Agent SDK をフル活用し、ストリーミング、セッション管理、MCP 統合をサポート
3. **セキュリティ**: RBAC によるきめ細かなアクセス制御
4. **インタラクティブ性**: AskUserQuestion/PermissionRequest による Discord 上での対話的操作
5. **堅牢性**: クラッシュハンドリング、グレースフルシャットダウン、レート制限対応
6. **拡張性**: 設定システム、フックシステム、カスタムエージェントによる柔軟なカスタマイズ

**主要コンポーネント**:
- **Deno 2.2.0**: ランタイム環境
- **discord.js 14.14.1**: Discord API 統合
- **@anthropic-ai/claude-agent-sdk 0.2.45**: Claude Code 統合
- **45以上のコマンド**: Claude、Git、Shell、設定、システムモニタリング等
