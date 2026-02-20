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
| v2.1.0 | MCP接続プール | ✅ |
| v2.2.0 | readability導入 | ✅ |
| v2.3.0 | セキュリティ強化 | ✅ |
| v2.4.0 | 包括的セキュリティ監査 | ✅ |
| v2.4.1 | レビュー指摘修正 | ✅ |

---

## v2.4.1 - レビュー指摘修正 ✅ [feature:security]

### 概要

v2.4.0セキュリティ監査実装後の厳格コードレビューで指摘された問題を修正。

### 優先度マトリックス

| 機能 | 優先度 | 工数 | 状態 |
|------|--------|------|------|
| タイミング攻撃対策 | **P0** | 0.5h | ✅ |
| クライアント識別改善 | **P0** | 1h | ✅ |
| JSON形式マスキング対応 | **P1** | 0.5h | ✅ |
| CSPヘッダー追加 | **P1** | 0.25h | ✅ |
| ドキュメンテーション修正 | **P2** | 0.25h | ⬜ |
| ログマスキング適用範囲拡大 | **P2** | 0.5h | ⬜ |
| エラーメッセージ一般化 | **P2** | 0.25h | ⬜ |

**合計工数**: 3.25時間（P0/P1完了: 2.25時間）

---

### Phase 1: タイミング攻撃対策 ✅ P0 Required [feature:security]

- [x] `subtle` クレートを追加
- [x] `api.rs` のAPIキー比較を定数時間比較に変更

---

### Phase 2: クライアント識別改善 ✅ P0 Required [feature:security]

- [x] X-Forwarded-For ヘッダーからIP取得
- [x] APIキー + IPアドレスの組み合わせで識別

---

### Phase 3: JSON形式マスキング対応 ✅ P1 Recommended [feature:security]

- [x] 正規表現を拡張してJSON形式に対応

---

### Phase 4: CSPヘッダー追加 ✅ P1 Recommended [feature:security]

- [x] Content-Security-Policy ヘッダーを追加

---

### Phase 5-7: P2 Optional ✅

- [x] ドキュメンテーション修正（logging.rs Example）
- [x] ログマスキング適用範囲拡大（api.rs ユーザーメッセージ）
- [x] エラーメッセージ一般化（read_file.rs, list_files.rs）

---

## v2.4.0 - 包括的セキュリティ監査 ✅ [feature:security]

### 完了内容

- [x] Phase 1: レートリミッター適用
- [x] Phase 2: ログマスキング実装
- [x] Phase 3: エラーメッセージ改善
- [x] Phase 4: セキュリティヘッダー追加

---

## v2.2.0 - Web取得/検索改善 🚧

### 概要

web_fetchツールの出力品質を改善し、検索ツールを追加して「調べて」系のリクエストに適切に対応できるようにする。

### 背景

**現状の問題**:
- web_fetchの出力が読みにくい（HTMLタグ、alt text、重複情報）
- ナビゲーションやメタデータも含まれてしまう
- 「検索して」に対して検索ツールではなく直接fetchを選択してしまう

### 優先度マトリックス

| 機能 | 優先度 | 工数 | 備考 |
|------|--------|------|------|
| readability導入 | Required | 0.5d | 本文抽出 ✅ |
| web_fetch改善 | Optional | 0.5d | 不要要素削除、フォーマット改善 |

**合計工数（目安）**: 1日

---

### Phase 1: readability導入 ✅ Required

- [x] ライブラリ選定・導入
- [x] 本文抽出機能の実装
- [x] テスト追加

#### ライブラリ候補

| ライブラリ | バージョン | 特徴 | 推奨 |
|-----------|-----------|------|------|
| `legible` | 0.4.1 | Mozilla Readability移植、最新 | ⭐ |
| `readability-rust` | 0.1.0 | Mozilla移植、18K+DL | |
| `readabilityrs` | 0.1.2 | Mozilla移植、93.8%テスト通過 | |

**推奨**: `legible` (最新、Rust 2024対応、APIシンプル)

#### 実装内容

```rust
// web_fetch.rs に追加
use legible::{parse, is_probably_readerable};

// fetch()内で本文抽出
if is_probably_readerable(&body, None) {
    match parse(&body, Some(&url), None) {
        Ok(article) => {
            // article.title, article.content, article.text_content を使用
        }
        Err(_) => // フォールバック: 従来の正規表現処理
    }
}
```

---

### Phase 2: web_fetch改善 ✅ Required

- [ ] imgタグ処理改善（alt textを適切に扱う）
- [ ] リンクプレビュー/OGP除外
- [ ] 出力フォーマット最適化
- [ ] テスト追加

#### 改善内容

1. **本文抽出後の処理**
   - 見出し構造を維持
   - リストを適切にMarkdown化
   - コードブロックの保持

2. **メタデータ処理**
   - タイトルを最初に表示
   - URL、更新日時を簡潔に表示
   - OGP画像は除外

3. **出力形式**
   ```markdown
   # 記事タイトル

   > URL: https://example.com/article
   > 取得日時: 2026-02-20

   本文内容...
   ```

---

## v2.3.0 - セキュリティ強化 ✅ [feature:security]

### 概要

複数サーバー導入に向けたセキュリティ強化。APIキー認証とシンボリックリンク対策を実装。

### 優先度マトリックス

| 機能 | 優先度 | 工数 |
|------|--------|------|
| APIキー必須化 | Required | 0.5h ✅ |
| シンボリックリンク検出 | Required | 1h ✅ |
| CORS設定強化 | Recommended | 0.5h ✅ |

**合計工数**: 2時間

---

### Phase 1: APIキー必須化 ✅ Required

- [x] APIキー未設定時の起動ブロック
- [x] 環境変数チェックの強化
- [x] エラーメッセージの改善

---

### Phase 2: シンボリックリンク検出 ✅ Required

- [x] 全ファイル操作ツールにsymlinkチェック追加
- [x] validation.rsの機能をツールに統合
- [x] テスト追加

#### 対象ファイル

- `src/tools/read_file.rs`
- `src/tools/write_file.rs`
- `src/tools/edit.rs`
- `src/tools/list_files.rs`

---

### Phase 3: CORS設定強化 ✅ Recommended

- [x] デフォルトCORSの無効化
- [x] ALLOWED_ORIGINS必須化（本番環境）
- [x] 設定ドキュメント追加

---

## 完了済みバージョン

### v2.1.0 - MCP接続プール ✅

- [x] MCPConnectionPool実装
- [x] 接続再利用によるパフォーマンス改善
- [x] アイドル接続の自動クリーンアップ

---

## 将来機能（Backlog）

- v2.3.0: ブラウザ自動化（headless-chrome）
- v2.4.0: Skills System（YAML定義）
- v3.0.0: マルチサーバー対応
- v3.1.0: Web UI（ダッシュボード）

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

**テスト数**: 293
