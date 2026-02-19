# 実装パターンまとめ

## 概要

agent-discord-skillsとclaude-code-discordから学んだ実装パターンをまとめる。

---

## 1. スキル定義パターン（agent-discord-skills）

### ファイル構造

```
skill-name/
├── SKILL.md       # メインのスキル定義
└── examples.md    # 使用例
```

### SKILL.mdの構造

#### YAML frontmatter

```yaml
---
name: discord-send-message
description: Send messages to Discord channels via the Discord API. Use this skill when the user wants to send text messages, notifications, or formatted content to a Discord channel.
---
```

**必須フィールド**:
- `name`: スキル名（一意識別子、小文字、ハイフン区切り）
- `description`: スキルの説明と使用タイミング

#### Markdownセクション

1. **タイトル**: `# Discord Send Message`
2. **When to Use This Skill**: 使用タイミングの具体的な例
3. **Prerequisites**: 必要な環境変数と権限
4. **Instructions**: Claude Codeへの具体的な手順指示
5. **Validation Rules**: バリデーションルール
6. **Error Handling**: 一般的なエラーと対処法
7. **Security Notes**: セキュリティ上の注意点
8. **Examples**: examples.mdへの参照
9. **API Reference**: Discord APIのエンドポイント情報

### examples.mdの構造

1. **具体的な使用例**:
   - ユーザーリクエスト
   - Skill Actions（実行内容）
   - 期待されるレスポンス
   - ユーザーフィードバック

2. **エラーハンドリング例**:
   - 一般的なエラーシナリオ
   - エラーレスポンス
   - 対処法

3. **ベストプラクティス**:
   - 実装上の推奨事項
   - パフォーマンス上の注意点

4. **リファレンス**:
   - カラーコード
   - 権限値
   - APIステータスコード

---

## 2. APIリクエストパターン

### 基本的なリクエスト構造

```bash
curl -X {METHOD} "https://discord.com/api/v10/{ENDPOINT}" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{JSON_PAYLOAD}'
```

### 認証ヘッダー

```bash
-H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**重要**:
- `Bot ` プレフィックスが必要
- トークンは環境変数で管理
- ログやメッセージに暴露しない

### レスポンス処理

#### 成功レスポンス

**200 OK / 201 Created**:
```json
{
  "id": "987654321098765432",
  "content": "Message content",
  ...
}
```

**204 No Content**:
- 削除成功など
- レスポンスボディなし

#### エラーレスポンス

**400 Bad Request**:
```json
{
  "code": 50035,
  "message": "Invalid Form Body",
  "errors": {
    "name": {
      "_errors": [
        {
          "code": "BASE_TYPE_BAD_LENGTH",
          "message": "Must be between 2 and 100 in length."
        }
      ]
    }
  }
}
```

**401 Unauthorized**:
```json
{
  "code": 0,
  "message": "401: Unauthorized"
}
```

**403 Forbidden**:
```json
{
  "code": 50013,
  "message": "Missing Permissions"
}
```

**404 Not Found**:
```json
{
  "code": 10003,
  "message": "Unknown Channel"
}
```

---

## 3. バリデーションパターン

### チャンネルID

```javascript
// 18-19桁の数値
const channelIdRegex = /^\d{18,19}$/;
if (!channelIdRegex.test(channelId)) {
  throw new Error('Invalid channel ID format');
}
```

### チャンネル名

```javascript
// 2-100文字、小文字、ハイフン/アンダースコアのみ
const channelNameRegex = /^[a-z0-9_-]{2,100}$/;
if (!channelNameRegex.test(channelName)) {
  throw new Error('Invalid channel name');
}
```

### メッセージコンテンツ

```javascript
// 最大2000文字
if (content.length > 2000) {
  throw new Error('Message content exceeds 2000 characters');
}
```

### 権限ビット

```javascript
// 権限のビット演算
const SEND_MESSAGES = 2048;      // 0x800
const VIEW_CHANNEL = 1024;      // 0x400
const READ_MESSAGE_HISTORY = 65536;  // 0x10000

