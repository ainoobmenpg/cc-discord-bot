mod bash;
mod edit;
mod glob;
mod grep;
mod list_files;
mod mcp;
mod read_file;
mod remember;
mod web_fetch;
mod write_file;

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use list_files::ListFilesTool;
pub use mcp::{load_mcp_tools, MCPToolAdapter};
pub use read_file::ReadFileTool;
pub use web_fetch::WebFetchTool;
pub use write_file::WriteFileTool;

use crate::memory_store::MemoryStore;
use crate::tool::{Tool, ToolManager};
use remember::{RecallTool, RememberTool};
use std::sync::Arc;
use tracing::info;

/// デフォルトツールを登録
pub fn register_default_tools(manager: &mut ToolManager) {
    manager.register(ReadFileTool::new());
    manager.register(WriteFileTool::new());
    manager.register(ListFilesTool::new());
    // 新しいツール
    manager.register(EditTool::new());
    manager.register(GlobTool::new());
    manager.register(GrepTool::new());
    manager.register(BashTool::new());
    // Web ツール
    manager.register(WebFetchTool::new());
}

/// メモリツールを登録
pub fn register_memory_tools(manager: &mut ToolManager, memory_store: Arc<MemoryStore>) {
    manager.register(RememberTool::new(memory_store.clone()));
    manager.register(RecallTool::new(memory_store));
}

/// MCPツールを登録（非同期）
pub async fn register_mcp_tools(manager: &mut ToolManager, config_path: &str) -> Result<(), String> {
    match load_mcp_tools(config_path).await {
        Ok(tools) => {
            let count = tools.len();
            for tool in tools {
                let name = tool.name().to_string();
                info!("Registering MCP tool: {}", name);
                manager.register(tool);
            }
            info!("Registered {} MCP tools", count);
            Ok(())
        }
        Err(e) => {
            // MCP設定がなくてもエラーにせず、警告のみ
            if e.contains("No such file") || e.contains("not found") {
                info!("MCP config not found at {}, skipping MCP tools", config_path);
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
