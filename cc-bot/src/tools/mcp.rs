//! MCPツールアダプター
//!
//! MCPサーバーから提供されるツールをTool traitに適合させます。

use crate::mcp_client::MCPClient;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

/// MCPツールアダプター
pub struct MCPToolAdapter {
    client: Arc<MCPClient>,
    tool_name: String,
    description: String,
    input_schema: JsonValue,
}

impl MCPToolAdapter {
    pub fn new(
        client: Arc<MCPClient>,
        tool_name: String,
        description: String,
        input_schema: JsonValue,
    ) -> Self {
        Self {
            client,
            tool_name,
            description,
            input_schema,
        }
    }
}

#[async_trait]
impl Tool for MCPToolAdapter {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let arguments = params.as_object().cloned();

        match self.client.execute_tool(&self.tool_name, arguments).await {
            Ok(result) => {
                let output = serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|_| result.to_string());
                Ok(ToolResult::success(output))
            }
            Err(e) => {
                Err(ToolError::ExecutionFailed(format!("MCP tool error: {}", e)))
            }
        }
    }
}

/// MCP設定ファイルからツールを読み込み
pub async fn load_mcp_tools(config_path: &str) -> Result<Vec<MCPToolAdapter>, String> {
    let client = MCPClient::load(config_path)
        .map_err(|e| format!("Failed to load MCP config: {}", e))?;

    let client = Arc::new(client);

    // ツール一覧を更新
    client.refresh_all_tools().await
        .map_err(|e| format!("Failed to refresh MCP tools: {}", e))?;

    let tools = client.list_all_tools().await;

    let adapters: Vec<MCPToolAdapter> = tools
        .into_iter()
        .map(|tool| {
            MCPToolAdapter::new(
                client.clone(),
                tool.name,
                tool.description,
                tool.input_schema,
            )
        })
        .collect();

    Ok(adapters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_adapter_creation() {
        let client = Arc::new(MCPClient::new());
        let adapter = MCPToolAdapter::new(
            client,
            "mcp_exa_web_search".to_string(),
            "Search the web".to_string(),
            json!({"type": "object"}),
        );

        assert_eq!(adapter.name(), "mcp_exa_web_search");
        assert_eq!(adapter.description(), "Search the web");
    }
}
