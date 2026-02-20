use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// ファイル読み込みツール
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }

    /// パスがシンボリックリンクかチェック
    fn is_symlink(path: &Path) -> bool {
        path.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }

    /// パスのバリデーション
    fn validate_path(path: &str) -> Result<(), ToolError> {
        // 絶対パスは禁止
        if path.starts_with('/') {
            return Err(ToolError::PermissionDenied(
                "Absolute paths are not allowed".to_string(),
            ));
        }

        // 親ディレクトリ参照を禁止
        if path.contains("..") {
            return Err(ToolError::PermissionDenied(
                "Parent directory references are not allowed".to_string(),
            ));
        }

        // シンボリックリンクを禁止（セキュリティ対策）
        if Self::is_symlink(Path::new(path)) {
            return Err(ToolError::PermissionDenied(
                "Symbolic links are not allowed for security reasons".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Only relative paths within the allowed directory are permitted."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = params["path"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'path' parameter".to_string())
        })?;

        debug!("Reading file: {}", path);

        // パスのバリデーション
        Self::validate_path(path)?;

        // ファイル存在確認
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(ToolError::ExecutionFailed(
                "File not found. Please check the path.".to_string()
            ));
        }

        // ファイル読み込み
        match fs::read_to_string(path).await {
            Ok(content) => {
                debug!("Successfully read {} bytes from {}", content.len(), path);
                Ok(ToolResult::success(content))
            }
            Err(e) => {
                warn!("Failed to read file {}: {}", path, e);
                // ユーザーには一般的なエラーメッセージを返す
                Err(ToolError::ExecutionFailed(
                    "Failed to read file. Please check the path and permissions.".to_string()
                ))
            }
        }
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> ToolContext {
        ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
    }

    #[test]
    fn test_validate_path_absolute() {
        let result = ReadFileTool::validate_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_parent() {
        let result = ReadFileTool::validate_path("../secret.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let result = ReadFileTool::validate_path("data/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition() {
        let tool = ReadFileTool::new();
        assert_eq!(tool.name(), "read_file");
    }
}
