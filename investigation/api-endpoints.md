# Discord APIエンドポイント一覧

## 概要

agent-discord-skillsとclaude-code-discordで使用されているDiscord APIエンドポイントをまとめる。

---

## 共通設定

### ベースURL

```
https://discord.com/api/v10
```

### 認証ヘッダー

```bash
Authorization: Bot ${DISCORD_BOT_TOKEN}
```

### コンテンツタイプ

```bash
Content-Type: application/json
```

---

## チャンネルメッセージ

### メッセージ送信

**エンドポイント**:
```
POST /channels/{channel.id}/messages
```

**URL**:
```
https://discord.com/api/v10/channels/{CHANNEL_ID}/messages
```

**リクエストボディ**:
```json
{
  "content": "Hello from Claude Code!",
  "embeds": [{
    "title": "Notification",
    "description": "This is an embed message",
    "color": 3447003
  }]
}
```

**curl例**:
```bash
curl -X POST "https://discord.com/api/v10/channels/123456789012345678/messages" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"content": "Hello from Claude Code!"}'
```

**レスポンス**:
```json
{
  "id": "987654321098765432",
  "content": "Hello from Claude Code!",
  "channel_id": "123456789012345678",
  "author": {
    "id": "bot_user_id",
    "username": "YourBot",
    "bot": true
  },
  "timestamp": "2025-10-20T12:00:00.000000+00:00"
}
```

**ステータスコード**:
- `200 OK`: 成功
- `400 Bad Request`: 無効なメッセージコンテンツ
- `401 Unauthorized`: 無効なボットトークン
- `403 Forbidden`: 権限不足
- `404 Not Found`: チャンネルが存在しない

---

### メッセージ取得

**エンドポイント**:
```
GET /channels/{channel.id}/messages
```

**URL**:
```
https://discord.com/api/v10/channels/{CHANNEL_ID}/messages?limit=50
```

**クエリパラメータ**:
- `limit`: 取得するメッセージ数（1-100、デフォルト: 50）
- `before`: このメッセージIDより前のメッセージを取得
- `after`: このメッセージIDより後のメッセージを取得
- `around`: このメッセージID付近のメッセージを取得

**curl例**:
```bash
curl -X GET "https://discord.com/api/v10/channels/123456789012345678/messages?limit=50" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
```json
[
  {
    "id": "1234567890123456789",
    "channel_id": "123456789012345678",
    "author": {
      "id": "987654321098765432",
      "username": "Username",
      "discriminator": "0000",
      "avatar": "avatar_hash"
    },
    "content": "Message text content",
    "timestamp": "2025-10-20T12:00:00.000000+00:00",
    "edited_timestamp": null,
    "tts": false,
    "mention_everyone": false,
    "mentions": [],
    "mention_roles": [],
    "attachments": [],
    "embeds": [],
    "reactions": [],
    "pinned": false,
    "type": 0
  }
]
```

**ステータスコード**:
- `200 OK`: 成功
- `401 Unauthorized`: 無効なボットトークン
- `403 Forbidden`: 権限不足
- `404 Not Found`: チャンネルが存在しない

---

## チャンネル管理

### チャンネル作成

**エンドポイント**:
```
POST /guilds/{guild.id}/channels
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}/channels
```

**リクエストボディ**:
```json
{
  "name": "general-chat",
  "type": 0,
  "topic": "General discussion for all members",
  "parent_id": "123456789012345678",
  "nsfw": false,
  "permission_overwrites": []
}
```

**チャンネルタイプ**:
- `0`: GUILD_TEXT（テキストチャンネル）
- `2`: GUILD_VOICE（ボイスチャンネル）
- `4`: GUILD_CATEGORY（カテゴリ）
- `5`: GUILD_ANNOUNCEMENT（アナウンスチャンネル）
- `13`: GUILD_STAGE_VOICE（ステージチャンネル）

**curl例**:
```bash
curl -X POST "https://discord.com/api/v10/guilds/123456789012345678/channels" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "general-chat",
    "type": 0
  }'
