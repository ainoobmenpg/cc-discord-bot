//! Memory tools for storing and retrieving information.
//!
//! Provides `remember` and `recall` tools that integrate with MemoryStore.

use crate::memory_store::{MemoryStore, NewMemory};
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Tool for storing information in persistent memory.
pub struct RememberTool {
    memory_store: Arc<MemoryStore>,
}

impl RememberTool {
    /// Creates a new RememberTool with the given MemoryStore.
    pub fn new(memory_store: Arc<MemoryStore>) -> Self {
        Self { memory_store }
    }
}

#[async_trait]
impl Tool for RememberTool {
    fn name(&self) -> &str {
        "remember"
    }

    fn description(&self) -> &str {
        "Store information in persistent memory. The information will be saved with a key and can be retrieved later using the recall tool."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "A unique key/tag to categorize this memory (e.g., 'favorite_color', 'meeting_notes')"
                },
                "value": {
                    "type": "string",
                    "description": "The information to remember"
                }
            },
            "required": ["key", "value"]
        })
    }

    async fn execute(&self, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let key = params["key"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'key' parameter".to_string())
        })?;

        let value = params["value"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'value' parameter".to_string())
        })?;

        if key.trim().is_empty() {
            return Err(ToolError::InvalidParams("Key cannot be empty".to_string()));
        }

        debug!("Remembering for user {}: {} = {}", context.user_id, key, value);

        // key-value形式をcontentに保存: "[key] value"
        let content = format!("[{}] {}", key, value);

        match self.memory_store.add_memory(NewMemory {
            user_id: context.user_id,
            content: content.clone(),
            ..Default::default()
        }) {
            Ok(memory) => {
                info!("Saved memory {} for user {}", memory.id, context.user_id);
                Ok(ToolResult::success(format!(
                    "Remembered: {} = {} (ID: {})",
                    key, value, memory.id
                )))
            }
            Err(e) => {
                error!("Failed to save memory: {}", e);
                Err(ToolError::ExecutionFailed(format!("Failed to save memory: {}", e)))
            }
        }
    }
}

/// Tool for retrieving stored information from memory.
pub struct RecallTool {
    memory_store: Arc<MemoryStore>,
}

impl RecallTool {
    /// Creates a new RecallTool with the given MemoryStore.
    pub fn new(memory_store: Arc<MemoryStore>) -> Self {
        Self { memory_store }
    }

    /// contentからkeyとvalueを抽出
    /// Format: "[key] value"
    fn parse_content(content: &str) -> (&str, &str) {
        if content.starts_with('[') {
            if let Some(close_bracket) = content.find(']') {
                let key = &content[1..close_bracket];
                let value = content[close_bracket + 1..].trim();
                return (key, value);
            }
        }
        // フォーマット外の場合はそのまま返す
        ("note", content)
    }
}

#[async_trait]
impl Tool for RecallTool {
    fn name(&self) -> &str {
        "recall"
    }

    fn description(&self) -> &str {
        "Retrieve stored information from memory. You can search by query string or list all memories."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find matching memories (searches in key and value)"
                }
            }
        })
    }

    async fn execute(&self, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError> {
        // クエリが指定されている場合
        if let Some(query) = params["query"].as_str() {
            if !query.trim().is_empty() {
                debug!("Searching memories for user {} with query: {}", context.user_id, query);

                match self.memory_store.search_memories(context.user_id, query) {
                    Ok(memories) => {
                        if memories.is_empty() {
                            return Ok(ToolResult::success(format!(
                                "No memories found matching: {}",
                                query
                            )));
                        }

                        let list: Vec<String> = memories
                            .iter()
                            .map(|m| {
                                let (key, value) = Self::parse_content(&m.content);
                                format!("- [{}] {}: {}", m.id, key, value)
                            })
                            .collect();
                        Ok(ToolResult::success(format!(
                            "Found {} memories:\n{}",
                            memories.len(),
                            list.join("\n")
                        )))
                    }
                    Err(e) => {
                        error!("Failed to search memories: {}", e);
                        Err(ToolError::ExecutionFailed(format!("Failed to search memories: {}", e)))
                    }
                }
            } else {
                // 空のクエリは全件取得
                self.list_all_memories(context).await
            }
        }
        // パラメータなしの場合は全件取得
        else {
            self.list_all_memories(context).await
        }
    }
}

