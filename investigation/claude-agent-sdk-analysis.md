# Claude Agent SDK調査結果

更新日: 2026-02-19

---

## 概要

Claude Agent SDK（旧称: Claude Code SDK）は、Claude Codeをライブラリとして使用するためのSDK。PythonとTypeScriptで提供されており、Claude Codeと同じツール、エージェントループ、コンテキスト管理をプログラマブルに利用できる。

---

## 公式ドキュメント

- **TypeScript**: https://docs.claude.com/en/docs/agent-sdk/typescript
- **Python**: https://docs.claude.com/en/docs/agent-sdk/python
- **Overview**: https://docs.claude.com/en/docs/agent-sdk/overview
- **GitHub（TypeScript）**: https://github.com/anthropics/claude-agent-sdk-typescript
- **GitHub（Python）**: https://github.com/anthropics/claude-agent-sdk-python

---

## インストール

### TypeScript

```bash
npm install @anthropic-ai/claude-agent-sdk
```

### Python

```bash
pip install claude-agent-sdk
```

**前提条件**:
- Python 3.10+

---

## 認証

### APIキー

```bash
export ANTHROPIC_API_KEY=your-api-key
```

### サードパーティプロバイダー

- **Amazon Bedrock**: `CLAUDE_CODE_USE_BEDROCK=1`
- **Google Vertex AI**: `CLAUDE_CODE_USE_VERTEX=1`
- **Microsoft Azure**: `CLAUDE_CODE_USE_FOUNDRY=1`

---

## 基本的な使い方

### query()関数

#### TypeScript

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Find and fix the bug in auth.py",
  options: { allowedTools: ["Read", "Edit", "Bash"] }
})) {
  console.log(message);
}
```

#### Python

```python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions

async def main():
    async for message in query(
        prompt="Find and fix the bug in auth.py",
        options=ClaudeAgentOptions(allowed_tools=["Read", "Edit", "Bash"]),
    ):
        print(message)

asyncio.run(main())
```

---

## 主な機能

### 1. Built-in Tools

| Tool | 説明 |
|------|------|
| **Read** | ファイルを読む |
| **Write** | ファイルを作成 |
| **Edit** | ファイルを編集 |
| **Bash** | コマンドを実行 |
| **Glob** | ファイルをパターンで検索 |
| **Grep** | ファイル内容を検索 |
| **WebSearch** | Webを検索 |
| **WebFetch** | Webページを取得 |
| **AskUserQuestion** | ユーザーに質問 |

### 2. Hooks

エージェントライフサイクルの特定のポイントでカスタムコードを実行。

**利用可能なフック**:
- `PreToolUse`
- `PostToolUse`
- `Stop`
- `SessionStart`
- `SessionEnd`
- `UserPromptSubmit`

**例**（TypeScript）:

```typescript
import { query, HookCallback } from "@anthropic-ai/claude-agent-sdk";
import { appendFile } from "fs/promises";

const logFileChange: HookCallback = async (input) => {
  const filePath = (input as any).tool_input?.file_path ?? "unknown";
  await appendFile("./audit.log", `${new Date().toISOString()}: modified ${filePath}\n`);
  return {};
};

for await (const message of query({
  prompt: "Refactor utils.py to improve readability",
  options: {
    permissionMode: "acceptEdits",
    hooks: {
      PostToolUse: [{ matcher: "Edit|Write", hooks: [logFileChange] }]
    }
  }
})) {
  if ("result" in message) console.log(message.result);
}
```

### 3. Subagents

専門的なサブタスクを処理するために専門化されたエージェントを生成。

**例**（TypeScript）:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Use the code-reviewer agent to review this codebase",
  options: {
    allowedTools: ["Read", "Glob", "Grep", "Task"],
    agents: {
      "code-reviewer": {
        description: "Expert code reviewer for quality and security reviews.",
        prompt: "Analyze code quality and suggest improvements.",
        tools: ["Read", "Glob", "Grep"]
      }
    }
  }
})) {
  if ("result" in message) console.log(message.result);
}
```

### 4. MCP（Model Context Protocol）

外部システム（データベース、ブラウザ、APIなど）に接続。