```

**レスポンス**:
```json
{
  "id": "987654321098765432",
  "type": 0,
  "guild_id": "123456789012345678",
  "position": 0,
  "permission_overwrites": [],
  "name": "general-chat",
  "topic": null,
  "nsfw": false,
  "last_message_id": null,
  "parent_id": null
}
```

**ステータスコード**:
- `201 Created`: 作成成功
- `400 Bad Request`: 無効なチャンネル名またはパラメータ
- `401 Unauthorized`: 無効なボットトークン
- `403 Forbidden`: Manage Channels権限不足
- `404 Not Found`: サーバーが存在しない

---

### チャンネル一覧取得

**エンドポイント**:
```
GET /guilds/{guild.id}/channels
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}/channels
```

**curl例**:
```bash
curl -X GET "https://discord.com/api/v10/guilds/123456789012345678/channels" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
```json
[
  {
    "id": "123456789012345678",
    "type": 0,
    "guild_id": "987654321098765432",
    "position": 0,
    "permission_overwrites": [],
    "name": "general",
    "topic": "General discussion",
    "nsfw": false,
    "last_message_id": "111222333444555666",
    "parent_id": null
  }
]
```

**ステータスコード**:
- `200 OK`: 成功
- `401 Unauthorized`: 無効なボットトークン
- `403 Forbidden`: 権限不足
- `404 Not Found`: サーバーが存在しない

---

### チャンネル更新

**エンドポイント**:
```
PATCH /channels/{channel.id}
```

**URL**:
```
https://discord.com/api/v10/channels/{CHANNEL_ID}
```

**リクエストボディ**:
```json
{
  "name": "new-channel-name",
  "topic": "New channel topic",
  "position": 5,
  "parent_id": "123456789012345678",
  "nsfw": false
}
```

**curl例**:
```bash
curl -X PATCH "https://discord.com/api/v10/channels/123456789012345678" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "new-channel-name",
    "topic": "New channel topic"
  }'
```

**レスポンス**:
```json
{
  "id": "123456789012345678",
  "type": 0,
  "guild_id": "987654321098765432",
  "position": 5,
  "permission_overwrites": [],
  "name": "new-channel-name",
  "topic": "New channel topic",
  "nsfw": false,
  "last_message_id": "111222333444555666",
  "parent_id": "123456789012345678"
}
```

**ステータスコード**:
- `200 OK`: 更新成功
- `400 Bad Request`: 無効なパラメータ
- `401 Unauthorized`: 無効なボットトークン
- `403 Forbidden`: Manage Channels権限不足
- `404 Not Found`: チャンネルが存在しない

---

### チャンネル削除

**エンドポイント**:
```
DELETE /channels/{channel.id}
```

**URL**:
```
https://discord.com/api/v10/channels/{CHANNEL_ID}
```

**curl例**:
```bash
curl -X DELETE "https://discord.com/api/v10/channels/123456789012345678" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
- `204 No Content`: 削除成功（レスポンスボディなし）

**ステータスコード**:
- `204 No Content`: 削除成功
- `401 Unauthorized`: 無効なボットトークン
- `403 Forbidden`: Manage Channels権限不足
- `404 Not Found`: チャンネルが存在しない

---

## サーバー（Guild）

### サーバー情報取得

**エンドポイント**:
```
GET /guilds/{guild.id}
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}
```

**curl例**:
```bash
curl -X GET "https://discord.com/api/v10/guilds/123456789012345678" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
```json
{
  "id": "123456789012345678",
  "name": "Server Name",
  "icon": "icon_hash",
  "owner_id": "987654321098765432",
  "permissions": "2147483647",
  "features": []
}
```

---

### サーバーチャンネル一覧

**エンドポイント**:
```
GET /guilds/{guild.id}/channels
```

（上記「チャンネル一覧取得」を参照）

---

## メンバー

### メンバー情報取得

**エンドポイント**:
```
GET /guilds/{guild.id}/members/{user.id}
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}/members/{USER_ID}
```

**curl例**:
```bash
curl -X GET "https://discord.com/api/v10/guilds/123456789012345678/members/987654321098765432" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
```json
{
  "user": {
    "id": "987654321098765432",
    "username": "Username",
    "discriminator": "0000",
    "avatar": "avatar_hash"
  },
  "nick": "Nickname",
  "roles": ["role_id_1", "role_id_2"],
  "joined_at": "2025-01-01T00:00:00.000000+00:00",
  "premium_since": null,
  "deaf": false,
  "mute": false,
  "flags": 0
}
```

---

### メンバーリスト取得

**エンドポイント**:
```
GET /guilds/{guild.id}/members
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}/members?limit=100
```

**クエリパラメータ**:
- `limit`: 取得するメンバー数（1-1000、デフォルト: 1）
- `after`: このメンバーID以降のメンバーを取得

**curl例**:
```bash
curl -X GET "https://discord.com/api/v10/guilds/123456789012345678/members?limit=100" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
```json
[
  {
    "user": {
      "id": "987654321098765432",
      "username": "Username",
      "discriminator": "0000",
      "avatar": "avatar_hash"
    },
    "nick": "Nickname",
    "roles": ["role_id_1"],
    "joined_at": "2025-01-01T00:00:00.000000+00:00",
    "premium_since": null,
    "deaf": false,
    "mute": false,
    "flags": 0
  }
]
```

