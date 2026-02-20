use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// ツール実行コンテキスト
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub user_id: u64,
    pub user_name: String,
    pub channel_id: u64,
    pub base_output_dir: String,
    /// カスタム出力サブディレクトリ（ユーザー設定から取得）
    pub custom_output_subdir: Option<String>,
}

impl ToolContext {
    pub fn new(user_id: u64, user_name: String, channel_id: u64, base_output_dir: String) -> Self {
        Self {
            user_id,
            user_name,
            channel_id,
            base_output_dir,
            custom_output_subdir: None,
        }
    }

    /// カスタムサブディレクトリを指定して作成
    pub fn with_custom_subdir(mut self, subdir: impl Into<String>) -> Self {
        self.custom_output_subdir = Some(subdir.into());
        self
    }

    /// ユーザー設定から出力先を設定して作成
    pub fn with_user_settings(mut self, output_subdir: Option<&str>) -> Self {
        self.custom_output_subdir = output_subdir.map(|s| s.to_string());
        self
    }

    /// ユーザー固有の出力ディレクトリを生成
    /// 優先順位:
    /// 1. カスタムサブディレクトリが設定されている場合: {base_output_dir}/{custom_subdir}/
    /// 2. デフォルト: {base_output_dir}/{日付}/{ユーザー名}_{ユーザーID}/
    pub fn get_user_output_dir(&self) -> String {
        // カスタムサブディレクトリが設定されている場合はそれを使用
        if let Some(ref subdir) = self.custom_output_subdir {
            // サブディレクトリ名をサニタイズ
            let safe_subdir = subdir
                .replace('/', "_")
                .replace('\\', "_")
                .replace(':', "_")
                .replace('*', "_")
                .replace('?', "_")
                .replace('"', "_")
                .replace('<', "_")
                .replace('>', "_")
                .replace('|', "_");
            return format!("{}/{}", self.base_output_dir, safe_subdir);
        }

        // デフォルト: 日付ベースのパス
        let today = Utc::now().format("%Y-%m-%d").to_string();
        // ファイルシステムで問題になる文字のみ置換（日本語は許可）
        let safe_name = self.user_name
            .replace('/', "_")
            .replace('\\', "_")
            .replace(':', "_")
            .replace('*', "_")
            .replace('?', "_")
            .replace('"', "_")
            .replace('<', "_")
            .replace('>', "_")
            .replace('|', "_");
        format!("{}/{}/{}_{}", self.base_output_dir, today, safe_name, self.user_id)
    }

    /// ユーザーIDベースの出力ディレクトリを生成（シンプル版）
    /// format: {base_output_dir}/{user_id}/
    pub fn get_user_id_output_dir(&self) -> String {
        format!("{}/{}", self.base_output_dir, self.user_id)
    }
}

/// ツール実行エラー
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// ツール実行結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            is_error: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            output: message.into(),
            is_error: true,
        }
    }
}

/// ツール定義（GLM API用）
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}

/// Tool trait - すべてのツールが実装する
#[async_trait]
pub trait Tool: Send + Sync {
    /// ツール名
    fn name(&self) -> &str;

    /// ツールの説明（LLM用）
    fn description(&self) -> &str;

    /// パラメータスキーマ（JSON Schema）
    fn parameters_schema(&self) -> JsonValue;

    /// ツール実行（コンテキスト付き）
    async fn execute(&self, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError>;

    /// ToolDefinitionを生成
    fn to_definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: self.name().to_string(),
                description: self.description().to_string(),
                parameters: self.parameters_schema(),
            },
        }
    }
}

/// ツールマネージャー
pub struct ToolManager {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// ツールを登録
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);
        self.tools.insert(name, Arc::new(tool));
    }

    /// ツールを取得
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// 全ツールの定義を取得（GLM API用）
    pub fn get_all_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.to_definition()).collect()
    }

    /// ツールを実行
    pub async fn execute(&self, name: &str, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let tool = self.tools.get(name).ok_or_else(|| {
            error!("Tool not found: {}", name);
            ToolError::NotFound(name.to_string())
        })?;

        debug!("Executing tool: {} with params: {:?}", name, params);
        let result = tool.execute(params, context).await;

        match &result {
            Ok(r) => debug!("Tool {} result: {}", name, r.output),
            Err(e) => error!("Tool {} error: {}", name, e),
        }

        result
    }

    /// 登録されているツール名一覧
    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 共有状態で使用するToolManager
