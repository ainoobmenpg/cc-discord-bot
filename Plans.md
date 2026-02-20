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

**テスト数**: 154

---

## v1.0.0 - 権限システム再設計 🚧 [feature:security]

### 概要

SuperUser権限追加、ロール連携、コマンドベース権限管理を統合。

### 権限チェックフロー

```
1. SuperUser? → 全チェックバイパス
2. 個別ユーザー権限? → 適用
3. ロール権限? → 適用
4. デフォルト権限 → 適用
```

### 設定ファイル形式（data/roles.json）

```json
{
  "roles": {
    "Admin": ["Admin", "FileRead", "FileWrite", "Schedule"],
    "Trusted": ["FileRead", "FileWrite", "Schedule"],
    "Member": ["FileRead"]
  },
  "default_permissions": ["FileRead"]
}
```

### タスク

- [ ] `Permission::SuperUser` 追加
- [ ] `SUPER_USER_IDS` 環境変数読み込み
- [ ] SuperUser時の全制限バイパス実装
- [ ] `src/role_config.rs` - ロール設定ファイル読み込み
- [ ] `RoleConfig` struct実装（Serde deserialize）
- [ ] `PermissionManager`にロールベース権限チェック追加
- [ ] Discord Guild APIからユーザーロール取得
- [ ] `/permission roles` - ロール-権限マッピング表示
- [ ] `/permission sync` - ロールと権限を同期
- [ ] `/permission grant @user <perm>` - 個別権限付与
- [ ] `/permission revoke @user <perm>` - 個別権限剥奪

---

## v1.1.0 - ユーザー毎設定管理 🚧

### 出力先パス

```
{BASE_OUTPUT_DIR}/{user_id}/           # デフォルト
{BASE_OUTPUT_DIR}/{custom_subdir}/     # カスタム設定時
```

### タスク

- [ ] `src/user_settings.rs` - ユーザー設定ストア
- [ ] `UserSettings` struct実装
- [ ] `ToolContext`に出力先パス生成ロジック追加
- [ ] `/settings output` - 出力先設定コマンド
- [ ] `/settings show` - 現在の設定表示

---

## v1.2.0 - 個人メモリ強化 🚧

### 拡張メモリスキーマ

```sql
ALTER TABLE memories ADD COLUMN category TEXT DEFAULT 'general';
ALTER TABLE memories ADD COLUMN tags TEXT DEFAULT '[]';
ALTER TABLE memories ADD COLUMN metadata TEXT DEFAULT '{}';
```

### タスク

- [ ] `Memory` structに`category`, `tags`, `metadata`追加
- [ ] マイグレーションロジック（既存DB互換）
- [ ] `/memory add --category` - カテゴリ付きメモリ追加
- [ ] `/memory add --tag` - タグ付きメモリ追加
- [ ] `/memory list --category` - カテゴリでフィルタ
- [ ] `/memory search` - 全文検索（LIKE実装）

---

## 将来機能（Backlog）

- v1.3.0: メモリエクスポート（Markdown/JSON）
- v1.4.0: チャンネル毎設定
- v2.0.0: マルチサーバー対応
