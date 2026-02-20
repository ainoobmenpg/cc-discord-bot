# 権限システム仕様

## 概要

cc-discord-botの権限システムは、ファイル操作やスケジュール管理などの機能へのアクセスを制御します。

---

## 権限階層

```
SuperUser（最上位）
    ↓ 付与可能
Admin
    ↓ 付与可能
一般ユーザー（デフォルト権限）
```

---

## 権限一覧

| 権限 | 説明 | デフォルト | 付与可能者 |
|------|------|:--------:|-----------|
| `FileRead` | ファイル読み取り | ✅ | Admin |
| `FileWrite` | ファイル書き込み | ✅ | Admin |
| `Schedule` | スケジュール管理 | ✅ | Admin |
| `Admin` | 管理者権限・他ユーザーの権限管理 | ❌ | **SuperUserのみ** |
| `SuperUser` | 全権限・制限なし | ❌ | **環境変数のみ** |

---

## ユーザータイプ

### SuperUser（スーパーユーザー）

- **設定方法**: 環境変数 `SUPER_USER_IDS` のみ
- **権限**: 全権限（FileRead, FileWrite, Schedule, Admin, SuperUser）
- **特徴**:
  - 全ての権限チェックをバイパス
  - Admin権限の付与/剥奪が可能
  - SuperUser権限の付与は不可（環境変数経由のみ）

```bash
# .env
SUPER_USER_IDS=123456789,987654321
```

### Admin（管理者）

- **設定方法**: 環境変数 `ADMIN_USER_IDS` または SuperUserによる付与
- **権限**: デフォルト権限 + Admin
- **特徴**:
  - 一般ユーザーに権限を付与/剥奪可能
  - Admin権限の付与は不可（SuperUserのみ可能）

```bash
# .env
ADMIN_USER_IDS=111222333,444555666
```

### 一般ユーザー

- **設定方法**: なし（デフォルト）
- **権限**: FileRead, FileWrite, Schedule
- **特徴**:
  - 基本的な機能を利用可能
  - Admin/SuperUser権限なし

---

## 権限チェックフロー

```
リクエスト受信
    ↓
SuperUser?
    ├─ Yes → ✅ 許可（全チェックバイパス）
    └─ No ↓
個別ユーザー権限にあり?
    ├─ Yes → ✅ 許可
    └─ No ↓
ロール権限にあり?
    ├─ Yes → ✅ 許可
    └─ No ↓
デフォルト権限にあり?
    ├─ Yes → ✅ 許可
    └─ No → ❌ 拒否
```

---

## Discord コマンド

### 権限確認

```
/permission list
```

### 権限付与（Admin以上）

```
/permission grant @ユーザー 権限名
```

### 権限剥奪（Admin以上）

```
/permission revoke @ユーザー 権限名
```

---

## 永続化

権限設定は `data/permissions.json` に保存されます。

```json
{
  "custom_permissions": {
    "123456789": ["Admin"]
  },
  "admins": [123456789],
  "super_users": [987654321],
  "version": 1
}
```

---

## 個人利用の推奨設定

自分だけが使う場合、以下の設定で十分です：

```bash
# .env
SUPER_USER_IDS=あなたのDiscordID
```

これで全機能を制限なく利用できます。

---

## 注意事項

- SuperUser権限は環境変数でのみ設定可能（コマンドでは不可）
- Admin権限の付与/剥奪はSuperUserのみ可能
- 一般ユーザーはデフォルトで FileRead/FileWrite/Schedule を持つ