**例**（TypeScript）:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Open example.com and describe what you see",
  options: {
    mcpServers: {
      playwright: { command: "npx", args: ["@playwright/mcp@latest"] }
    }
  }
})) {
  if ("result" in message) console.log(message.result);
}
```

### 5. Permissions

ツールの使用許可を制御。

**例**（TypeScript）:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Review this code for best practices",
  options: {
    allowedTools: ["Read", "Glob", "Grep"],
    permissionMode: "bypassPermissions"
  }
})) {
  if ("result" in message) console.log(message.result);
}
```

### 6. Sessions

複数の交換にわたってコンテキストを維持。

**例**（TypeScript）:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

let sessionId: string | undefined;

// First query: capture the session ID
for await (const message of query({
  prompt: "Read the authentication module",
  options: { allowedTools: ["Read", "Glob"] }
})) {
  if (message.type === "system" && message.subtype === "init") {
    sessionId = message.sessionId;
  }
}

// Resume with full context from the first query
for await (const message of query({
  prompt: "Now find all places that call it",
  options: { resume: sessionId }
})) {
  if ("result" in message) console.log(message.result);
}
```

---

## Claude Code機能

SDKは、ファイルシステムベースの設定もサポート。

| Feature | 説明 | 場所 |
|---------|------|------|
| Skills | マークダウンで定義された専門機能 | .claude/skills/SKILL.md |
| Slash commands | 一般的なタスクのためのカスタムコマンド | .claude/commands/*.md |
| Memory | プロジェクトコンテキストと指示 | CLAUDE.md または .claude/CLAUDE.md |
| Plugins | カスタムコマンド、エージェント、MCPサーバーで拡張 | pluginsオプションでプログラマブル |

---

## 比較

### Agent SDK vs Client SDK

| Feature | Client SDK | Agent SDK |
|---------|-----------|-----------|
| ツール実装 | 自分で実装 | 組み込み |
| ツールループ | 自分で実装 | Claudeが処理 |
| 制御 | 細かい制御 | 高度な抽象化 |

**Client SDK**:
```typescript
// ツールループを自分で実装
let response = await client.messages.create({ ...params });
while (response.stop_reason === "tool_use") {
  const result = yourToolExecutor(response.tool_use);
  response = await client.messages.create({ tool_result: result, ...params });
}
```

**Agent SDK**:
```typescript
// Claudeがツールを自律的に処理
for await (const message of query({ prompt: "Fix the bug in auth.py" })) {
  console.log(message);
}
```

### Agent SDK vs Claude Code CLI

| Use case | Best choice |
|----------|-------------|
| インタラクティブな開発 | CLI |
| CI/CDパイプライン | SDK |
| カスタムアプリケーション | SDK |
| ワンオフタスク | CLI |
| 本番 automation | SDK |

---

## TypeScript API

### query()

```typescript
async function query(params: {
  prompt: string;
  options?: ClaudeAgentOptions;
}): AsyncIterable<Message>
```

### ClaudeAgentOptions

```typescript
interface ClaudeAgentOptions {
  systemPrompt?: string;
  allowedTools?: string[];
  permissionMode?: "bypassPermissions" | "acceptEdits";
  hooks?: Record<string, HookCallback[]>;
  agents?: Record<string, AgentDefinition>;
  mcpServers?: Record<string, McpServerConfig>;
  resume?: string;
  cwd?: string;
}
```

---

## Python API

### query()

```python
async def query(
    *,
    prompt: str | AsyncIterable[dict[str, Any]],
    options: ClaudeAgentOptions | None = None
) -> AsyncIterator[Message]
```

### ClaudeAgentOptions

```python
class ClaudeAgentOptions:
    system_prompt: str | None
    allowed_tools: list[str] | None
    permission_mode: str | None  # "bypassPermissions" | "acceptEdits"
    hooks: dict[str, list[HookMatcher]] | None
    agents: dict[str, AgentDefinition] | None
    mcp_servers: dict[str, McpServerConfig] | None
    resume: str | None
    cwd: str | None
```

---

## カスタムツール（Pythonのみ）

### @toolデコレータ

```python
from claude_agent_sdk import tool, create_sdk_mcp_server

