//! ストリーミング表示と進捗管理
//!
//! DiscordでのLLM応答ストリーミング表示とツール実行進捗表示を提供

use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Discordメッセージの最大文字数
const MAX_MESSAGE_LENGTH: usize = 2000;

/// 進捗ステータス
#[derive(Debug, Clone)]
pub enum ProgressStatus {
    /// ツール実行開始
    ToolStarting { name: String },
    /// ツール実行完了
    ToolCompleted { name: String, success: bool },
    /// 思考中
    Thinking,
    /// 応答生成中
    Generating { progress: usize },
}

impl ProgressStatus {
    /// 進捗ステータスを表示用文字列に変換
    pub fn to_display(&self) -> String {
        match self {
            ProgressStatus::ToolStarting { name } => {
                format!("🔧 {} を実行中...", name)
            }
            ProgressStatus::ToolCompleted { name, success } => {
                if *success {
                    format!("✅ {} 完了", name)
                } else {
                    format!("❌ {} 失敗", name)
                }
            }
            ProgressStatus::Thinking => "🤔 考え中...".to_string(),
            ProgressStatus::Generating { progress } => {
                format!("📝 生成中{}...", ".".repeat(*progress % 4))
            }
        }
    }
}

/// ストリーミング表示マネージャー
pub struct StreamingManager {
    /// 現在のメッセージ内容
    current_content: Arc<RwLock<String>>,
    /// 進捗情報
    progress: Arc<RwLock<Vec<ProgressStatus>>>,
    /// 最後のメッセージID
    last_message_id: Arc<RwLock<Option<u64>>>,
}

impl StreamingManager {
    /// 新しいストリーミングマネージャーを作成
    pub fn new() -> Self {
        Self {
            current_content: Arc::new(RwLock::new(String::new())),
            progress: Arc::new(RwLock::new(Vec::new())),
            last_message_id: Arc::new(RwLock::new(None)),
        }
    }

    /// 内容を追加
    pub async fn append_content(&self, chunk: &str) {
        let mut content = self.current_content.write().await;
        content.push_str(chunk);
    }

    /// 現在の内容を取得（最大2000文字）
    pub async fn get_content(&self) -> String {
        let content = self.current_content.read().await;
        if content.len() > MAX_MESSAGE_LENGTH {
            // 末尾2000文字を取得
            let start = content.len() - MAX_MESSAGE_LENGTH;
            format!("...{}", &content[start..])
        } else {
            content.clone()
        }
    }

    /// 全内容を取得
    pub async fn get_full_content(&self) -> String {
        self.current_content.read().await.clone()
    }

    /// 内容をクリア
    pub async fn clear(&self) {
        let mut content = self.current_content.write().await;
        content.clear();
        let mut progress = self.progress.write().await;
        progress.clear();
        let mut msg_id = self.last_message_id.write().await;
        *msg_id = None;
    }

    /// 進捗を追加
    pub async fn add_progress(&self, status: ProgressStatus) {
        let mut progress = self.progress.write().await;
        info!("Progress: {:?}", status);
        progress.push(status);
    }

    /// 進捗付きのメッセージを構築
    pub async fn build_message(&self) -> String {
        let content = self.current_content.read().await;
        let progress = self.progress.read().await;

        // 最新の進捗情報を取得
        let progress_text = if !progress.is_empty() {
            let last = progress.last().unwrap();
            format!("\n\n{}", last.to_display())
        } else {
            String::new()
        };

        // 結合して2000文字制限を確認
        let combined = format!("{}{}", content, progress_text);
        if combined.len() > MAX_MESSAGE_LENGTH {
            let start = combined.len() - MAX_MESSAGE_LENGTH;
            format!("...{}", &combined[start..])
        } else {
            combined
        }
    }

    /// Discordにメッセージを送信または更新
    pub async fn send_or_update(
        &self,
        http: &Http,
        channel_id: u64,
        initial_message: Option<&str>,
    ) -> Result<Message, String> {
        let content = if let Some(msg) = initial_message {
            msg.to_string()
        } else {
            self.build_message().await
        };

        let channel = ChannelId::new(channel_id);
        let mut last_msg_id = self.last_message_id.write().await;

        if let Some(msg_id) = *last_msg_id {
            // 既存メッセージを更新
            let message_id = serenity::model::id::MessageId::new(msg_id);
            let builder = serenity::builder::EditMessage::new().content(&content);
            match channel.edit_message(http, message_id, builder).await {
                Ok(msg) => {
                    debug!("Updated message: {}", msg_id);
                    Ok(msg)
                }
                Err(e) => {
                    error!("Failed to update message: {}", e);
                    // 更新に失敗したら新規送信
                    match channel.say(http, &content).await {
                        Ok(msg) => {
                            *last_msg_id = Some(msg.id.get());
                            Ok(msg)
                        }
                        Err(e2) => Err(format!("Failed to send message: {}", e2)),
                    }
                }
            }
        } else {
            // 新規メッセージ送信
            match channel.say(http, &content).await {
                Ok(msg) => {
                    *last_msg_id = Some(msg.id.get());
                    info!("Sent new message: {:?}", msg.id);
                    Ok(msg)
                }
                Err(e) => Err(format!("Failed to send message: {}", e)),
            }
        }
    }

