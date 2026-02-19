# プロジェクト方針 - cc-discord-bot

更新日: 2026-02-19

---

## 目標

**Discordで使えるAIボットを作る**

Discord Botとして機能し、LLM（GLM-4.7）とやり取りできるようにする。

---

## 実現したい機能

### 1. Discordでチャットできる ✅
- Discordからメッセージを送る → GLM-4.7が応答
- `!ask` コマンドで質問に回答

### 2. スケジュール起動ができる ⏳
- 定期的にタスクを実行
- cron式のスケジュール設定
- 結果をDiscordに通知

### 3. エージェントから話しかけてくれる ⏳
- ボットが自発的にメッセージを送ってくる
- 非同期な通知
- イベントドリブンな通知

---

## 技術スタック

### Rust + Serenity + GLM-4.7 API

**選択理由**:
1. **型安全性**: バグを防ぐ
2. **パフォーマンス**: 高速な実行、メモリ効率が良い
3. **エラーハンドリング**: `Result<T, E>` で明示的なエラー処理
4. **並列処理**: 並列処理が得意
5. **シンプルさ**: 単一言語で完結

**使用ライブラリ**:
- **Serenity**: 機能豊富なDiscordライブラリ
- **reqwest**: HTTPクライアント（GLM API呼び出し用）
- **tokio**: 非同期ランタイム
- **thiserror**: エラーハンドリング
- **tracing**: ロギング

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    Discord                              │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│              Rust Discord Bot                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Serenity                                          │ │
│  │  - メッセージ受信                                   │ │
│  │  - コマンド処理 (!ask)                             │ │
│  │  - レスポンス送信                                   │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↓                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  GLM Client (glm.rs)                              │ │
│  │  - GLM-4.7 API呼び出し                              │ │
│  │  - エラーハンドリング                                │ │
│  └────────────────────────────────────────────────────┘ │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│          GLM-4.7 API (Z.ai)                            │
└─────────────────────────────────────────────────────────┘
```

---

## プロジェクト構成

```
cc-discord-bot/
├── .env                  # 環境変数（Git管理外）
├── .env.example          # 環境変数のテンプレート（Git管理）
├── run.sh                # ボット起動スクリプト
├── .gitignore            # Git除外設定
├── README.md             # ユーザー向けドキュメント
├── PROGRESS.md           # 進捗記録
├── PROJECT_DIRECTION.md  # このファイル
│
├── cc-bot/               # Rust Discord Bot
│   ├── Cargo.toml        # 依存関係
│   ├── src/
│   │   ├── main.rs       # Discord Bot本体
│   │   └── glm.rs        # GLM APIクライアント
│   └── target/           # コンパイル済みバイナリ（Git管理外）
│
├── investigation/        # 調査結果（過去の検討）
│   ├── agent-discord-skills-analysis.md
│   ├── claude-code-discord-analysis.md
│   ├── implementation-patterns.md
│   ├── api-endpoints.md
│   ├── architecture-comparison.md
│   └── claude-agent-sdk-analysis.md
│
└── references/           # 参考リポジトリ（過去の検討）
    ├── agent-discord-skills/
    └── claude-code-discord/
