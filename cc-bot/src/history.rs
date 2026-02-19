use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// メッセージのロール
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// チャットメッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// チャット履歴管理
#[derive(Debug, Clone)]
pub struct ChatHistory {
    messages: VecDeque<ChatMessage>,
    max_size: usize,
}

impl ChatHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// メッセージを追加
    pub fn push(&mut self, message: ChatMessage) {
        if self.messages.len() >= self.max_size {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }

    /// 全メッセージを取得
    pub fn messages(&self) -> &VecDeque<ChatMessage> {
        &self.messages
    }

    /// メッセージをVecで取得（API送信用）
    pub fn to_vec(&self) -> Vec<ChatMessage> {
        self.messages.iter().cloned().collect()
    }

    /// 履歴をクリア
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// 履歴数を取得
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 空かどうか
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// 最大サイズを取得
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

impl Default for ChatHistory {
    fn default() -> Self {
        Self::new(50) // デフォルトは50件
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");

        let msg = ChatMessage::assistant("Hi there");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_chat_history_push() {
        let mut history = ChatHistory::new(3);

        history.push(ChatMessage::user("msg1"));
        history.push(ChatMessage::user("msg2"));
        history.push(ChatMessage::user("msg3"));
        history.push(ChatMessage::user("msg4"));

        // 最大サイズを超えると古いものから削除
        assert_eq!(history.len(), 3);
        let msgs = history.to_vec();
        assert_eq!(msgs[0].content, "msg2");
        assert_eq!(msgs[2].content, "msg4");
    }

    #[test]
    fn test_chat_history_clear() {
        let mut history = ChatHistory::new(10);
        history.push(ChatMessage::user("test"));
        assert_eq!(history.len(), 1);

        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::User;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""user""#);

        let role = Role::Assistant;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""assistant""#);
    }
}
