# cc-discord-bot 実装計画

## 完了状況

| バージョン | 機能 | 状態 |
|-----------|------|------|
| v0.1.0 | MVP (GLM連携) | ✅ |
| v0.2.0 | セッション管理 | ✅ |
| v0.3.0 | ツール実行 | ✅ |
| v0.4.0 | スケジューラー | ✅ |
| v0.5.0 | セキュリティ | ✅ |
| v0.6.0 | メモリシステム | ✅ |
| v0.7.x | Slash Commands | ✅ |
| v0.8.x | CLI/HTTP API | ✅ |
| v0.9.0 | Windows対応 | ✅ |
| v1.0.0 | 権限システム再設計 | ✅ |
| v1.1.0 | ユーザー毎設定管理 | ✅ |
| v1.2.0 | 個人メモリ強化 | ✅ |
| v1.3.0 | コード品質改善 | ✅ |
| v1.4.0 | メモリエクスポート | ✅ |
| v1.5.0 | チャンネル毎設定 | ✅ |
| v2.0.0 | ClaudeCode統合 | ✅ |

> 📦 アーカイブ: [Plans-archive-2026-02-20.md](.claude/memory/archive/Plans-archive-2026-02-20.md)

---

## 利用可能なコマンド（Slash Commands）

```
/ask <question>                     # GLM-4.7に質問
/clear                              # セッション履歴クリア
/tools                              # ツール一覧
/schedule add/list/remove           # スケジュール管理
/permission list/grant/revoke       # パーミッション
/memory add/list/search/delete      # メモリ操作
/admin status/reload                # 管理者コマンド
```

## 環境変数

```bash
DISCORD_BOT_TOKEN=xxx
GLM_API_KEY=xxx
GLM_MODEL=glm-4.7
ADMIN_USER_IDS=123,456          # 管理者ユーザー
SUPER_USER_IDS=789              # 制限なしユーザー（環境変数のみ）
BASE_OUTPUT_DIR=/tmp/cc-bot
ALLOWED_ORIGINS=http://localhost:3000
API_KEY=your-api-key
```

## 権限階層

| 権限 | 設定方法 | アクセス範囲 |
|------|---------|-------------|
| **SuperUser** | 環境変数 | 全ディレクトリ、全操作（制限なし） |
| Admin | 環境変数/ロール/コマンド | + ユーザー権限管理 |
| Trusted | ロール/コマンド | + 書き込み、スケジュール |
| Member | デフォルト | 読み取りのみ |

---

**テスト数**: 276

---

## v1.4.0 - メモリエクスポート（Markdown/JSON） ✅

- [x] メモリをMarkdown形式でエクスポート
- [x] メモリをJSON形式でエクスポート
- [x] `/memory export` コマンド追加

---

## v1.5.0 - チャンネル毎設定 ✅

- [x] チャンネルごとのワーキングディレクトリ設定
- [x] チャンネルごとの権限設定
- [x] `/settings channel` コマンド追加

---

## v2.0.0 - ClaudeCode統合 ✅ [feature:claude-integration]

### 概要

Discord経由でClaudeCode相当の機能を提供。
**GLM-4.7ベース**で実装し、抽象化レイヤー経由で将来的にClaude APIにも対応可能。

### アーキテクチャ

```
Discord Bot (cc-discord-bot)
    ↓ LLM抽象化レイヤー (LLMClient trait)
GLM-4.7 (現在) / Claude (将来対応)
    ↓ Tool Calls
ツール群 (Read/Write/Edit/Glob/Grep/Bash)
    ↓ MCP Protocol
MCP Servers (Skills/Plugins)
```

### 優先度マトリックス

| 機能 | 優先度 | 工数 | 備考 |
|------|--------|------|------|
| LLM抽象化レイヤー | Required | 2d | GLM実装、Claude対応準備 |
| Editツール | Required | 1d | 部分編集必須 |
| Globツール | Required | 0.5d | ファイル検索 |
| Grepツール | Required | 0.5d | 内容検索 |
| Bashツール | Required | 2d | クロスプラットフォーム |
| MCP統合 | Required | 5d | rmcp/mcprクレート使用 |
| チャンクストリーミング | Recommended | 2d | SSE実装 |
| Skills実行エンジン | Required | 3d | MCPツール連携 |

**合計工数（目安）**: 16日

### Phase 1: LLM抽象化レイヤー ✅

- [x] `src/llm.rs` - `LLMClient` trait定義
- [x] `src/llm/glm.rs` - GLM実装（既存glm.rsから移行）
- [x] `src/llm/mod.rs` - モジュール統合
- [x] main.rsでLLMClient経由で使用するよう修正
- [x] テスト: モックLLMClientでツール動作確認

### Phase 2: ツール拡張 ✅

- [x] `src/tools/edit.rs` - Editツール（部分編集）
- [x] `src/tools/glob.rs` - Globツール（ファイル検索）
- [x] `src/tools/grep.rs` - Grepツール（内容検索）
- [x] `src/tools/bash.rs` - Bashツール（クロスプラットフォーム）
- [x] Windows対応（PowerShell/batファイル実行）

### Phase 3: MCP統合 ✅

- [x] `Cargo.toml` に `rmcp` または `mcpr` クレート追加
- [x] `src/mcp_client.rs` - MCPクライアント実装
- [x] MCPサーバー設定ファイル（`mcp-servers.json`）
- [x] 動的ツールロード（MCP tools）
- [x] Skills実行エンジン

### Phase 4: UX改善 ✅

- [x] チャンクストリーミング表示（Discordメッセージ更新）
- [x] ツール実行の進捗表示
- [x] メッセージ監視モード（`/ask` なしで反応）
- [x] ツール実行のユーザー確認（権限設定連動）

### 環境変数（追加）

```bash
# LLMプロバイダー（現在はGLMのみ、将来Claude対応）
LLM_PROVIDER=glm              # glm | claude（将来）
# GLM設定（現状維持）
GLM_API_KEY=xxx
GLM_MODEL=glm-4.7
# Claude設定（将来用、オプション）
# ANTHROPIC_API_KEY=xxx
# CLAUDE_MODEL=claude-sonnet-4-6
# MCP設定
MCP_SERVERS=path/to/servers.json
```

### TDD戦略

| フェーズ | テスト観点 | カバレッジ目標 |
|---------|-----------|---------------|
| Phase 1 | LLM trait、GLM実装 | 80% |
| Phase 2 | 各ツールの単体テスト | 85% |
| Phase 3 | MCPプロトコルテスト | 75% |
| Phase 4 | 統合テスト | 70% |

### 将来のClaude対応

抽象化レイヤー導入後、Claude対応は以下の手順で追加可能：

1. `src/llm/claude.rs` を作成
2. `LLMClient` traitを実装
3. `LLM_PROVIDER=claude` で切り替え

---

## 将来機能（Backlog）

- v2.1.0: マルチサーバー対応
- v2.2.0: Web UI（ダッシュボード）