```

---

## 実装のステップ

### ✅ Phase 1: 基本実装（完了）

**目的**: Discord Bot + GLM-4.7 APIの統合

**完了したタスク**:
1. ✅ Rustプロジェクト作成
2. ✅ SerenityによるDiscord Bot実装
3. ✅ GLM-4.7 APIクライアント実装
4. ✅ `!ask` コマンド実装
5. ✅ エラーハンドリング（`thiserror`）
6. ✅ ロギング（`tracing`）
7. ✅ テスト追加
8. ✅ セキュリティ対策（`.env`方式）
9. ✅ コードレビュー完了

**依存関係**:
```toml
[dependencies]
serenity = { version = "0.12", features = ["client", "gateway", "rustls_backend"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

### ⏳ Phase 2: スケジューラー（未実装）

**目的**: 定期的にGLM-4.7を実行してDiscordに通知

**計画中のタスク**:
1. ライブラリ選定
   - **tokio-cron-scheduler**: Tokioベースのスケジューラー
   - **cron**: シンプルなcronライブラリ

2. 実装
   - スケジュールの登録
   - 定期実行
   - Discord通知

3. コマンド（予定）
   - `/schedule add`: スケジュール追加
   - `/schedule list`: スケジュール一覧
   - `/schedule remove`: スケジュール削除

---

### ⏳ Phase 3: エージェント通知（未実装）

**目的**: ボットから自発的に通知を送る

**計画中のタスク**:
1. 通知方法の検討
   - 定時タスクの結果通知
   - イベントベースの通知
   - 外部トリガー

2. 実装
   - 通知キューの管理
   - Discordへの送信

---

## アーキテクチャの変更履歴

### 2026-02-19: Claude Agent SDK → GLM-4.7 API に変更

**変更前の計画**:
- Rust + Node.js の2層アーキテクチャ
- Claude Agent SDK (Node.js) を HTTP API でラップ
- Rust Bot → Node.js API → Claude Code

**変更後の実装**:
- Rustのみのシンプルなアーキテクチャ
- GLM-4.7 APIを直接呼び出し
- Rust Bot → GLM-4.7 API

**理由**:
1. **APIキーの問題**: Claude Agent SDKにAnthropic APIキーが必要
2. **既存のリソース**: GLM-4.7 Flash（無料版）を既に使用可能
3. **シンプルさ**: 単一言語で完結し、メンテナンスが容易
4. **学習目的**: Rust + 非同期処理の学習に適している

**削除した計画**:
- ~~Claude Code API（Node.js）~~
- ~~HTTP APIサーバー~~
- ~~WebSocketストリーミング~~

---

## 環境変数

### .env ファイル

```bash
# GLM API Configuration
GLM_API_KEY=your-glm-api-key-here
GLM_MODEL=glm-4.7-flash

# Discord Bot Configuration
DISCORD_BOT_TOKEN=your-discord-bot-token-here
```

**セキュリティ**:
- ✅ `.env` は `.gitignore` で除外
- ✅ `.env.example` でテンプレートを提供
- ⚠️ 決して `.env` をコミットしないでください

---

## 開発環境

### 必要なツール

1. **Rustツールチェーン**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Discord Developer Account**
   - https://discord.com/developers/applications

### MESSAGE CONTENT INTENTの設定

[Discord Developer Portal](https://discord.com/developers/applications) で:
1. ボットを選択
2. 「Bot」タブ
3. 「MESSAGE CONTENT INTENT」をONにする

---

## テスト計画

### ✅ Unit Test（完了）
- GLMクライアントのテスト
- Role列挙型のシリアライゼーションテスト
- エラーハンドリングのテスト

### ⏳ Integration Test（未実装）
- Discord Bot ↔ GLM APIの通信テスト
- エラーハンドリングの確認

### ✅ Manual Test（完了）
- 実際のDiscordサーバーでテスト
- `!ask` コマンドの動作確認

---

## リリース計画

### ✅ v0.1.0 - MVP（完了）
- Discordでチャットできる
- `!ask` コマンド
- エラーハンドリング
- ロギング
- セキュリティ対策

### ⏳ v0.2.0 - スケジューラー（計画中）
- スケジュール実行
- cron式のサポート
- スケジュール管理コマンド

### ⏳ v0.3.0 - エージェント通知（計画中）
- 自発的な通知
- イベントベースの通知

---

## 参考リソース

### Rust
- [Rust公式ドキュメント](https://doc.rust-lang.org/)
- [Serenity](https://github.com/serenity-rs/serenity)
- [tokio](https://tokio.rs/)
- [reqwest](https://docs.rs/reqwest/)

### GLM API
- [GLM-4.7 ドキュメント](https://docs.z.ai/guides/llm/glm-4)
- [Z.ai コンソール](https://console.z.ai/)

### Discord
- [Discord Developer Portal](https://discord.com/developers/applications)
- [Discord API Documentation](https://discord.com/developers/docs/reference)

---

## 次のステップ

1. **スケジューラーの実装**
   - tokio-cron-scheduler の調査
   - スケジュール管理コマンドの設計
   - 定期実行の実装

2. **エージェント通知の検討**
   - 通知方法の設計
   - イベントハンドリングの検討

3. **テストの拡充**
   - 統合テストの追加
   - エッジケースのテスト

---

## ライセンス

MIT License

---

## 作者

ainoobmenpg
