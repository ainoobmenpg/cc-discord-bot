# Slash Commands 一覧

Discordで使用できるスラッシュコマンドの完全なリファレンスです。

---

## コマンド一覧

### `/ask` - GLM-4.7に質問

メインの対話コマンド。GLM-4.7に質問したり、タスクを実行させたりできます。

```
/ask <question>
```

**引数**:
- `question` (必須): 質問や指示内容

**例**:
```
/ask 今日の天気を調べて
/ask このファイルの内容を要約して: output/test.txt
/ask Pythonでフィボナッチ数列を書いて
```

**機能**:
- 会話履歴を保持（セッション管理）
- ツールの自動実行（ファイル操作、Web取得など）
- 長い応答は自動分割して送信

---

### `/clear` - セッション履歴クリア

現在のチャンネルの会話履歴をクリアします。

```
/clear
```

**効果**:
- チャンネル毎の会話履歴がリセットされます
- 新しい会話を始めたい時に使用

---

### `/tools` - ツール一覧表示

GLM-4.7が使用できるツールの一覧を表示します。

```
/tools
```

**表示内容**:
- ツール名
- 説明
- パラメータ

---

### `/schedule` - スケジュール管理

定期的なタスクをスケジュールします。

#### スケジュール追加

```
/schedule add <cron> <prompt>
```

**引数**:
- `cron`: Cron形式のスケジュール
- `prompt`: 実行するプロンプト

**Cron形式**:
```
分 時 日 月 曜日
* * * * *

例:
0 9 * * *     # 毎日9:00
0 9 * * 1     # 毎週月曜9:00
0 9 1 * *     # 毎月1日9:00
*/30 * * * *  # 30分毎
```

**例**:
```
/schedule add "0 9 * * *" おはよう！今日の予定を確認
/schedule add "0 18 * * 5" 週報をまとめて
```

#### スケジュール一覧

```
/schedule list
```

登録されているスケジュールの一覧を表示します。

#### スケジュール削除

```
/schedule remove <id>
```

**引数**:
- `id`: スケジュールID（UUID）

---

### `/permission` - 権限管理

ユーザーの権限を管理します（Admin以上のみ使用可能）。

#### 権限一覧

```
/permission list
```

自分の現在の権限を表示します。

#### 権限付与

```
/permission grant <user> <permission>
```

**引数**:
- `user`: 対象ユーザー（メンション）
- `permission`: 権限名

**権限一覧**:
| 権限 | 説明 |
|------|------|
| `FileRead` | ファイル読み取り |
| `FileWrite` | ファイル書き込み |
| `Schedule` | スケジュール管理 |
| `Admin` | 管理者権限（SuperUserのみ付与可能） |

**例**:
```
/permission grant @user123 FileWrite
```

#### 権限剥奪

```
/permission revoke <user> <permission>
```

**例**:
```
/permission revoke @user123 FileWrite
```

---

### `/memory` - メモリ操作

長期記憶として情報を保存・検索します。

#### メモリ追加

```
/memory add <content>
```

**例**:
```
/memory add 私の誕生日は3月15日です
/memory add プロジェクトAの担当は田中さん
```

#### メモリ一覧

```
/memory list
```

保存されているメモリの一覧を表示します。

#### メモリ検索

```
/memory search <query>
```

**例**:
```
/memory search 誕生日
/memory search プロジェクト
```

#### メモリ削除

```
/memory delete <id>
```

---

### `/settings` - ユーザー設定

ユーザー毎の設定を管理します。

#### 設定一覧

```
/settings list
```

#### 設定変更

```
/settings set <key> <value>
```

**設定一覧**:
| キー | 説明 | デフォルト |
|------|------|-----------|
| `output_subdir` | 出力サブディレクトリ | なし |

**例**:
```
/settings set output_subdir my-project
```

#### 設定リセット

```
/settings reset <key>
```

---

### `/admin` - 管理者コマンド

システム管理用のコマンドです（Admin以上のみ）。

#### ステータス確認

```
/admin status
```

ボットの状態を表示します：
- セッション数
- スケジュール数
- メモリ数

#### 設定リロード

```
/admin reload
```

設定を再読み込みします。

---

## 権限要件まとめ

| コマンド | 一般ユーザー | Admin | SuperUser |
|---------|:-----------:|:-----:|:---------:|
| `/ask` | ✅ | ✅ | ✅ |
| `/clear` | ✅ | ✅ | ✅ |
| `/tools` | ✅ | ✅ | ✅ |
| `/schedule` | ✅ | ✅ | ✅ |
| `/permission list` | ✅ | ✅ | ✅ |
| `/permission grant/revoke` | ❌ | ✅ | ✅ |
| `/memory` | ✅ | ✅ | ✅ |
| `/settings` | ✅ | ✅ | ✅ |
| `/admin` | ❌ | ✅ | ✅ |
