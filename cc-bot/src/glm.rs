#![allow(dead_code)]

use crate::history::ChatMessage;
use crate::tool::{ToolContext, ToolDefinition, ToolManager};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

// 定数
// Coding Plan用エンドポイント
const GLM_API_URL: &str = "https://api.z.ai/api/coding/paas/v4/chat/completions";

// カスタムエラー型
#[derive(Debug, Error)]
pub enum GLMError {
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

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

/// APIレスポンスのメッセージ
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// ツール呼び出し
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// 関数呼び出し
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Clone)]
pub struct GLMClient {
    api_key: String,
    client: Client,
    model: String,
    tool_manager: Arc<RwLock<ToolManager>>,
}

impl GLMClient {
    pub fn new() -> Result<Self, GLMError> {
        let api_key = env::var("GLM_API_KEY")
            .map_err(|_| GLMError::ApiKeyMissing)?;

        let model = env::var("GLM_MODEL")
            .unwrap_or_else(|_| "glm-4.7-flash".to_string());

        info!("GLM client created with model: {}", model);

        Ok(Self {
            api_key,
            client: Client::new(),
            model,
            tool_manager: Arc::new(RwLock::new(ToolManager::new())),
        })
    }

    /// ツールマネージャーを取得
    pub fn tool_manager(&self) -> Arc<RwLock<ToolManager>> {
        self.tool_manager.clone()
    }

    /// 履歴付きでチャット（ツール対応）
    pub async fn chat_with_tools(&self, messages: Vec<ChatMessage>, context: &ToolContext) -> Result<String, GLMError> {
        let tools = {
            let manager = self.tool_manager.read().await;
            manager.get_all_definitions()
        };

        // システムメッセージを先頭に追加
        let mut all_messages = vec![ChatMessage::system(
            "あなたは日本語で応答するAIアシスタントです。\
             ユーザーが特に他言語を指定しない限り、必ず日本語で回答してください。\
             コードや技術用語はそのままで構いません。"
        )];
        all_messages.extend(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
        };

        debug!("Request: {}", serde_json::to_string(&request)?);

        let http_response = self
            .client
            .post(GLM_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = http_response.status();
        debug!("API status: {}", status);

        if !status.is_success() {
            let error_text = http_response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            let error_msg = format!("API returned {}: {}", status, error_text);
            error!("{}", error_msg);
            return Err(GLMError::ApiError(error_msg));
        }

        let response_text = http_response.text().await?;
        debug!("Response: {}", response_text);

        let chat_response: ChatResponse = serde_json::from_str(&response_text)?;

        let choice = chat_response.choices.into_iter().next().ok_or_else(|| {
            error!("No response from API");
            GLMError::NoResponse
        })?;

        // ツール呼び出しがある場合
        if let Some(tool_calls) = &choice.message.tool_calls {
            if !tool_calls.is_empty() {
                return self.handle_tool_calls(tool_calls.clone(), context).await;
            }
        }

        // 通常のテキスト応答
        choice.message.content.ok_or_else(|| {
            error!("No content in response");
            GLMError::NoResponse
        })
    }

    /// ツール呼び出しを処理
    async fn handle_tool_calls(&self, tool_calls: Vec<ToolCall>, context: &ToolContext) -> Result<String, GLMError> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
            let function_name = &tool_call.function.name;
            let arguments_str = &tool_call.function.arguments;

            debug!("Tool call: {}({})", function_name, arguments_str);

            // 引数をパース
            let arguments: serde_json::Value = serde_json::from_str(arguments_str)
                .map_err(|e| GLMError::ToolError(format!("Invalid arguments: {}", e)))?;

            // ツールを実行
            let manager = self.tool_manager.read().await;
            let result = manager.execute(function_name, arguments, context).await
                .map_err(|e| GLMError::ToolError(e.to_string()))?;

            results.push(format!("{}: {}", function_name, result.output));
        }

        Ok(format!("Tool results:\n{}", results.join("\n")))
    }

    /// 履歴付きでチャット（ツールなし、互換性維持）
    pub async fn chat_with_history(&self, messages: Vec<ChatMessage>, context: &ToolContext) -> Result<String, GLMError> {
        self.chat_with_tools(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Role;

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "glm-4.7-flash".to_string(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: "Hello".to_string(),
            }],
            tools: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""role":"user""#));
        assert!(json.contains(r#""content":"Hello""#));
        assert!(json.contains(r#""model":"glm-4.7-flash""#));
    }

    #[test]
    fn test_glm_error_display() {
        let err = GLMError::ApiKeyMissing;
        assert_eq!(format!("{}", err), "API key not found");

        let err = GLMError::ApiError("Test error".to_string());
        assert_eq!(format!("{}", err), "API error: Test error");

        let err = GLMError::NoResponse;
        assert_eq!(format!("{}", err), "No response from API");
    }

    #[test]
    fn test_api_url_constant() {
        assert_eq!(GLM_API_URL, "https://api.z.ai/api/coding/paas/v4/chat/completions");
    }
}