---

## ロール

### ロール作成

**エンドポイント**:
```
POST /guilds/{guild.id}/roles
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}/roles
```

**リクエストボディ**:
```json
{
  "name": "New Role",
  "permissions": "2147483647",
  "color": 3447003,
  "hoist": false,
  "mentionable": false
}
```

**curl例**:
```bash
curl -X POST "https://discord.com/api/v10/guilds/123456789012345678/roles" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "New Role",
    "permissions": "2147483647",
    "color": 3447003
  }'
```

**レスポンス**:
```json
{
  "id": "987654321098765432",
  "name": "New Role",
  "permissions": "2147483647",
  "color": 3447003,
  "hoist": false,
  "mentionable": false
}
```

---

### ロール一覧取得

**エンドポイント**:
```
GET /guilds/{guild.id}/roles
```

**URL**:
```
https://discord.com/api/v10/guilds/{GUILD_ID}/roles
```

**curl例**:
```bash
curl -X GET "https://discord.com/api/v10/guilds/123456789012345678/roles" \
  -H "Authorization: Bot ${DISCORD_BOT_TOKEN}"
```

**レスポンス**:
```json
[
  {
    "id": "123456789012345678",
    "name": "Role Name",
    "permissions": "2147483647",
    "position": 0,
    "color": 3447003,
    "hoist": false,
    "mentionable": false
  }
]
```

---

## 権限

### 権限のビット値

| 権限 | ビット値 | 16進数 | 説明 |
|------|----------|--------|------|
| CREATE_INSTANT_INVITE | 1 | 0x1 | インスタント招待を作成 |
| KICK_MEMBERS | 2 | 0x2 | メンバーをキック |
| BAN_MEMBERS | 4 | 0x4 | メンバーをBAN |
| ADMINISTRATOR | 8 | 0x8 | 管理者 |
| MANAGE_CHANNELS | 16 | 0x10 | チャンネルを管理 |
| MANAGE_GUILD | 32 | 0x20 | サーバーを管理 |
| ADD_REACTIONS | 64 | 0x40 | リアクションを追加 |
| VIEW_AUDIT_LOG | 128 | 0x80 | 監査ログを表示 |
| PRIORITY_SPEAKER | 256 | 0x100 | 優先スピーカー |
| STREAM | 512 | 0x200 | 動画を配信 |
| READ_MESSAGES | 1024 | 0x400 | メッセージを読む |
| SEND_MESSAGES | 2048 | 0x800 | メッセージを送信 |
| SEND_TTS_MESSAGES | 4096 | 0x1000 | TTSメッセージを送信 |
| MANAGE_MESSAGES | 8192 | 0x2000 | メッセージを管理 |
| EMBED_LINKS | 16384 | 0x4000 | リンクを埋め込み |
| ATTACH_FILES | 32768 | 0x8000 | ファイルを添付 |
| READ_MESSAGE_HISTORY | 65536 | 0x10000 | メッセージ履歴を読む |
| MENTION_EVERYONE | 131072 | 0x20000 | @everyoneをメンション |
| EXTERNAL_EMOJIS | 262144 | 0x40000 | 外部絵文字を使用 |
| VIEW_GUILD_INSIGHTS | 524288 | 0x80000 | サーバーインサイトを表示 |
| CONNECT | 1048576 | 0x100000 | ボイスチャンネルに接続 |
| SPEAK | 2097152 | 0x200000 | ボイスチャンネルで発言 |
| MUTE_MEMBERS | 4194304 | 0x400000 | メンバーをミュート |
| DEAFEN_MEMBERS | 8388608 | 0x800000 | メンバーを deafen |
| MOVE_MEMBERS | 16777216 | 0x1000000 | メンバーを移動 |
| USE_VAD | 33554432 | 0x2000000 | 音声検出を使用 |
| CHANGE_NICKNAME | 67108864 | 0x4000000 | ニックネームを変更 |
| MANAGE_NICKNAMES | 134217728 | 0x8000000 | ニックネームを管理 |
| MANAGE_ROLES | 268435456 | 0x10000000 | ロールを管理 |
| MANAGE_WEBHOOKS | 536870912 | 0x20000000 | Webhookを管理 |
| MANAGE_EMOJIS_AND_STICKERS | 1073741824 | 0x40000000 | 絵文字とステッカーを管理 |
| USE_APPLICATION_COMMANDS | 2147483648 | 0x80000000 | アプリケーションコマンドを使用 |

