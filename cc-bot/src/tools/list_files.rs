use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// ファイル一覧ツール
pub struct ListFilesTool;

impl ListFilesTool {
    pub fn new() -> Self {
        Self
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

        Ok(())
    }
}

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories in a given path. Only relative paths within the allowed directory are permitted."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the directory to list (default: current directory)"
                }
            },
            "required": []
        })
    }

    async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = params["path"].as_str().unwrap_or(".");

        debug!("Listing files in: {}", path);

        // パスのバリデーション
        Self::validate_path(path)?;

        // ディレクトリ存在確認
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Directory not found: {}",
                path
            )));
        }

        if !path_obj.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Not a directory: {}",
                path
            )));
        }

        // ディレクトリ読み込み
        let mut entries = match fs::read_dir(path).await {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read directory {}: {}", path, e);
                return Err(ToolError::ExecutionFailed(format!(
                    "Failed to read directory: {}",
                    e
                )));
            }
        };

        let mut files = Vec::new();
        let mut dirs = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();

            if let Ok(file_type) = entry.file_type().await {
                if file_type.is_dir() {
                    dirs.push(format!("{}/", name));
                } else {
                    files.push(name);
                }
            }
        }

        // ソート
        dirs.sort();
        files.sort();

        // 結果をフォーマット
        let mut result = Vec::new();
        if !dirs.is_empty() {
            result.push("Directories:".to_string());
            result.extend(dirs.iter().map(|d| format!("  {}", d)));
        }
        if !files.is_empty() {
            if !dirs.is_empty() {
                result.push(String::new());
            }
            result.push("Files:".to_string());
            result.extend(files.iter().map(|f| format!("  {}", f)));
        }

        if result.is_empty() {
            result.push("(empty directory)".to_string());
        }

        debug!("Found {} dirs, {} files", dirs.len(), files.len());
        Ok(ToolResult::success(result.join("\n")))
    }
}

impl Default for ListFilesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> ToolContext {
        ToolContext::new(123, "test_user".to_string(), 456)
    }

    #[test]
    fn test_validate_path_absolute() {
        let result = ListFilesTool::validate_path("/etc");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_parent() {
        let result = ListFilesTool::validate_path("..");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let result = ListFilesTool::validate_path("data");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition() {
        let tool = ListFilesTool::new();
        assert_eq!(tool.name(), "list_files");
    }
}
