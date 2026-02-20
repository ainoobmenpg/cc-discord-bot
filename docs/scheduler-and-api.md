# スケジューラー・HTTP API

## スケジューラー

### 概要

スケジューラーは、Cron形式で指定した時間に自動的にプロンプトを実行する機能です。

### Cron形式

```
分 時 日 月 曜日
* * * * *

フィールド  | 範囲
-----------|----------------
分         | 0-59
時         | 0-23
日         | 1-31
月         | 1-12
曜日       | 0-6 (0=日曜)
```

### 特殊文字

| 文字 | 意味 | 例 |
|------|------|-----|
| `*` | 全ての値 | `* * * * *` = 毎分 |
| `,` | 値のリスト | `0,30 * * * *` = 毎時0分と30分 |
| `-` | 範囲 | `0 9-17 * * *` = 9時〜17時の毎時 |
| `/` | 間隔 | `*/15 * * * *` = 15分毎 |

### 使用例

| Cron | 意味 |
|------|------|
| `0 9 * * *` | 毎日9:00 |
| `0 9 * * 1` | 毎週月曜9:00 |
| `0 9 * * 1-5` | 平日9:00 |
| `0 9 1 * *` | 毎月1日9:00 |
| `*/30 * * * *` | 30分毎 |
| `0 */3 * * *` | 3時間毎 |
| `0 9,12,18 * * *` | 9時、12時、18時 |

### スケジュール構造

```rust
ScheduledTask {
    id: Uuid,                    // 一意ID
    cron_expression: String,     // Cron式
    prompt: String,              // 実行するプロンプト
    channel_id: u64,             // 送信先チャンネル
    created_at: DateTime<Utc>,   // 作成日時
    enabled: bool,               // 有効/無効
}
```

### コマンド

#### 追加

```
/schedule add "0 9 * * *" おはよう！今日のタスクを確認して
```

#### 一覧

```
/schedule list
```

出力例：
```
ID: 550e8400-e29b-41d4-a716-446655440000
Cron: 0 9 * * *
次回実行: 2026-02-22 09:00:00 UTC
プロンプト: おはよう！今日のタスクを確認して
```

#### 削除

```
/schedule remove <ID>
```

### 実行フロー

```
1. スケジューラーが毎分チェック
        ↓
2. 現在時刻にマッチするタスクを検索
        ↓
3. チャンネルにプロンプトを送信
        ↓
4. GLM-4.7がプロンプトを処理
        ↓
5. 必要に応じてツールを実行
        ↓
6. 結果をチャンネルに送信
```

---

## HTTP API

### 概要

HTTP APIは、Discord外からボットを操作するためのREST APIです。

### 認証

全てのAPIエンドポイントでBearer認証が必要です。

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" http://localhost:3000/api/...
```

### 環境変数

```bash
API_KEY=your-secure-api-key
API_PORT=3000
ALLOWED_ORIGINS=https://your-frontend.com
```

### エンドポイント

#### ヘルスチェック

```
GET /api/health
```

認証不要。

**レスポンス**:
```json
{
  "status": "ok",
  "version": "0.7.1"
}
```

---

#### チャット

```
POST /api/chat
```

**リクエスト**:
```json
{
  "user_id": 123456789,
  "message": "こんにちは",
  "channel_id": 987654321
}
```

**レスポンス**:
```json
{
  "response": "こんにちは！何かお手伝いできることはありますか？",
  "tool_calls": []
}
```

---

#### スケジュール一覧

```
GET /api/schedules
```

**レスポンス**:
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "cron_expression": "0 9 * * *",
    "prompt": "おはよう！",
    "channel_id": 123456789,
    "enabled": true,
    "next_run": "2026-02-22T09:00:00Z"
  }
]
```

---

#### スケジュール作成

```
POST /api/schedules
```

**リクエスト**:
```json
{
  "cron_expression": "0 9 * * *",
  "prompt": "おはよう！",
  "channel_id": 123456789
}
```

---

#### スケジュール削除

```
DELETE /api/schedules/{id}
```

---

#### メモリ一覧

```
GET /api/memories?user_id=123456789
```

---

#### メモリ作成

```
POST /api/memories
```

**リクエスト**:
```json
{
  "user_id": 123456789,
  "content": "重要な情報",
  "category": "general",
  "tags": ["important"]
}
```

---

#### メモリ検索

```
GET /api/memories/search?user_id=123456789&query=キーワード
```

---

#### メモリ削除

```
DELETE /api/memories/{id}
```

---

### セキュリティ

#### ヘッダー

全レスポンスに以下のセキュリティヘッダーが付与されます：

```
Content-Security-Policy: default-src 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
```

#### レートリミット

- 1クライアントあたり1分間に10リクエスト
- 超過時は `429 Too Many Requests` を返却

#### ログマスキング

APIキーや機密情報はログ出力時に自動的にマスキングされます。

---

### 使用例

```bash
# チャット
curl -X POST http://localhost:3000/api/chat \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"user_id": 123, "message": "こんにちは", "channel_id": 456}'

# スケジュール一覧
curl http://localhost:3000/api/schedules \
  -H "Authorization: Bearer YOUR_API_KEY"
```