impl RecallTool {
    async fn list_all_memories(&self, context: &ToolContext) -> Result<ToolResult, ToolError> {
        debug!("Listing all memories for user {}", context.user_id);

        match self.memory_store.list_memories(context.user_id, 20) {
            Ok(memories) => {
                if memories.is_empty() {
                    Ok(ToolResult::success("No memories stored yet.".to_string()))
                } else {
                    let list: Vec<String> = memories
                        .iter()
                        .map(|m| {
                            let (key, value) = Self::parse_content(&m.content);
                            format!("- [{}] {}: {}", m.id, key, value)
                        })
                        .collect();
                    Ok(ToolResult::success(format!(
                        "You have {} memories:\n{}",
                        memories.len(),
                        list.join("\n")
                    )))
                }
            }
            Err(e) => {
                error!("Failed to list memories: {}", e);
                Err(ToolError::ExecutionFailed(format!("Failed to list memories: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> Arc<MemoryStore> {
        let store = MemoryStore::new().expect("Failed to create test store");
        Arc::new(store)
    }

    fn create_test_context() -> ToolContext {
        ToolContext::new(12345, "test_user".to_string(), 67890, "output".to_string())
    }

    #[test]
    fn test_remember_tool_name() {
        let store = create_test_store();
        let tool = RememberTool::new(store);
        assert_eq!(tool.name(), "remember");
    }

    #[test]
    fn test_recall_tool_name() {
        let store = create_test_store();
        let tool = RecallTool::new(store);
        assert_eq!(tool.name(), "recall");
    }

    #[test]
    fn test_remember_tool_parameters_schema() {
        let store = create_test_store();
        let tool = RememberTool::new(store);
        let schema = tool.parameters_schema();

        assert!(schema["properties"]["key"].is_object());
        assert!(schema["properties"]["value"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("key")));
        assert!(schema["required"].as_array().unwrap().contains(&json!("value")));
    }

    #[tokio::test]
    async fn test_remember_saves_memory() {
        let store = create_test_store();
        let tool = RememberTool::new(store.clone());
        let ctx = create_test_context();

        let result = tool
            .execute(json!({"key": "favorite_color", "value": "blue"}), &ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("favorite_color"));
        assert!(result.output.contains("blue"));

        // Verify it was saved
        let memories = store.list_memories(12345, 10).unwrap();
        assert_eq!(memories.len(), 1);
        assert!(memories[0].content.contains("favorite_color"));
        assert!(memories[0].content.contains("blue"));
    }

    #[tokio::test]
    async fn test_remember_missing_key() {
        let store = create_test_store();
        let tool = RememberTool::new(store);
        let ctx = create_test_context();

        let result = tool.execute(json!({"value": "test"}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remember_missing_value() {
        let store = create_test_store();
        let tool = RememberTool::new(store);
        let ctx = create_test_context();

        let result = tool.execute(json!({"key": "test"}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remember_empty_key() {
        let store = create_test_store();
        let tool = RememberTool::new(store);
        let ctx = create_test_context();

        let result = tool.execute(json!({"key": "   ", "value": "test"}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recall_by_query() {
        let store = create_test_store();

        // Save some memories
        store.add_memory(NewMemory {
            user_id: 12345,
            content: "shopping list: buy milk".to_string(),
            ..Default::default()
        }).unwrap();
        store.add_memory(NewMemory {
            user_id: 12345,
            content: "work: meeting notes".to_string(),
            ..Default::default()
        }).unwrap();
        store.add_memory(NewMemory {
            user_id: 12345,
            content: "personal: gym at 5pm".to_string(),
            ..Default::default()
        }).unwrap();

        let tool = RecallTool::new(store);
        let ctx = create_test_context();

        // 前方一致検索: "shopping"で始まるコンテンツを検索
        let result = tool.execute(json!({"query": "shopping"}), &ctx).await.unwrap();

        assert!(!result.is_error, "Result should not be an error");
        assert!(result.output.contains("shopping"), "Output should contain 'shopping'");
        assert!(result.output.contains("buy milk"), "Output should contain 'buy milk'");
    }

    #[tokio::test]
    async fn test_recall_by_query_no_results() {
        let store = create_test_store();
        let tool = RecallTool::new(store);
        let ctx = create_test_context();

        let result = tool.execute(json!({"query": "nonexistent"}), &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("No memories found"));
    }

    #[tokio::test]
    async fn test_recall_list_all() {
        let store = create_test_store();

        // Save some memories
        store.add_memory(NewMemory {
            user_id: 12345,
            content: "[key1] value1".to_string(),
            ..Default::default()
        }).unwrap();
        store.add_memory(NewMemory {
            user_id: 12345,
            content: "[key2] value2".to_string(),
            ..Default::default()
        }).unwrap();

        let tool = RecallTool::new(store);
        let ctx = create_test_context();

        let result = tool.execute(json!({}), &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("2 memories"));
        assert!(result.output.contains("key1"));
        assert!(result.output.contains("key2"));
    }

    #[tokio::test]
    async fn test_recall_list_all_empty() {
        let store = create_test_store();
        let tool = RecallTool::new(store);
        let ctx = create_test_context();

        let result = tool.execute(json!({}), &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("No memories stored"));
    }

    #[tokio::test]
    async fn test_recall_is_user_isolated() {
        let store = create_test_store();

        // Save memory for one user (前方一致検索用にコンテンツの先頭にキーワードを配置)
        store.add_memory(NewMemory {
            user_id: 12345,
            content: "secret: my secret data".to_string(),
            ..Default::default()
        }).unwrap();
        store.add_memory(NewMemory {
            user_id: 99999,
            content: "secret: other secret data".to_string(),
            ..Default::default()
        }).unwrap();

        let tool = RecallTool::new(store);
        let ctx = create_test_context(); // user_id: 12345

        // 前方一致検索: "secret"で始まるコンテンツを検索
        let result = tool.execute(json!({"query": "secret"}), &ctx).await.unwrap();

        // Should only return memories for user 12345
        assert!(result.output.contains("my secret data"), "Should contain user 12345's secret data");
        assert!(!result.output.contains("other secret data"), "Should not contain user 99999's secret data");
    }

    #[test]
    fn test_parse_content_with_key() {
        let content = "[favorite_color] blue";
        let (key, value) = RecallTool::parse_content(content);
        assert_eq!(key, "favorite_color");
        assert_eq!(value, "blue");
    }

    #[test]
    fn test_parse_content_without_key() {
        let content = "just some text";
        let (key, value) = RecallTool::parse_content(content);
        assert_eq!(key, "note");
        assert_eq!(value, "just some text");
    }
}
