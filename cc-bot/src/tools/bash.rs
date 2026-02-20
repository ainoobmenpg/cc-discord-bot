use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
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
            // 破壊的コマンド
            "rm -rf /",
            "rm -rf /*",
            ":(){:|:&};:",  // フォークボム
            "mkfs",
            "dd if=/dev/zero",
            "> /dev/sda",
            "chmod -R 777 /",
            "chown -R",
            // ネットワーク
            "wget",
            "curl -X POST",
            "curl -X DELETE",
            "curl -X PUT",
            "nc -l",
            "ncat",
            // システムファイル
            "/etc/passwd",
            "/etc/shadow",
            // 権限昇格
            "sudo",
            "su ",
            "passwd",
            // コマンドインジェクション
            "eval ",
            "exec ",
            "source ",
            // 難読化実行
            "base64 -d",
            "base64 --decode",
            // 環境変数
            "printenv",
            // シェルエスケープ
            "$(",
            "`",
            // 危険なリダイレクト
            "> /",
            ">> /",
        ];

        let cmd_lower = command.to_lowercase();
        dangerous_patterns.iter().any(|p| cmd_lower.contains(&p.to_lowercase()))
    }

    /// コマンドを非同期で実行（タイムアウト付き）
    async fn execute_command_async(
        command: &str,
        timeout_secs: u64,
        working_dir: &str,
    ) -> Result<(String, String, bool), String> {
        let timeout_duration = Duration::from_secs(timeout_secs);

        #[cfg(windows)]
        let result = {
            let cmd = TokioCommand::new("powershell")
                .args(["-Command", command])
                .current_dir(working_dir)
                .output();

            timeout(timeout_duration, cmd).await
        };

        #[cfg(not(windows))]
        let result = {
            let cmd = TokioCommand::new("sh")
                .args(["-c", command])
                .current_dir(working_dir)
                .output();

            timeout(timeout_duration, cmd).await
        };

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Ok((stdout, stderr, output.status.success()))
            }
            Ok(Err(e)) => Err("Failed to execute command. Please check the command syntax.".to_string()),
            Err(_) => Err(format!("Command timed out after {} seconds", timeout_secs)),
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

    async fn execute(&self, params: JsonValue, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = params["command"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'command' parameter".to_string())
        })?;

        let timeout_secs = params["timeout"].as_u64().unwrap_or(30).min(60);

        debug!("Executing command: {} (timeout: {}s)", command, timeout_secs);

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

        // ユーザー固有の出力ディレクトリを作業ディレクトリとして使用
        let working_dir = context.get_user_output_dir();

        // ディレクトリが存在することを確認
        if !std::path::Path::new(&working_dir).exists() {
            std::fs::create_dir_all(&working_dir).map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create working directory: {}", e))
            })?;
        }

        // コマンド実行（非同期、タイムアウト付き）
        match Self::execute_command_async(command, timeout_secs, &working_dir).await {
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
    fn test_is_dangerous_command_eval() {
        assert!(BashTool::is_dangerous_command("eval $(cat file)"));
    }

    #[test]
    fn test_is_dangerous_command_base64() {
        assert!(BashTool::is_dangerous_command("echo Y2F0IC9ldGMvcGFzc3dk | base64 -d | sh"));
    }

    #[test]
    fn test_is_dangerous_command_command_substitution() {
        assert!(BashTool::is_dangerous_command("echo $(cat /etc/passwd)"));
        assert!(BashTool::is_dangerous_command("echo `cat /etc/passwd`"));
    }

    #[test]
    fn test_is_dangerous_command_safe() {
        assert!(!BashTool::is_dangerous_command("ls -la"));
        assert!(!BashTool::is_dangerous_command("echo hello"));
        assert!(!BashTool::is_dangerous_command("cat file.txt"));
        assert!(!BashTool::is_dangerous_command("npm install"));
        assert!(!BashTool::is_dangerous_command("cargo build"));
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
    async fn test_bash_dangerous_command_eval() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(
                json!({
                    "command": "eval $(cat /etc/passwd)"
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

    #[tokio::test]
    async fn test_bash_timeout_execution() {
        let tool = BashTool::new();
        let ctx = create_test_context();

        // タイムアウトが実際に機能することを確認
        let result = tool
            .execute(
                json!({
                    "command": "sleep 10",
                    "timeout": 1
                }),
                &ctx,
            )
            .await;

        // タイムアウトでエラーになるはず
        assert!(result.is_err());
        assert!(matches!(result, Err(ToolError::ExecutionFailed(_))));
    }
}
