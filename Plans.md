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
| v0.7.1 | Slash Commands実装完了 | cc:TODO |
| v0.9.0 | Windows対応 | ✅ |

---

## v0.7.1 - Slash Commands完成 cc:TODO

**プレースホルダーを実装に置き換える**

### 1. `/ask` コマンド実装
- [ ] Handler参照を渡すようシグネチャ変更
- [ ] GLM-4.7 API呼び出し実装
- [ ] セッション履歴との連携
- [ ] ツール実行対応

### 2. `/clear` コマンド実装
- [ ] Handler参照を渡すようシグネチャ変更
- [ ] セッション履歴クリア実装

### 3. `/tools` コマンド実装
- [ ] Handler参照を渡すようシグネチャ変更
- [ ] 登録済みツール一覧表示

### 4. `/schedule` コマンド確認・修正
- [ ] add/list/remove動作確認
- [ ] 必要に応じて修正

### 5. `/memory` コマンド確認・修正
- [ ] add/list/search/delete動作確認
- [ ] 必要に応じて修正

### 6. `/permission` コマンド確認・修正
- [ ] list/grant/revoke動作確認
- [ ] 必要に応じて修正

### 7. `/admin` コマンド確認・修正
- [ ] status/reload動作確認
- [ ] 必要に応じて修正

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
```

---

## v0.8.0 - CLI/HTTP API cc:TODO

**Discord外からの操作を可能にする**

- [ ] HTTP API基盤（axum）
- [ ] `POST /api/chat`
- [ ] `GET/POST/DELETE /api/schedules`
- [ ] `GET/POST/DELETE /api/memories`
- [ ] `cc-cli` CLIツール

---

## v0.9.0 - Windows対応 ✅

- [x] `run.bat` 作成
- [x] `run.ps1` 作成

---

**テスト数**: 113
