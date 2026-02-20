//! LLM抽象化レイヤー
//!
//! 複数のLLMプロバイダーを統一的に扱うためのtraitと型定義

mod glm;
#[cfg(test)]
mod mock;

use crate::history::ChatMessage;
use crate::tool::{SharedToolManager, ToolContext};
use async_trait::async_trait;
use thiserror::Error;

// パブリックエクスポート
pub use glm::GLMClientImpl;
#[cfg(test)]
pub use mock::MockLLMClient;

/// LLMエラー
#[derive(Debug, Error)]
pub enum LLMError {
    #[error("API key not found")]
    ApiKeyMissing,

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("No response from API")]
    NoResponse,

    #[error("Tool error: {0}")]
    ToolError(String),
}

/// LLMクライアントtrait
///
/// すべてのLLMプロバイダーが実装する共通インターフェース
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// ツール付きでチャット
    ///
    /// # Arguments
    /// * `messages` - チャットメッセージ履歴
    /// * `tool_context` - ツール実行コンテキスト
    ///
    /// # Returns
    /// * `Ok(String)` - LLMからの応答テキスト
    /// * `Err(LLMError)` - エラー
    async fn chat_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tool_context: &ToolContext,
    ) -> Result<String, LLMError>;

    /// ツールマネージャーを取得
    fn tool_manager(&self) -> SharedToolManager;

    /// 履歴付きでチャット（ツールなし、互換性維持用）
    async fn chat_with_history(
        &self,
        messages: Vec<ChatMessage>,
        tool_context: &ToolContext,
    ) -> Result<String, LLMError> {
        self.chat_with_tools(messages, tool_context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_error_display() {
        let err = LLMError::ApiKeyMissing;
        assert_eq!(format!("{}", err), "API key not found");

        let err = LLMError::ApiError("Test error".to_string());
        assert_eq!(format!("{}", err), "API error: Test error");

        let err = LLMError::NoResponse;
        assert_eq!(format!("{}", err), "No response from API");
    }
}
