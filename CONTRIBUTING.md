# 貢献ガイド - CC-Discord-Bot

バグ報告、機能リクエスト、コードの改善など、貢献を歓迎します！

---

## 貢献の方法

### 1. Issueを開く

バグ報告や機能リクエストは、まずIssueで議論してください。

#### バグ報告のテンプレート

```markdown
## バグの説明
何が起きましたか？

## 再現手順
1. '...' に行く
2. '...' をクリック
3. エラーが発生

## 期待する動作
どうなるべきでしたか？

## スクリーンショット
もし可能なら、スクリーンショットを追加してください。

## 環境
- OS:
- Rustバージョン:
- ボットバージョン:

## 追加の context
ここにその他の context を追加してください。
```

#### 機能リクエストのテンプレート

```markdown
## 機能の説明
あなたの機能リクエストについて説明してください。

## 解決する問題
この機能が解決する問題は何ですか？

## 望まれる解決策
どのような解決策を望みますか？

## 代替案
他にどのような解決策を検討しましたか？

## 追加の context
ここにその他の context やスクリーンショットを追加してください。
```

---

### 2. Forkしてプルリクエストを送る

#### 開発環境のセットアップ

```bash
# 1. リポジトリをForkしてクローン
git clone https://github.com/ainoobmenpg/cc-discord-bot.git
cd cc-discord-bot

# 2. Rustのインストール（まだの場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 3. 環境変数の設定
cp .env.example .env
# .env を編集して認証情報を設定

# 4. テスト実行
cd cc-bot
cargo test

# 5. ビルド
cargo build
```

---

## 開発のワークフロー

### 1. ブランチを切る

```bash
git checkout -b feature/your-feature-name
# または
git checkout -b fix/your-bug-fix
```

### 2. コードを書く

#### コーディング規約

- **エラーハンドリング**: `thiserror` クレートを使用
  ```rust
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum MyError {
      #[error("API error: {0}")]
      ApiError(String),
  }
  ```

- **ロギング**: `tracing` クレートを使用
  ```rust
  use tracing::{info, debug, error};

  info!("Starting bot...");
  error!("Failed to send message: {}", e);
  ```

- **テスト**: 変更を加えた場合はテストを書く
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_something() {
          assert_eq!(2 + 2, 4);
      }
  }
  ```

- **フォーマット**: Rust標準のフォーマットを使用
  ```bash
  cargo fmt
  ```

- **リント**: Clippyを使用
  ```bash
  cargo clippy
  ```

### 3. テストを書く

```bash
cd cc-bot
cargo test
```

すべてのテストがパスすることを確認してください。

### 4. コミットする

コミットメッセージは分かりやすく：

```bash
git add .
git commit -m "Add: スケジューラー機能の実装"

# または
git commit -m "Fix: GLM APIのエラーハンドリングを改善"
```

### 5. プッシュしてプルリクエストを送る

```bash
git push origin feature/your-feature-name
```

その後、GitHubでプルリクエストを作成してください。

---

## プルリクエストのチェックリスト

プルリクエストを送る前に、以下を確認してください：

- [ ] コードが `cargo fmt` でフォーマットされている
- [ ] `cargo clippy` で警告がない
- [ ] `cargo test` ですべてのテストがパスする
- [ ] 新しい機能にはテストが含まれている
- [ ] ドキュメント（README.md、PROGRESS.mdなど）を更新している
- [ ] コミットメッセージが分かりやすい

---

## 開発のヒント

### ローカルでのテスト

```bash
# ボットを起動
./run.sh

# テストチャンネルで試す
!ask テストメッセージ
```

### デバッグ

```bash
# ログを確認
tail -f cc-bot/bot.log

# または、環境変数でログレベルを設定
export RUST_LOG=debug
./run.sh
```

### テストの追加

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

---

## プロジェクトの構造

```
cc-bot/
├── src/
│   ├── main.rs      # Discord Bot本体
│   └── glm.rs       # GLM APIクライアント
└── Cargo.toml       # 依存関係
```

### main.rs

- Discord Botのエントリーポイント
- イベントハンドラー
- コマンド処理

### glm.rs

- GLM-4.7 APIとの通信
- エラーハンドリング
- データシリアライゼーション

---

## よくある質問

### Q: 新しいコマンドを追加するには？

A: `main.rs` の `message()` ハンドラーに新しい条件を追加してください：

```rust
if msg.content.starts_with("!mycommand") {
    // コマンドの処理
}
```

### Q: 新しい依存関係を追加するには？

A: `cargo add` コマンドを使用してください：

```bash
cd cc-bot
cargo add crate-name
```

### Q: エラーハンドリングを追加するには？

A: `thiserror` を使用してカスタムエラー型を定義してください：

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Something went wrong: {0}")]
    CustomError(String),
}
```

---

## 行動規範

このプロジェクトでは、すべての貢献者に対して敬意を持つことを期待しています。

- 建設的なフィードバックをする
- 他の貢献者を尊重する
- 異なる意見を受け入れる

---

## サポート

質問がある場合は、Issueを開くか、Discordで メンテナー にメッセージを送ってください。

---

ありがとうございます！貢献をお待ちしています 🎉
