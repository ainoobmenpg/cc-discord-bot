# セッション管理・メモリシステム

## セッション管理

### 概要

セッション管理は、ユーザーとの会話履歴を保持する機能です。チャンネル毎に独立したセッションが作成されます。

### セッションキー

セッションは「ユーザーID + チャンネルID」で一意に識別されます。

```
SessionKey {
    user_id: 123456789,
    channel_id: 987654321
}
```

### セッション構造

```rust
Session {
    id: Uuid,                    // 一意ID
    key: SessionKey,             // 識別キー
    history: ChatHistory,        // 会話履歴
    created_at: DateTime<Utc>,   // 作成日時
    last_active: DateTime<Utc>,  // 最終活動日時
}
```

### 履歴サイズ

- デフォルト: 20メッセージ
- 古いメッセージは自動的に削除（FIFO）

### タイムアウト

- 非アクティブ状態が続くとセッションが期限切れ
- 期限切れセッションは自動クリーンアップ

### 永続化

セッションは SQLite データベースに保存されます。

```
data/sessions.db
├── sessions テーブル
│   ├── id (TEXT, PRIMARY KEY)
│   ├── user_id (INTEGER)
│   ├── channel_id (INTEGER)
│   ├── history (TEXT, JSON)
│   ├── created_at (TEXT)
│   └── last_active (TEXT)
```

### フロー

```
ユーザーが /ask を入力
        ↓
1. セッションキーを生成（user_id + channel_id）
        ↓
2. セッションストアから既存セッションを取得
   ├─ 存在する → 履歴を復元
   └─ 存在しない → 新規作成
        ↓
3. ユーザーメッセージを履歴に追加
        ↓
4. LLMにリクエスト（履歴付き）
        ↓
5. LLM応答を履歴に追加
        ↓
6. セッションを保存
        ↓
7. 応答をDiscordに送信
```

---

## メモリシステム

### 概要

メモリシステムは、長期記憶として情報を保存・検索する機能です。セッションがクリアされても保持されます。

### メモリ構造

```rust
Memory {
    id: i64,                     // 一意ID
    user_id: u64,                // ユーザーID
    content: String,             // 内容
    category: String,            // カテゴリ
    tags: Vec<String>,           // タグ
    metadata: HashMap<String, String>,  // メタデータ
    created_at: DateTime<Utc>,   // 作成日時
    updated_at: DateTime<Utc>,   // 更新日時
}
```

### 機能

#### 保存（remember）

```
/memory add <content>
```

またはツール経由：
```
remember(content="内容", tags=["tag1", "tag2"])
```

#### 検索（recall）

```
/memory search <query>
```

またはツール経由：
```
recall(query="検索キーワード", limit=5)
```

#### 一覧

```
/memory list
```

#### 削除

```
/memory delete <id>
```

### 永続化

メモリは SQLite データベースに保存されます。

```
data/sessions.db
├── memories テーブル
│   ├── id (INTEGER, PRIMARY KEY)
│   ├── user_id (INTEGER)
│   ├── content (TEXT)
│   ├── category (TEXT)
│   ├── tags (TEXT, JSON)
│   ├── metadata (TEXT, JSON)
│   ├── created_at (TEXT)
│   └── updated_at (TEXT)
```

### 検索機能

- **全文検索**: content カラムに対する LIKE 検索
- **カテゴリフィルタ**: カテゴリで絞り込み
- **ユーザー分離**: 自分のメモリのみアクセス可能

---

## セッション vs メモリ

| 特性 | セッション | メモリ |
|------|-----------|--------|
| **寿命** | 会話中（クリア可能） | 永続 |
| **スコープ** | チャンネル毎 | ユーザー全体 |
| **内容** | 会話履歴全て | 重要な情報のみ |
| **自動使用** | ✅ 自動的にLLMに送信 | ❌ LLMが明示的に検索 |
| **サイズ制限** | 20メッセージ | 無制限 |

---

## 使用例

### セッションの活用

```
ユーザー: /ask 今日の天気は？
ボット: 今日は晴れです...

ユーザー: /ask 明日は？（"天気"を省略）
ボット: 明日は雨の予報です...
（セッション履歴から文脈を理解）
```

### メモリの活用

```
ユーザー: /ask 私の誕生日は3月15日です。覚えておいて
ボット: （rememberツールを実行）承知しました。

[数日後]

ユーザー: /ask 私の誕生日はいつ？
ボット: （recallツールを実行）3月15日ですね。
```

---

## データ管理

### データベース場所

```
cc-bot/data/sessions.db
```

### バックアップ

データベースファイルをコピーしてバックアップできます。

```bash
cp cc-bot/data/sessions.db backup/sessions_$(date +%Y%m%d).db
```

### データ削除

```bash
# 全データ削除
rm cc-bot/data/sessions.db
```
