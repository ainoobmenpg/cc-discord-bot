use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// Globツール（ファイルパターン検索）
pub struct GlobTool;

impl GlobTool {
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

    /// globパターンをマッチング
    fn matches_pattern(file_name: &str, pattern: &str) -> bool {
        // シンプルなglobマッチング実装
        // サポート: *, ?, **

        let pattern_chars: Vec<char> = pattern.chars().collect();
        let file_chars: Vec<char> = file_name.chars().collect();

        Self::match_recursive(&pattern_chars, &file_chars, 0, 0)
    }

    fn match_recursive(
        pattern: &[char],
        file: &[char],
        p_idx: usize,
        f_idx: usize,
    ) -> bool {
        // パターン終了
        if p_idx >= pattern.len() && f_idx >= file.len() {
            return true;
        }

        // パターンだけ終了
        if p_idx >= pattern.len() {
            return false;
        }

        // ** の処理（任意のディレクトリ）
        if p_idx + 1 < pattern.len() && pattern[p_idx] == '*' && pattern[p_idx + 1] == '*' {
            // **/ をスキップ
            let next_p_idx = if p_idx + 2 < pattern.len() && pattern[p_idx + 2] == '/' {
                p_idx + 3
            } else {
                p_idx + 2
            };

            // 任意の位置でマッチを試行
            for i in f_idx..=file.len() {
                if Self::match_recursive(pattern, file, next_p_idx, i) {
                    return true;
                }
            }
            return false;
        }

        // ファイル終了
        if f_idx >= file.len() {
            // 残りが * だけならマッチ
            return pattern[p_idx..].iter().all(|&c| c == '*');
        }

        match pattern[p_idx] {
            '*' => {
                // * は0文字以上にマッチ
                // 0文字マッチ
                if Self::match_recursive(pattern, file, p_idx + 1, f_idx) {
                    return true;
                }
                // 1文字以上マッチ
                Self::match_recursive(pattern, file, p_idx, f_idx + 1)
            }
            '?' => {
                // ? は1文字にマッチ
                Self::match_recursive(pattern, file, p_idx + 1, f_idx + 1)
            }
            c => {
                // 文字が一致すれば次へ
                if c == file[f_idx] {
                    Self::match_recursive(pattern, file, p_idx + 1, f_idx + 1)
                } else {
                    false
                }
            }
        }
    }

    /// 再帰的にファイルを検索
    async fn find_files(
        base_path: &Path,
        current_path: &Path,
        pattern: &str,
        results: &mut Vec<String>,
    ) -> Result<(), std::io::Error> {
        let mut entries = fs::read_dir(current_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // 隠しファイルはスキップ
            if file_name.starts_with('.') {
                continue;
            }

            let relative_path = path
                .strip_prefix(base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            if path.is_dir() {
                // サブディレクトリを検索
                Box::pin(Self::find_files(base_path, &path, pattern, results)).await?;
            } else if Self::matches_pattern(&relative_path, pattern) {
                results.push(relative_path);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Supports *, ?, and ** patterns. Only searches within the allowed directory."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., '**/*.rs', 'src/*.txt', 'data/**/*.json')"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search from (default: current directory)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let pattern = params["pattern"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'pattern' parameter".to_string())
        })?;

        let base_path = params["path"].as_str().unwrap_or(".");

        debug!("Glob search: pattern='{}' in '{}'", pattern, base_path);

        // パスのバリデーション
        Self::validate_path(base_path)?;

        // ベースディレクトリ存在確認
        let base_path_obj = Path::new(base_path);
        if !base_path_obj.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Directory not found: {}",
                base_path
            )));
        }

        if !base_path_obj.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Not a directory: {}",
                base_path
            )));
        }

        // ファイル検索
        let mut results = Vec::new();
        if let Err(e) = Self::find_files(base_path_obj, base_path_obj, pattern, &mut results).await {
            warn!("Failed to search files: {}", e);
            return Err(ToolError::ExecutionFailed(format!(
                "Failed to search files: {}",
                e
            )));
        }

        // ソート
        results.sort();

        debug!("Found {} files matching pattern", results.len());

        if results.is_empty() {
            Ok(ToolResult::success(format!(
                "No files found matching pattern: {}",
                pattern
            )))
        } else {
            Ok(ToolResult::success(format!(
                "Found {} files:\n{}",
                results.len(),
                results.join("\n")
            )))
        }
    }
}

impl Default for GlobTool {
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
        let result = GlobTool::validate_path("/etc");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_parent() {
        let result = GlobTool::validate_path("..");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let result = GlobTool::validate_path("data");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition() {
        let tool = GlobTool::new();
        assert_eq!(tool.name(), "glob");
    }

    #[test]
    fn test_matches_pattern_simple() {
        assert!(GlobTool::matches_pattern("test.txt", "*.txt"));
        assert!(!GlobTool::matches_pattern("test.rs", "*.txt"));
        assert!(GlobTool::matches_pattern("file.rs", "*.?s"));
    }

    #[test]
    fn test_matches_pattern_with_path() {
        assert!(GlobTool::matches_pattern("src/main.rs", "src/*.rs"));
        assert!(GlobTool::matches_pattern("src/main.rs", "**/*.rs"));
        assert!(GlobTool::matches_pattern("lib/test/mod.rs", "**/*.rs"));
    }

    #[tokio::test]
    async fn test_glob_missing_pattern() {
        let tool = GlobTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "path": "."
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_glob_directory_not_found() {
        let tool = GlobTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "pattern": "*.txt",
                    "path": "nonexistent_dir"
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
    }
}
