use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// Grepツール（ファイル内容検索）
pub struct GrepTool;

impl GrepTool {
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

    /// ユーザー固有のパスに変換
    fn get_user_path(path: &str, context: &ToolContext) -> String {
        let output_dir = context.get_user_output_dir();
        format!("{}/{}", output_dir, path)
    }

    /// ファイル内で正規表現検索
    async fn search_in_file(
        path: &str,
        pattern: &Regex,
        case_insensitive: bool,
    ) -> Result<Vec<(usize, String)>, std::io::Error> {
        let content = fs::read_to_string(path).await?;
        let mut matches = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let search_line = if case_insensitive {
                line.to_lowercase()
            } else {
                line.to_string()
            };

            if pattern.is_match(&search_line) {
                matches.push((line_num + 1, line.to_string()));
            }
        }

        Ok(matches)
    }

    /// 再帰的にファイルを検索
    async fn search_directory(
        base_path: &Path,
        current_path: &Path,
        pattern: &Regex,
        case_insensitive: bool,
        results: &mut Vec<(String, usize, String)>,
        file_pattern: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let mut entries = fs::read_dir(current_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // 隠しファイルはスキップ
            if file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                // サブディレクトリを検索
                Box::pin(Self::search_directory(
                    base_path,
                    &path,
                    pattern,
                    case_insensitive,
                    results,
                    file_pattern,
                ))
                .await?;
            } else {
                // ファイルパターンチェック
                if let Some(fp) = file_pattern {
                    let ext = path.extension().map(|e| e.to_string_lossy().to_string());
                    let matches_ext = match ext {
                        Some(e) => fp.trim_start_matches('.').eq_ignore_ascii_case(&e),
                        None => false,
                    };
                    if !matches_ext {
                        continue;
                    }
                }

                // バイナリファイルをスキップ（簡易チェック）
                let file_type = entry.file_type().await;
                if let Ok(ft) = file_type {
                    if ft.is_symlink() {
                        continue;
                    }
                }

                let relative_path = path
                    .strip_prefix(base_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                // ファイル内検索
                match Self::search_in_file(path.to_str().unwrap(), pattern, case_insensitive).await {
                    Ok(file_matches) => {
                        for (line_num, line) in file_matches {
                            results.push((relative_path.clone(), line_num, line));
                        }
                    }
                    Err(e) => {
                        debug!("Skipping file {} due to error: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for text patterns in files using regular expressions. Searches recursively through directories."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory or file to search (default: current directory)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "File extension to filter (e.g., 'rs', 'txt')"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive search (default: false)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let pattern_str = params["pattern"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'pattern' parameter".to_string())
        })?;

        let base_path = params["path"].as_str().unwrap_or(".");
        let file_pattern = params["file_pattern"].as_str();
        let case_insensitive = params["case_insensitive"].as_bool().unwrap_or(false);

        // パスのバリデーション
        Self::validate_path(base_path)?;

        // ユーザー固有のパスに変換
        let user_path = Self::get_user_path(base_path, context);
        debug!(
            "Grep search: pattern='{}' in '{}' (case_insensitive={})",
            pattern_str, user_path, case_insensitive
        );

        // 正規表現コンパイル
        let pattern = if case_insensitive {
            Regex::new(&format!("(?i){}", pattern_str))
        } else {
            Regex::new(pattern_str)
        }
        .map_err(|e| {
            ToolError::InvalidParams(format!("Invalid regex pattern: {}", e))
        })?;

        let base_path_obj = Path::new(&user_path);

        // ベースパス存在確認
        if !base_path_obj.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path not found: {}",
                base_path
            )));
        }

        let mut results = Vec::new();

        if base_path_obj.is_file() {
            // 単一ファイル検索
            match Self::search_in_file(&user_path, &pattern, case_insensitive).await {
                Ok(file_matches) => {
                    for (line_num, line) in file_matches {
                        results.push((base_path.to_string(), line_num, line));
                    }
                }
                Err(e) => {
                    warn!("Failed to search file {}: {}", user_path, e);
                    return Err(ToolError::ExecutionFailed(format!(
                        "Failed to search file: {}",
                        e
                    )));
                }
            }
        } else {
            // ディレクトリ検索
            if let Err(e) = Self::search_directory(
                base_path_obj,
                base_path_obj,
                &pattern,
                case_insensitive,
                &mut results,
                file_pattern,
            )
            .await
            {
                warn!("Failed to search directory: {}", e);
                return Err(ToolError::ExecutionFailed(format!(
                    "Failed to search directory: {}",
                    e
                )));
            }
        }

        debug!("Found {} matches", results.len());

        if results.is_empty() {
            Ok(ToolResult::success(format!(
                "No matches found for pattern: {}",
                pattern_str
            )))
        } else {
            // 結果をフォーマット
            let output: Vec<String> = results
                .iter()
                .map(|(path, line_num, line)| format!("{}:{}: {}", path, line_num, line))
                .collect();

            // 最大100件に制限
            let limited = if output.len() > 100 {
                let mut truncated = output[..100].to_vec();
                truncated.push(format!("... ({} more results)", output.len() - 100));
                truncated
            } else {
                output
            };

            Ok(ToolResult::success(format!(
                "Found {} matches:\n{}",
                results.len(),
                limited.join("\n")
            )))
        }
    }
}

impl Default for GrepTool {
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
        let result = GrepTool::validate_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_parent() {
        let result = GrepTool::validate_path("..");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let result = GrepTool::validate_path("data");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition() {
        let tool = GrepTool::new();
        assert_eq!(tool.name(), "grep");
    }

    #[tokio::test]
    async fn test_grep_invalid_regex() {
        let tool = GrepTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "pattern": "[invalid",
                    "path": "."
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_missing_pattern() {
        let tool = GrepTool::new();
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
    async fn test_grep_path_not_found() {
        let tool = GrepTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "pattern": "test",
                    "path": "nonexistent_path"
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
    }
}