@tool("greet", "Greet a user", {"name": str})
async def greet_user(args: dict[str, Any]) -> dict[str, Any]:
    return {"content": [{"type": "text", "text": f"Hello, {args['name']}!"}]}

# Create an SDK MCP server
server = create_sdk_mcp_server(
    name="my-tools",
    version="1.0.0",
    tools=[greet_user]
)

# Use it with Claude
options = ClaudeAgentOptions(
    mcp_servers={"tools": server},
    allowed_tools=["mcp__tools__greet"]
)

async with ClaudeSDKClient(options=options) as client:
    await client.query("Greet Alice")
    async for msg in client.receive_response():
        print(msg)
```

---

## ClaudeSDKClient（Pythonのみ）

### セッションを維持するクライアント

```python
from claude_agent_sdk import ClaudeSDKClient

async with ClaudeSDKClient() as client:
    # First question
    await client.query("What's the capital of France?")
    
    async for message in client.receive_response():
        if isinstance(message, AssistantMessage):
            for block in message.content:
                if isinstance(block, TextBlock):
                    print(f"Claude: {block.text}")
    
    # Follow-up question - Claude remembers the previous context
    await client.query("What's the population of that city?")
    
    async for message in client.receive_response():
        if isinstance(message, AssistantMessage):
            for block in message.content:
                if isinstance(block, TextBlock):
                    print(f"Claude: {block.text}")
```

---

## エラーハンドリング（Python）

```python
from claude_agent_sdk import (
    ClaudeSDKError,      # Base error
    CLINotFoundError,    # Claude Code not installed
    CLIConnectionError,  # Connection issues
    ProcessError,        # Process failed
    CLIJSONDecodeError,  # JSON parsing issues
)

try:
    async for message in query(prompt="Hello"):
        pass
except CLINotFoundError:
    print("Please install Claude Code")
except ProcessError as e:
    print(f"Process failed with exit code: {e.exit_code}")
except CLIJSONDecodeError as e:
    print(f"Failed to parse response: {e}")
```

---

## ライセンスと規約

- **ライセンス**: AnthropicのCommercial Terms of Service
- **ブランディング**: 
  - 許可: "Claude Agent", "{YourAgentName} Powered by Claude"
  - 禁止: "Claude Code", "Claude Code Agent"

---

## 次のステップ

### 1. HTTP APIとしてラップ

Claude Agent SDKをHTTP APIとしてラップして、Rustから呼び出せるようにする。

**アーキテクチャ**:
```
Rust Discord Bot → HTTP Client → Claude Agent SDK API（Node.js） → Claude
```

**実装言語**: TypeScript

**理由**:
- SDKがTypeScriptで提供されている
- 非同期処理が自然
- Express.jsでHTTPサーバーを簡単に実装できる

### 2. API仕様（予定）

#### POST /query

**リクエスト**:
```json
{
  "prompt": "Hello, Claude!",
  "options": {
    "allowedTools": ["Read", "Write", "Bash"],
    "permissionMode": "acceptEdits"
  }
}
```

**レスポンス**:
```json
{
  "sessionId": "session-id",
  "messages": [
    {
      "type": "assistant",
      "content": [
        {
          "type": "text",
          "text": "Hi! How can I help you?"
        }
      ]
    }
  ]
}
```

#### GET /session

**リクエスト**:
```
GET /session?sessionId=session-id
```

**レスポンス**:
```json
{
  "sessionId": "session-id",
  "createdAt": "2026-02-19T12:00:00Z",
  "lastActivity": "2026-02-19T12:05:00Z"
}
```

#### DELETE /session

**リクエスト**:
```
DELETE /session?sessionId=session-id
```

**レスポンス**:
```json
{
  "success": true
}
```

---

## 参考リソース

- [Agent SDK Overview](https://docs.claude.com/en/docs/agent-sdk/overview)
- [TypeScript SDK](https://docs.claude.com/en/docs/agent-sdk/typescript)
- [Python SDK](https://docs.claude.com/en/docs/agent-sdk/python)
- [Quickstart](https://docs.claude.com/en/docs/agent-sdk/quickstart)
- [Example Agents](https://github.com/anthropics/claude-agent-sdk-demos)