// 権限チェック
if ((permissions & SEND_MESSAGES) === SEND_MESSAGES) {
  // メッセージ送信権限がある
}
```

---

## 4. エラーハンドリングパターン

### 共通エラーハンドリング

```javascript
async function makeDiscordRequest(endpoint, options) {
  try {
    const response = await fetch(endpoint, options);
    
    if (!response.ok) {
      const error = await response.json();
      throw new DiscordError(error.code, error.message);
    }
    
    return await response.json();
  } catch (error) {
    if (error instanceof DiscordError) {
      handleDiscordError(error);
    } else {
      handleNetworkError(error);
    }
  }
}

function handleDiscordError(error) {
  switch (error.code) {
    case 401:
      console.error('Invalid bot token');
      break;
    case 403:
      console.error('Missing permissions');
      break;
    case 404:
      console.error('Resource not found');
      break;
    default:
      console.error(`Discord API error: ${error.message}`);
  }
}
```

### ユーザーフレンドリーなエラーメッセージ

```javascript
function getUserFriendlyErrorMessage(error) {
  switch (error.code) {
    case 50013:  // Missing Permissions
      return 'Error: The bot is missing the required permissions. Please check the bot\'s permissions in your Discord server.';
    case 10003:  // Unknown Channel
      return 'Error: Channel not found. Please verify the channel ID is correct.';
    case 50035:  // Invalid Form Body
      return 'Error: Invalid input. Please check your parameters.';
    default:
      return `Error: ${error.message}`;
  }
}
```

---

## 5. レート制限パターン

### 遅延挿入

```javascript
// リクエスト間に遅延を挿入
async function makeRequestsWithDelay(requests, delayMs = 1000) {
  for (const request of requests) {
    await makeDiscordRequest(request);
    await sleep(delayMs);
  }
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
```

### レート制限レスポンスの処理

```javascript
// Discordからレート制限情報を取得
const response = await fetch(endpoint, options);
const remaining = response.headers.get('X-RateLimit-Remaining');
const reset = response.headers.get('X-RateLimit-Reset');

if (remaining === '0') {
  const resetTime = new Date(reset * 1000);
  const waitTime = resetTime - Date.now();
  await sleep(waitTime);
}
```

---

## 6. ページネーションパターン

### メッセージのページネーション

```javascript
async function getAllMessages(channelId, limit = 100) {
  const messages = [];
  let lastMessageId = null;
  
  while (messages.length < limit) {
    const params = new URLSearchParams({
      limit: Math.min(100, limit - messages.length)
    });
    
    if (lastMessageId) {
      params.append('before', lastMessageId);
    }
    
    const response = await fetch(
      `https://discord.com/api/v10/channels/${channelId}/messages?${params}`,
      {
        headers: {
          'Authorization': `Bot ${DISCORD_BOT_TOKEN}`
        }
      }
    );
    
    const batch = await response.json();
    
    if (batch.length === 0) break;
    
    messages.push(...batch);
    lastMessageId = batch[batch.length - 1].id;
  }
  
  return messages;
}
```

### Embedのページネーション

```javascript
// 長いメッセージを分割
function splitMessageIntoPages(content, maxChars = 2000) {
  const pages = [];
  let currentPage = '';
  
  const lines = content.split('\n');
  
  for (const line of lines) {
    if (currentPage.length + line.length + 1 > maxChars) {
      pages.push(currentPage);
      currentPage = line;
    } else {
      currentPage += (currentPage ? '\n' : '') + line;
    }
  }
  
  if (currentPage) {
    pages.push(currentPage);
  }
  
  return pages;
}
```

---

## 7. 設定管理パターン（claude-code-discord）

### 環境変数の読み込み

```typescript
interface EnvConfig {
  DISCORD_TOKEN: string;
  APPLICATION_ID: string;
  ANTHROPIC_API_KEY?: string;
  USER_ID?: string;
  CATEGORY_NAME?: string;
  WORK_DIR?: string;
  ADMIN_ROLE_IDS?: string;
  ADMIN_USER_IDS?: string;
}

function loadEnvConfig(): EnvConfig {
  return {
    DISCORD_TOKEN: Deno.env.get('DISCORD_TOKEN') || '',
    APPLICATION_ID: Deno.env.get('APPLICATION_ID') || '',
    ANTHROPIC_API_KEY: Deno.env.get('ANTHROPIC_API_KEY'),
    USER_ID: Deno.env.get('USER_ID'),
    CATEGORY_NAME: Deno.env.get('CATEGORY_NAME'),
    WORK_DIR: Deno.env.get('WORK_DIR'),
    ADMIN_ROLE_IDS: Deno.env.get('ADMIN_ROLE_IDS'),
    ADMIN_USER_IDS: Deno.env.get('ADMIN_USER_IDS'),
  };
}

function validateEnvConfig(config: EnvConfig): void {
  if (!config.DISCORD_TOKEN) {
    throw new ConfigurationError('DISCORD_TOKEN is required');
  }
  if (!config.APPLICATION_ID) {
    throw new ConfigurationError('APPLICATION_ID is required');
  }
}
```

### デフォルト値の適用

```typescript
function applyDefaults(config: EnvConfig): RequiredEnvConfig {
  return {
    ...config,
    CATEGORY_NAME: config.CATEGORY_NAME || 'claude-code',
    WORK_DIR: config.WORK_DIR || Deno.cwd(),
    ADMIN_ROLE_IDS: config.ADMIN_ROLE_IDS || '',
    ADMIN_USER_IDS: config.ADMIN_USER_IDS || '',
  };
}
```

---

## 8. ハンドラー登録パターン

### コマンドハンドラー

```typescript
interface CommandHandlers {
  [commandName: string]: (interaction: CommandInteraction) => Promise<void>;
}

const commandHandlers: CommandHandlers = {
  'claude': handleClaudeCommand,
  'git': handleGitCommand,
  'shell': handleShellCommand,
  // ... その他のコマンド
};

async function handleCommand(interaction: CommandInteraction) {
  const commandName = interaction.commandName;
  const handler = commandHandlers[commandName];
  
  if (!handler) {
    await interaction.reply({
      content: 'Unknown command',
      ephemeral: true
    });
    return;
  }
  
  await handler(interaction);
}
```

### ボタンハンドラー

```typescript
interface ButtonHandlers {
  [buttonId: string]: (interaction: ButtonInteraction) => Promise<void>;
}

const buttonHandlers: ButtonHandlers = {
  'ask_user_yes': handleAskUserYes,
  'ask_user_no': handleAskUserNo,
  'permission_allow': handlePermissionAllow,
  'permission_deny': handlePermissionDeny,
  // ... その他のボタン
};

async function handleButton(interaction: ButtonInteraction) {
  const buttonId = interaction.customId;
  const handler = buttonHandlers[buttonId];
  
  if (!handler) {
    await interaction.reply({
      content: 'Unknown button',
      ephemeral: true
    });
    return;
  }
  
  await handler(interaction);
}
```

---

## 9. RBACパターン

### 権限チェック

```typescript
function hasAdminPermission(
  userId: string,
  config: EnvConfig
): boolean {
  // ユーザーIDチェック
  if (config.ADMIN_USER_IDS) {
    const adminUserIds = config.ADMIN_USER_IDS.split(',');
    if (adminUserIds.includes(userId)) {
      return true;
    }
  }
  
  // ロールチェック（Discord interactionからロールを取得）
  // const member = await interaction.guild.members.fetch(userId);
  // const hasRole = member.roles.cache.some(role =>
  //   config.ADMIN_ROLE_IDS.split(',').includes(role.id)
  // );
  
  return false;
}

async function checkAdminPermission(
  interaction: CommandInteraction,
  config: EnvConfig
): Promise<boolean> {
  if (!hasAdminPermission(interaction.user.id, config)) {
    await interaction.reply({
      content: 'You do not have permission to use this command.',
      ephemeral: true
    });
    return false;
  }
  return true;
}
```

### 破壊的コマンドの保護

```typescript
const PROTECTED_COMMANDS = ['shell', 'git', 'worktree'];

async function handleProtectedCommand(
  interaction: CommandInteraction,
  config: EnvConfig
): Promise<void> {
  const hasPermission = await checkAdminPermission(interaction, config);
  
  if (!hasPermission) {
    return;
  }
  
  // コマンドを実行
  await executeCommand(interaction);
}
```

---

## 10. セッション管理パターン

### セッションの作成と管理

```typescript
interface Session {
  id: string;
  channelId: string;
  createdAt: Date;
  lastActivityAt: Date;
  abortController: AbortController;
}

class SessionManager {
  private sessions: Map<string, Session> = new Map();
  
  createSession(channelId: string): Session {
    const sessionId = generateSessionId();
    const session: Session = {
      id: sessionId,
      channelId,
      createdAt: new Date(),
      lastActivityAt: new Date(),
      abortController: new AbortController(),
    };
    
    this.sessions.set(sessionId, session);
    return session;
  }
  
  getSession(sessionId: string): Session | undefined {
    return this.sessions.get(sessionId);
  }
  
  deleteSession(sessionId: string): void {
    const session = this.sessions.get(sessionId);
    if (session) {
      session.abortController.abort();
      this.sessions.delete(sessionId);
    }
  }
  
  cleanupInactiveSessions(maxAge: number): void {
    const now = Date.now();
    
    for (const [sessionId, session] of this.sessions) {
      const age = now - session.lastActivityAt.getTime();
      
      if (age > maxAge) {
        this.deleteSession(sessionId);
      }
    }
  }
}
```

---

## 11. メッセージフォーマットパターン

### Embedの作成

```typescript
interface EmbedData {
  title?: string;
  description?: string;
  color?: number;
  fields?: Array<{
    name: string;
    value: string;
    inline?: boolean;
  }>;
  footer?: {
    text: string;
  };
  timestamp?: string;
}

function createEmbed(data: EmbedData): EmbedData {
  return {
    title: data.title,
    description: data.description,
    color: data.color || 3447003,  // デフォルト: 青
    fields: data.fields || [],
    footer: data.footer,
    timestamp: data.timestamp || new Date().toISOString(),
  };
}
```

### コンポーネント（ボタン）の作成

```typescript
interface ComponentData {
  type: 1;  // ACTION_ROW
  components: Array<{
    type: 2;  // BUTTON
    label: string;
    style: 1 | 2 | 3 | 4;  // PRIMARY, SECONDARY, SUCCESS, DANGER
    customId: string;
  }>;
}

function createConfirmationButtons(): ComponentData {
  return {
    type: 1,
    components: [
      {
        type: 2,
        label: 'Yes',
        style: 3,  // SUCCESS
        customId: 'confirm_yes',
      },
      {
        type: 2,
        label: 'No',
        style: 4,  // DANGER
        customId: 'confirm_no',
      },
    ],
  };
}
```

---

## 12. ロギングパターン

### 構造化ログ

```typescript
interface LogEntry {
  level: 'info' | 'warn' | 'error';
  message: string;
  timestamp: string;
  context?: Record<string, unknown>;
}

function log(level: 'info' | 'warn' | 'error', message: string, context?: Record<string, unknown>): void {
  const entry: LogEntry = {
    level,
    message,
    timestamp: new Date().toISOString(),
    context,
  };
  
  console.log(JSON.stringify(entry));
}

// 使用例
log('info', 'Message sent', { channelId: '123', messageId: '456' });
log('error', 'Failed to send message', { error: error.message, channelId: '123' });
```

---

## まとめ

### 共通パターン

1. **環境変数**: トークンや設定を環境変数で管理
2. **バリデーション**: 入力値を厳密にチェック
3. **エラーハンドリング**: ユーザーフレンドリーなエラーメッセージ
4. **レート制限**: 適切な遅延を挿入
5. **ページネーション**: 大量データを分割して取得
6. **セキュリティ**: トークン保護、権限管理
7. **モジュール化**: 機能をモジュールに分割

### agent-discord-skills特有のパターン

1. **SKILL.md + examples.md**: ドキュメントとコードを分離
2. **YAML frontmatter**: メタデータの明示的な定義
3. **curlベースの例**: APIリクエストを明示的に示す

### claude-code-discord特有のパターン

1. **ハンドラー登録**: コマンド/ボタンを中央管理
2. **RBAC**: ロールベースアクセス制御
3. **セッション管理**: Claude Codeセッションの管理
4. **SDK統合**: @anthropic-ai/claude-agent-sdkの使用

これらのパターンを組み合わせることで、堅牢で保守しやすいDiscordボットを実装できる。
