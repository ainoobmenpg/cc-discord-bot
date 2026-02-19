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
| v0.7.0 | Slash Commands基盤 | ✅ |
| v0.7.1 | Slash Commands実装完了 | ✅ |
| v0.7.2 | コード品質改善 | ✅ |
| v0.8.0 | CLI/HTTP API | ✅ |
| v0.9.0 | Windows対応 | ✅ |

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
ADMIN_USER_IDS=123,456
BASE_OUTPUT_DIR=/tmp/cc-bot        # ツール出力ディレクトリ（オプション）
```

---

## v0.7.2 - コード品質改善 ✅

**完了日時**: 2026-02-20

### 完了タスク

- [x] 古いプレフィックスコマンド(!xxx)の削除
- [x] デッドコード警告の解消（#[allow(dead_code)]追加）
- [x] `base_output_dir` を環境変数から読み込み（BASE_OUTPUT_DIR）

---

## v0.7.1 - Slash Commands完成 ✅

**完了日時**: 2026-02-20

### 完了タスク

- [x] `/ask` コマンド実装（GLM-4.7 API呼び出し、セッション履歴連携）
- [x] `/clear` コマンド実装（セッション履歴クリア）
- [x] `/tools` コマンド実装（ツール一覧表示）
- [x] `/schedule` コマンド実装（add/list/remove）
- [x] `/memory` コマンド実装（add/list/search/delete）
- [x] `/permission` コマンド実装（list/grant/revoke）
- [x] `/admin` コマンド実装（status/reload）

---

## v0.8.0 - CLI/HTTP API ✅

**完了日時**: 2026-02-20

### 完了タスク

- [x] HTTP API基盤（axum）
- [x] `GET /api/health` - ヘルスチェック
- [x] `POST /api/chat` - GLM-4.7に問い合わせ
- [x] `GET/POST/DELETE /api/schedules` - スケジュール管理
- [x] `GET/POST/DELETE /api/memories` - メモリ管理
- [x] `cc-cli` CLIツール作成

---

## v0.9.0 - Windows対応 ✅

- [x] `run.bat` 作成
- [x] `run.ps1` 作成

---

**テスト数**: 113