pub type SharedToolManager = Arc<RwLock<ToolManager>>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        fn parameters_schema(&self) -> JsonValue {
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input string"
                    }
                },
                "required": ["input"]
            })
        }

        async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
            let input = params["input"].as_str().ok_or_else(|| {
                ToolError::InvalidParams("Missing 'input' parameter".to_string())
            })?;
            Ok(ToolResult::success(format!("Echo: {}", input)))
        }
    }

    fn create_test_context() -> ToolContext {
        ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
    }

    #[test]
    fn test_tool_context_output_dir() {
        let ctx = create_test_context();
        let output_dir = ctx.get_user_output_dir();
        assert!(output_dir.contains("test_user_123"));
        assert!(output_dir.contains("output"));
    }

    #[test]
    fn test_tool_context_japanese_name() {
        let ctx = ToolContext::new(456, "太郎".to_string(), 789, "output".to_string());
        let output_dir = ctx.get_user_output_dir();
        assert!(output_dir.contains("太郎_456"));
    }

    #[test]
    fn test_tool_context_special_chars() {
        let ctx = ToolContext::new(789, "test/user:name".to_string(), 123, "output".to_string());
        let output_dir = ctx.get_user_output_dir();
        // パス区切りの「/」は含まれるが、ユーザー名部分には含まれない
        assert!(!output_dir.contains("test/user"));
        assert!(!output_dir.contains(":name"));
        assert!(output_dir.contains("test_user_name_789"));
    }

    #[test]
    fn test_tool_definition() {
        let tool = MockTool;
        let def = tool.to_definition();

        assert_eq!(def.tool_type, "function");
        assert_eq!(def.function.name, "mock_tool");
        assert_eq!(def.function.description, "A mock tool for testing");
    }

    #[test]
    fn test_tool_manager_register() {
        let mut manager = ToolManager::new();
        manager.register(MockTool);

        assert!(manager.get("mock_tool").is_some());
        assert!(manager.get("unknown").is_none());
    }

    #[test]
    fn test_tool_manager_list() {
        let mut manager = ToolManager::new();
        manager.register(MockTool);

        let tools = manager.list_tools();
        assert_eq!(tools.len(), 1);
        assert!(tools.contains(&"mock_tool"));
    }

    #[tokio::test]
    async fn test_tool_execute() {
        let mut manager = ToolManager::new();
        manager.register(MockTool);
        let ctx = create_test_context();

        let result = manager
            .execute("mock_tool", json!({"input": "hello"}), &ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.output, "Echo: hello");
    }

    #[tokio::test]
    async fn test_tool_execute_not_found() {
        let manager = ToolManager::new();
        let ctx = create_test_context();

        let result = manager.execute("unknown", json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_context_with_custom_subdir() {
        let ctx = ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
            .with_custom_subdir("my_custom_dir");

        let output_dir = ctx.get_user_output_dir();
        assert_eq!(output_dir, "output/my_custom_dir");
        // カスタムサブディレクトリ使用時は日付やユーザー名が含まれない
        assert!(!output_dir.contains("test_user"));
        assert!(!output_dir.contains("123"));
    }

    #[test]
    fn test_tool_context_custom_subdir_sanitization() {
        let ctx = ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
            .with_custom_subdir("dir/with:bad*chars?");

        let output_dir = ctx.get_user_output_dir();
        // サブディレクトリ内の危険な文字がサニタイズされている
        // base_output_dirとサブディレクトリの間の / は残るが、
        // 入力内の / も _ に置換されるため "dir_with" になる
        assert_eq!(output_dir, "output/dir_with_bad_chars_");
        // 元の文字列にあった危険な文字はサニタイズされている
        assert!(!output_dir.contains("dir/with"));
        assert!(!output_dir.contains(":bad"));
    }

    #[test]
    fn test_tool_context_with_user_settings_some() {
        let ctx = ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
            .with_user_settings(Some("user_custom_path"));

        let output_dir = ctx.get_user_output_dir();
        assert_eq!(output_dir, "output/user_custom_path");
    }

    #[test]
    fn test_tool_context_with_user_settings_none() {
        let ctx = ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
            .with_user_settings(None);

        let output_dir = ctx.get_user_output_dir();
        // Noneの場合はデフォルトのパス生成が使用される
        assert!(output_dir.contains("test_user_123"));
        assert!(output_dir.contains("output"));
    }

    #[test]
    fn test_tool_context_get_user_id_output_dir() {
        let ctx = ToolContext::new(99999, "test_user".to_string(), 456, "/tmp/output".to_string());

        let output_dir = ctx.get_user_id_output_dir();
        assert_eq!(output_dir, "/tmp/output/99999");
    }
}
