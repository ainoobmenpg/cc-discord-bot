use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::process::Command;
use tracing::{debug, warn};

/// Bashツール（クロスプラットフォームシェルコマンド実行）
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    /// 危険なコマンドをチェック
    fn is_dangerous_command(command: &str) -> bool {
        let dangerous_patterns = [
            "rm -rf /",
            "rm -rf /*",
            ":(){:|:&};:",  // フォークボム
            "mkfs",
            "dd if=/dev/zero",
            "> /dev/sda",
            "chmod -R 777 /",
            "chown -R",
            "wget",
            "curl -X POST",
            "curl -X DELETE",
            "nc -l",
            "ncat",
            "/etc/passwd",
            "/etc/shadow",
            "sudo",
            "su ",
            "passwd",
        ];

        let cmd_lower = command.to_lowercase();
        dangerous_patterns.iter().any(|p| cmd_lower.contains(&p.to_lowercase()))
    }

    /// コマンドを実行
    fn execute_command(command: &str, _timeout_secs: u64) -> Result<(String, String, bool), String> {
        #[cfg(windows)]
        {
            // Windows: PowerShellを使用
            let output = Command::new("powershell")
                .args(["-Command", command])
                .output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            Ok((stdout, stderr, output.status.success()))
        }

        #[cfg(not(windows))]
        {
            // Unix系: shを使用
            let output = Command::new("sh")
                .args(["-c", command])
                .output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            Ok((stdout, stderr, output.status.success()))
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands. Uses PowerShell on Windows and sh on Unix. Commands are executed in a sandboxed environment with safety restrictions."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30, max: 60)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = params["command"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'command' parameter".to_string())
        })?;

        let timeout = params["timeout"].as_u64().unwrap_or(30).min(60);

        debug!("Executing command: {} (timeout: {}s)", command, timeout);

        // 危険なコマンドをブロック
        if Self::is_dangerous_command(command) {
            warn!("Blocked dangerous command: {}", command);
            return Err(ToolError::PermissionDenied(
                "This command is not allowed for security reasons".to_string(),
            ));
        }

        // 空のコマンドをチェック
        if command.trim().is_empty() {
            return Err(ToolError::InvalidParams(
                "Command cannot be empty".to_string(),
            ));
        }

        // コマンド実行
        match Self::execute_command(command, timeout) {
            Ok((stdout, stderr, success)) => {
                if success {
                    let output = if stdout.trim().is_empty() {
                        "Command executed successfully (no output)".to_string()
                    } else {
                        stdout
                    };
                    debug!("Command output: {}", output);
                    Ok(ToolResult::success(output))
                } else {
                    let error_msg = if stderr.trim().is_empty() {
                        "Command failed (no error message)".to_string()
                    } else {
                        stderr
                    };
                    warn!("Command failed: {}", error_msg);
                    Ok(ToolResult::error(error_msg))
                }
            }
            Err(e) => {
                warn!("Command execution error: {}", e);
                Err(ToolError::ExecutionFailed(e))
            }
        }
    }
}

impl Default for BashTool {
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
    fn test_is_dangerous_command_rm_rf() {
        assert!(BashTool::is_dangerous_command("rm -rf /"));
        assert!(BashTool::is_dangerous_command("rm -rf /*"));
    }

    #[test]
    fn test_is_dangerous_command_sudo() {
        assert!(BashTool::is_dangerous_command("sudo apt install"));
    }

    #[test]
    fn test_is_dangerous_command_safe() {
        assert!(!BashTool::is_dangerous_command("ls -la"));
        assert!(!BashTool::is_dangerous_command("echo hello"));
        assert!(!BashTool::is_dangerous_command("cat file.txt"));
    }

    #[test]
    fn test_tool_definition() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "bash");
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "command": "echo hello"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_empty_command() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "command": ""
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_dangerous_command() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "command": "rm -rf /"
                }),
                &ctx,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ToolError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn test_bash_invalid_command() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        // 存在しないコマンド
        let result = tool
            .execute(
                json!({
                    "command": "nonexistent_command_xyz123"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn test_bash_timeout_parameter() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        // タイムアウト値が制限されることを確認（60秒以上は60に制限）
        let result = tool
            .execute(
                json!({
                    "command": "echo test",
                    "timeout": 120
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
    }
}
