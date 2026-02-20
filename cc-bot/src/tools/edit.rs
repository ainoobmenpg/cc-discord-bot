use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// ファイル編集ツール（部分編集）
pub struct EditTool;

impl EditTool {
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

    /// ユーザー固有のパスに変換
    fn get_user_path(path: &str, context: &ToolContext) -> String {
        let output_dir = context.get_user_output_dir();
        let path = path.trim_start_matches("./");
        if path == "." || path.is_empty() {
            output_dir.to_string()
        } else {
            format!("{}/{}", output_dir, path)
        }
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing specific text. Only relative paths within the allowed directory are permitted. Use this for partial edits instead of rewriting entire files."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to find and replace (must be unique in the file)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = params["path"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'path' parameter".to_string())
        })?;

        let old_string = params["old_string"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'old_string' parameter".to_string())
        })?;

        let new_string = params["new_string"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'new_string' parameter".to_string())
        })?;

        let replace_all = params["replace_all"].as_bool().unwrap_or(false);

        // パスのバリデーション
        Self::validate_path(path)?;

        // ユーザー固有のパスに変換
        let user_path = Self::get_user_path(path, context);
        debug!("Editing file: {}, replace_all: {}", user_path, replace_all);

        // ファイル存在確認
        let path_obj = Path::new(&user_path);
        if !path_obj.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                path
            )));
        }

        // ファイル読み込み
        let content = match fs::read_to_string(&user_path).await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read file {}: {}", user_path, e);
                return Err(ToolError::ExecutionFailed(format!(
                    "Failed to read file: {}",
                    e
                )));
            }
        };

        // 置換処理
        let new_content = if replace_all {
            let count = content.matches(old_string).count();
            if count == 0 {
                return Err(ToolError::ExecutionFailed(format!(
                    "Text not found in file: {}",
                    old_string
                )));
            }
            let result = content.replace(old_string, new_string);
            debug!("Replaced {} occurrences", count);
            result
        } else {
            // 単一置換
            if !content.contains(old_string) {
                return Err(ToolError::ExecutionFailed(format!(
                    "Text not found in file: {}",
                    old_string
                )));
            }

            // 一意性チェック（単一置換の場合）
            let count = content.matches(old_string).count();
            if count > 1 {
                return Err(ToolError::ExecutionFailed(format!(
                    "Found {} occurrences of the text. Use 'replace_all: true' or provide a more specific 'old_string' that is unique.",
                    count
                )));
            }

            content.replacen(old_string, new_string, 1)
        };

        // ファイル書き込み
        match fs::write(&user_path, &new_content).await {
            Ok(_) => {
                debug!("Successfully edited file: {}", user_path);
                Ok(ToolResult::success(format!(
                    "Successfully edited file: {}",
                    path
                )))
            }
            Err(e) => {
                warn!("Failed to write file {}: {}", user_path, e);
                Err(ToolError::ExecutionFailed(format!(
                    "Failed to write file: {}",
                    e
                )))
            }
        }
    }
}

impl Default for EditTool {
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
        let result = EditTool::validate_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_parent() {
        let result = EditTool::validate_path("../secret.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let result = EditTool::validate_path("data/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition() {
        let tool = EditTool::new();
        assert_eq!(tool.name(), "edit_file");
    }

    #[tokio::test]
    async fn test_edit_file_not_found() {
        let tool = EditTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "path": "nonexistent.txt",
                    "old_string": "foo",
                    "new_string": "bar"
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edit_missing_params() {
        let tool = EditTool::new();
        let ctx = create_test_context();

        // path missing
        let result = tool
            .execute(
                json!({
                    "old_string": "foo",
                    "new_string": "bar"
                }),
                &ctx,
            )
            .await;
        assert!(result.is_err());

        // old_string missing
        let result = tool
            .execute(
                json!({
                    "path": "test.txt",
                    "new_string": "bar"
                }),
                &ctx,
            )
            .await;
        assert!(result.is_err());

        // new_string missing
        let result = tool
            .execute(
                json!({
                    "path": "test.txt",
                    "old_string": "foo"
                }),
                &ctx,
            )
            .await;
        assert!(result.is_err());
    }
}