---

## エラーコード

### 一般的なエラーコード

| コード | メッセージ | 説明 |
|------|-----------|------|
| 0 | Unknown account | 不明なエラー |
| 10001 | Unknown account | 不明なアカウント |
| 10002 | Unknown application | 不明なアプリケーション |
| 10003 | Unknown Channel | 不明なチャンネル |
| 10004 | Unknown Guild | 不明なサーバー |
| 10005 | Unknown Integration | 不明な統合 |
| 10006 | Unknown Invite | 不明な招待 |
| 10007 | Unknown Member | 不明なメンバー |
| 10008 | Unknown Message | 不明なメッセージ |
| 10009 | Unknown Overwrite | 不明な権限上書き |
| 10010 | Unknown Provider | 不明なプロバイダー |
| 10011 | Unknown Role | 不明なロール |
| 10012 | Unknown Token | 不明なトークン |
| 10013 | Unknown User | 不明なユーザー |
| 10014 | Unknown Emoji | 不明な絵文字 |
| 10015 | Unknown Webhook | 不明なWebhook |
| 10020 | Unknown Webhook Service | 不明なWebhookサービス |
| 10026 | Unknown Session | 不明なセッション |
| 10027 | Unknown Ban | 不明なBAN |
| 10029 | Unknown SKU | 不明なSKU |
| 10030 | Unknown Store Listing | 不明なストアリスティング |
| 10031 | Unknown Entitlement | 不明なエンタイトルメント |
| 10032 | Unknown Build | 不明なビルド |
| 10033 | Unknown Lobby | 不明なロビー |
| 10034 | Unknown Branch | 不明なブランチ |
| 10035 | Unknown Store Directory Layout | 不明なストアディレクトリレイアウト |
| 10036 | Unknown Redistribution Limit | 不明な再配布制限 |
| 10037 | Unknown Purchase | 不明な購入 |
| 10038 | Unknown Guild Template | 不明なサーバーテンプレート |
| 10039 | Unknown Discovery Category | 不明なディスカバリーカテゴリ |
| 10040 | Unknown Sticker | 不明なステッカー |
| 10041 | Unknown Interaction | 不明なインタラクション |
| 10042 | Unknown Application Command | 不明なアプリケーションコマンド |
| 10043 | Unknown Application Command Permissions | 不明なアプリケーションコマンド権限 |
| 10044 | Unknown Stage Instance | 不明なステージインスタンス |
| 10045 | Unknown Guild Member Verification Form | 不明なサーバーメンバー検証フォーム |
| 10046 | Unknown Guild Welcome Screen | 不明なサーバーウェルカム画面 |
| 10047 | Unknown Guild Scheduled Event | 不明なサーバースケジュールイベント |
| 10048 | Unknown Guild Scheduled Event User | 不明なサーバースケジュールイベントユーザー |
| 10049 | Unknown Tag | 不明なタグ |
| 10050 | Unknown Plugin | 不明なプラグイン |
| 10057 | Unknown Guild Onboarding | 不明なサーバーオンボーディング |
| 10060 | Unknown Home Settings | 不明なホーム設定 |
| 10062 | Unknown Voice State | 不明なボイス状態 |
| 10063 | Unknown Guild Moderation | 不明なサーバーモデレーション |
| 10066 | Unknown Directory Entry | 不明なディレクトリエントリ |
| 10067 | Unknown Auto Moderation Rule | 不明な自動モデレーションルール |
| 10068 | Unknown Auto Moderation Trigger | 不明な自動モデレーショントリガー |
| 10069 | Unknown Auto Moderation Action | 不明な自動モデレーションアクション |
| 10070 | Unknown Linked Role | 不明なリンクされたロール |
| 10071 | Unknown Entitlement | 不明なエンタイトルメント |
| 10073 | Unknown Voice Channel Status | 不明なボイスチャンネルステータス |
| 20001 | Bots cannot use this endpoint | ボットはこのエンドポイントを使用できない |
| 20002 | Only bots can use this endpoint | ボットのみがこのエンドポイントを使用できる |
| 20009 | Explicit content cannot be sent to the desired recipient(s) | 望ましい受信者にアダルトコンテンツを送信できない |
| 20012 | You are not authorized to perform this action on this application | このアプリケーションでこのアクションを実行する権限がない |
| 20016 | This action cannot be performed while a slowmode is active | スローモードが有効な間、このアクションは実行できない |
| 20018 | This build is not available in your country | このビルドはあなたの国では利用できない |
| 20022 | The lobby is full | ロビーが満員です |
| 20024 | A level is required for the rating | レーティングにレベルが必要です |
| 20025 | The guild has subscribed to this SKU already | サーバーは既にこのSKUを購読しています |
| 20026 | A transfer target must be specified | 転送ターゲットを指定する必要があります |
| 20028 | You cannot transfer ownership to a bot user | ボットユーザーに所有権を転送できない |
| 20029 | The SKU requires a guild to be subscribed | SKUにはサーバーが購読している必要があります |
| 20030 | A transfer must be to a user who is a member of the guild | 転送はサーバーのメンバーであるユーザーに行う必要があります |
| 20031 | You cannot transfer ownership to a user who is already a member of a team | すでにチームのメンバーであるユーザーに所有権を転送できない |
| 20031 | The guild is already premium | サーバーはすでにプレミアムです |
| 20033 | Maximum number of guilds reached (100) | サーバーの最大数に達しました（100） |
| 20035 | Maximum number of guilds reached (100) | サーバーの最大数に達しました（100） |
| 20036 | Premium entitlement is too low for this operation | プレミアムエンタイトルメントがこの操作に対して低すぎます |
| 20040 | Unauthorized to grant this entitlement | このエンタイトルメントを付与する権限がない |
| 20041 | Unauthorized to revoke this entitlement | このエンタイトルメントを取り消す権限がない |
| 20042 | The guild has reached its maximum number of entitlements for this SKU | サーバーはこのSKUのエンタイトルメントの最大数に達しました |
| 20044 | Entitlement was already granted | エンタイトルメントは既に付与されています |
| 20045 | Entitlement was not granted | エンタイトルメントは付与されていませんでした |
| 20046 | Cannot retrieve entitlement for a deleted SKU | 削除されたSKUのエンタイトルメントを取得できない |
| 20047 | SKU is invalid because the parent application is absent | 親アプリケーションがないためSKUが無効です |
| 20052 | The guild has already created a channel for this SKU | サーバーは既にこのSKUのチャンネルを作成しています |
| 20053 | Cannot delete a channel that is being used for an active SKU | アクティブなSKUで使用されているチャンネルを削除できない |
| 20054 | Subscription is not yet redeemable | サブスクリプションはまだ償還可能ではありません |
| 20055 | Entitlement was not purchased | エンタイトルメントは購入されていませんでした |
| 20056 | Pending entitlement is not purchasable | 保留中のエンタイトルメントは購入可能ではありません |
| 20057 | Entitlement does not exist | エンタイトルメントが存在しません |
| 20060 | Invalid duration | 無効な期間 |
| 20061 | Subscription cannot be canceled | サブスクリプションをキャンセルできない |
| 20062 | Subscription has already been canceled | サブスクリプションは既にキャンセルされています |
| 20063 | Cannot redeem gifts on behalf of other users | 他のユーザーに代わってギフトを償還できない |
| 20064 | Subscription is currently in a trialing period | サブスクリプションは現在試用期間です |
| 20065 | Subscription has already been trialed | サブスクリプションはすでに試用されています |
| 20066 | Subscription is not in a trialing period | サブスクリプションは試用期間にありません |
| 20067 | Cannot update a subscription that has been canceled | キャンセルされたサブスクリプションを更新できない |
| 20068 | Cannot renew a subscription that is not active | アクティブではないサブスクリプションを更新できない |
| 20069 | Cannot update a subscription that has not yet started | まだ開始していないサブスクリプションを更新できない |
| 20070 | Cannot transfer a subscription that is not active | アクティブではないサブスクリプションを転送できない |
| 20071 | Cannot transfer a subscription that has not yet started | まだ開始していないサブスクリプションを転送できない |
| 20074 | Cannot delete a subscription that is not active | アクティブではないサブスクリプションを削除できない |
| 20078 | Cannot delete a subscription that is not active | アクティブではないサブスクリプションを削除できない |
| 20083 | The provided trial id is invalid | 提供された試用IDが無効です |
| 20091 | Cannot access monetization endpoints | マネタイズエンドポイントにアクセスできない |
| 30001 | Invalid activity secret | 無効なアクティビティシークレット |
| 30002 | Invalid activity secret | 無効なアクティビティシークレット |
| 30003 | Rate limited | レート制限 |
| 30004 | Execution expired | 実行が期限切れ |
| 30005 | Not authenticated | 認証されていません |
| 30006 | Session is no longer valid | セッションが無効になりました |
| 30007 | The user account is in the process of being deleted | ユーザーアカウントは削除中です |
| 30008 | Invalid token | 無効なトークン |
| 30009 | The session has been invalidated | セッションが無効になりました |
| 30010 | The session has been invalidated | セッションが無効になりました |
| 30011 | Session is still active | セッションはまだアクティブです |
| 30012 | Unknown session | 不明なセッション |
| 30013 | Invalid API version | 無効なAPIバージョン |
| 30014 | Invalid token | 無効なトークン |
| 30015 | You are sending requests too quickly | リクエストを送信する速度が速すぎます |
| 30016 | Request was blocked by the internal rate limiter | リクエストが内部レート制限によってブロックされました |
| 30017 | The user is banned from this guild | ユーザーはこのサーバーからBANされています |
| 30018 | The target user is not connected to voice | 対象ユーザーはボイスに接続されていません |
| 30019 | The message was blocked by automatic moderation | メッセージは自動モデレーションによってブロックされました |
| 30020 | The message was blocked by automatic moderation | メッセージは自動モデレーションによってブロックされました |
| 30021 | The message was blocked by automatic moderation | メッセージは自動モデレーションによってブロックされました |
| 30022 | Two factor authentication is required | 二要素認証が必要です |
| 30023 | Two factor authentication is required | 二要素認証が必要です |
| 30025 | The access token is missing | アクセストークンがありません |
| 30026 | Invalid author | 無効な作者 |
| 30027 | Invalid author | 無効な作者 |
| 30028 | Invalid author | 無効な作者 |
| 30030 | Access token is missing | アクセストークンがありません |
| 30031 | Invalid authorization header for this request | このリクエストの無効な認証ヘッダー |
| 30032 | Invalid authorization header for this request | このリクエストの無効な認証ヘッダー |
| 30033 | The access token is invalid | アクセストークンが無効です |
| 30034 | The account needs to be logged out | アカウントをログアウトする必要があります |
| 30035 | The account needs to be logged out | アカウントをログアウトする必要があります |
| 30035 | The account needs to be logged out | アカウントをログアウトする必要があります |
| 30038 | Invalid author | 無効な作者 |
| 30039 | Invalid author | 無効な作者 |
| 30040 | Invalid author | 無効な作者 |
| 30042 | Invalid author | 無効な作者 |
| 30043 | Invalid author | �無効な作者 |
| 30044 | Invalid author | 無効な作者 |
| 30046 | Invalid author | 無効な作者 |
| 30047 | Invalid author | 無効な作者 |
| 30048 | Invalid author | 無効な作者 |
| 30049 | Invalid author | 無効な作者 |
| 30050 | Invalid author | 無効な作者 |
| 30051 | Invalid author | 無効な作者 |
| 30052 | Invalid author | 無効な作者 |
| 30053 | Invalid author | 無効な作者 |
| 30054 | Invalid author | 無効な作者 |
| 30055 | Invalid author | 無効な作者 |
| 30056 | Invalid author | 無効な作者 |
| 30057 | Invalid author | 無効な作者 |
| 30058 | Invalid author | 無効な作者 |
| 30059 | Invalid author | 無効な作者 |
| 30060 | Invalid author | 無効な作者 |
| 30061 | Invalid author | 無効な作者 |
| 30062 | Invalid author | 無効な作者 |
| 30063 | Invalid author | 無効な作者 |
| 30064 | Invalid author | 無効な作者 |
| 30065 | Invalid author | 無効な作者 |
| 30066 | Invalid author | 無効な作者 |
| 30067 | Invalid author | 無効な作者 |
| 30068 | Invalid author | 無効な作者 |
| 30069 | Invalid author | 無効な作者 |
| 30070 | Invalid author | 無効な作者 |
| 30071 | Invalid author | 無効な作者 |
| 30072 | Invalid author | 無効な作者 |
| 30073 | Invalid author | 無効な作者 |
| 30074 | Invalid author | 無効な作者 |
| 30075 | Invalid author | 無効な作者 |
| 30076 | Invalid author | 無効な作者 |
| 30077 | Invalid author | 無効な作者 |
| 30078 | Invalid author | 無効な作者 |
|