use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;
use tracing::{debug, error, info};

// 定数
const GLM_API_URL: &str = "https://api.z.ai/api/paas/v4/chat/completions";

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
}

// Role を enum で型安全に
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: Role,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub struct GLMClient {
    api_key: String,
    client: Client,
    model: String,
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
        })
    }

    pub async fn chat(&self, prompt: &str) -> Result<String, GLMError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: prompt.to_string(),
            }],
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

        // HTTPステータスをチェック
        if !status.is_success() {
            let error_text = http_response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            let error_msg = format!("API returned {}: {}", status, error_text);
            error!("{}", error_msg);
            return Err(GLMError::ApiError(error_msg));
        }

        let response_text = http_response.text().await?;
        debug!("Response: {}", response_text);

        let chat_response: ChatResponse = serde_json::from_str(&response_text)?;

        // choices[0] の安全性を確保
        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| {
                error!("No response from API");
                GLMError::NoResponse
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_serialization() {
        let role = Role::User;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""user""#);

        let role = Role::Assistant;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""assistant""#);

        let role = Role::System;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""system""#);
    }

    #[test]
    fn test_role_deserialization() {
        let user: Role = serde_json::from_str(r#""user""#).unwrap();
        assert!(matches!(user, Role::User));

        let assistant: Role = serde_json::from_str(r#""assistant""#).unwrap();
        assert!(matches!(assistant, Role::Assistant));

        let system: Role = serde_json::from_str(r#""system""#).unwrap();
        assert!(matches!(system, Role::System));
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "glm-4.7-flash".to_string(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: "Hello".to_string(),
            }],
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
        assert_eq!(GLM_API_URL, "https://api.z.ai/api/paas/v4/chat/completions");
    }
}