    /// 最終メッセージを送信（進捗なし）
    pub async fn send_final(
        &self,
        http: &Http,
        channel_id: u64,
    ) -> Result<Message, String> {
        let content = self.current_content.read().await.clone();

        // 2000文字制限で分割
        let messages = split_message(&content, MAX_MESSAGE_LENGTH);
        let channel = ChannelId::new(channel_id);

        let mut last_msg = None;
        for msg_content in messages {
            match channel.say(http, &msg_content).await {
                Ok(msg) => {
                    last_msg = Some(msg);
                }
                Err(e) => {
                    error!("Failed to send final message: {}", e);
                    return Err(format!("Failed to send message: {}", e));
                }
            }
        }

        last_msg.ok_or_else(|| "No message sent".to_string())
    }
}

impl Default for StreamingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// メッセージを指定文字数で分割
pub fn split_message(content: &str, max_length: usize) -> Vec<String> {
    if content.len() <= max_length {
        return vec![content.to_string()];
    }

    let mut messages = Vec::new();
    let mut remaining = content;

    while !remaining.is_empty() {
        // 改行で区切りの良い位置を探す
        let cut_point = if remaining.len() > max_length {
            let search_end = max_length.min(remaining.len());
            if let Some(pos) = remaining[..search_end].rfind('\n') {
                pos + 1
            } else if let Some(pos) = remaining[..search_end].rfind(' ') {
                pos + 1
            } else {
                search_end
            }
        } else {
            remaining.len()
        };

        let (chunk, rest) = remaining.split_at(cut_point);
        if !chunk.is_empty() {
            messages.push(chunk.to_string());
        }
        remaining = rest;
    }

    messages
}

/// ツール実行のユーザー確認が必要かどうかを判定
pub fn requires_confirmation(tool_name: &str, confirmation_enabled: bool) -> bool {
    if !confirmation_enabled {
        return false;
    }

    // 確認が必要なツールのリスト
    const DANGEROUS_TOOLS: &[&str] = &[
        "bash",
        "execute_command",
        "delete_file",
        "write_file",
        "system",
    ];

    DANGEROUS_TOOLS.contains(&tool_name)
}

/// 確認メッセージを生成
pub fn build_confirmation_message(tool_name: &str, params: &serde_json::Value) -> String {
    format!(
        "⚠️ **ツール実行の確認**\n\n\
         ツール: `{}`\n\
         パラメータ: ```json\n{}\n```\n\n\
         このツールを実行しますか？\n\
         ✅ 実行を許可する場合は `/confirm` を入力\n\
         ❌ 拒否する場合は `/cancel` を入力",
        tool_name,
        serde_json::to_string_pretty(params).unwrap_or_else(|_| format!("{:?}", params))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_status_display() {
        let status = ProgressStatus::ToolStarting {
            name: "read_file".to_string(),
        };
        assert_eq!(status.to_display(), "🔧 read_file を実行中...");

        let status = ProgressStatus::ToolCompleted {
            name: "read_file".to_string(),
            success: true,
        };
        assert_eq!(status.to_display(), "✅ read_file 完了");

        let status = ProgressStatus::ToolCompleted {
            name: "read_file".to_string(),
            success: false,
        };
        assert_eq!(status.to_display(), "❌ read_file 失敗");

        let status = ProgressStatus::Thinking;
        assert_eq!(status.to_display(), "🤔 考え中...");

        let status = ProgressStatus::Generating { progress: 2 };
        assert_eq!(status.to_display(), "📝 生成中.....");
    }

    #[test]
    fn test_split_message_short() {
        let content = "Short message";
        let messages = split_message(content, 2000);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], content);
    }

    #[test]
    fn test_split_message_long() {
        let content = "a".repeat(5000);
        let messages = split_message(&content, 2000);
        assert!(messages.len() > 1);

        let total_len: usize = messages.iter().map(|m| m.len()).sum();
        assert_eq!(total_len, 5000);
    }

    #[test]
    fn test_split_message_with_newlines() {
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let messages = split_message(content, 15);
        // 改行で分割されることを確認
        for msg in &messages {
            assert!(msg.len() <= 15 || msg.lines().count() == 1);
        }
    }

    #[test]
    fn test_requires_confirmation_disabled() {
        assert!(!requires_confirmation("bash", false));
        assert!(!requires_confirmation("read_file", false));
    }

    #[test]
    fn test_requires_confirmation_enabled() {
        assert!(requires_confirmation("bash", true));
        assert!(requires_confirmation("write_file", true));
        assert!(requires_confirmation("delete_file", true));
        assert!(!requires_confirmation("read_file", true));
        assert!(!requires_confirmation("list_files", true));
    }

    #[tokio::test]
    async fn test_streaming_manager_append() {
        let manager = StreamingManager::new();
        manager.append_content("Hello ").await;
        manager.append_content("World").await;

        let content = manager.get_content().await;
        assert_eq!(content, "Hello World");
    }

    #[tokio::test]
    async fn test_streaming_manager_progress() {
        let manager = StreamingManager::new();
        manager
            .add_progress(ProgressStatus::ToolStarting {
                name: "test".to_string(),
            })
            .await;

        let msg = manager.build_message().await;
        assert!(msg.contains("🔧 test を実行中..."));
    }

    #[tokio::test]
    async fn test_streaming_manager_clear() {
        let manager = StreamingManager::new();
        manager.append_content("Content").await;
        manager
            .add_progress(ProgressStatus::Thinking)
            .await;

        manager.clear().await;

        let content = manager.get_content().await;
        assert!(content.is_empty());
    }
}
