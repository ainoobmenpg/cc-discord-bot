//! モックLLMクライアント（テスト用）

use crate::history::ChatMessage;
use crate::tool::{SharedToolManager, ToolContext, ToolManager};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{LLMClient, LLMError};

/// テスト用モックLLMクライアント
///
/// 固定のレスポンスを返すシンプルなモック実装
pub struct MockLLMClient {
    /// 固定レスポンス
    response: String,
    /// ツールマネージャー
    tool_manager: SharedToolManager,
}

impl MockLLMClient {
    /// 新しいモッククライアントを作成
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            tool_manager: Arc::new(RwLock::new(ToolManager::new())),
        }
    }

    /// カスタムツールマネージャーで作成
    pub fn with_tools(response: impl Into<String>, tool_manager: SharedToolManager) -> Self {
        Self {
            response: response.into(),
            tool_manager,
        }
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn chat_with_tools(
        &self,
        _messages: Vec<ChatMessage>,
        _tool_context: &ToolContext,
    ) -> Result<String, LLMError> {
        Ok(self.response.clone())
    }

    fn tool_manager(&self) -> SharedToolManager {
        self.tool_manager.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Tool, ToolError, ToolResult};
    use async_trait::async_trait;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echo back the input"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Message to echo"
                    }
                },
                "required": ["message"]
            })
        }

        async fn execute(
            &self,
            params: serde_json::Value,
            _context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            let message = params["message"].as_str().unwrap_or("");
            Ok(ToolResult::success(message.to_string()))
        }
    }

    fn create_test_context() -> ToolContext {
        ToolContext::new(1, "test_user".to_string(), 1, "/tmp/test".to_string())
    }

    #[test]
    fn test_mock_llm_client_creation() {
        let client = MockLLMClient::new("Hello, World!");
        assert_eq!(client.response, "Hello, World!");
    }

    #[tokio::test]
    async fn test_mock_llm_client_returns_fixed_response() {
        let client = MockLLMClient::new("Test response");
        let messages = vec![ChatMessage::user("Any message")];
        let context = create_test_context();

        let result = client.chat_with_tools(messages, &context).await.unwrap();
        assert_eq!(result, "Test response");
    }

    #[tokio::test]
    async fn test_mock_llm_client_ignores_message_content() {
        let client = MockLLMClient::new("Fixed response");
        let context = create_test_context();

        // 異なるメッセージでも同じレスポンスを返す
        let result1 = client
            .chat_with_tools(vec![ChatMessage::user("First message")], &context)
            .await
            .unwrap();
        let result2 = client
            .chat_with_tools(vec![ChatMessage::user("Second message")], &context)
            .await
            .unwrap();

        assert_eq!(result1, result2);
        assert_eq!(result1, "Fixed response");
    }

    #[tokio::test]
    async fn test_mock_llm_client_with_custom_tool_manager() {
        let tool_manager = Arc::new(RwLock::new(ToolManager::new()));
        tool_manager.write().await.register(EchoTool);

        let client = MockLLMClient::with_tools("Response", tool_manager.clone());
        let binding = client.tool_manager.read().await;
        let tools = binding.list_tools();

        assert!(tools.contains(&"echo"));
    }

    #[test]
    fn test_mock_llm_client_trait_object() {
        // trait objectとして使用できることを確認
        let client: Arc<dyn LLMClient> = Arc::new(MockLLMClient::new("Trait test"));

        // async関数は直接呼べないので、型チェックのみ
        let _tool_manager = client.tool_manager();
    }
}
