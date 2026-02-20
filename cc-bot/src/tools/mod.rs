mod bash;
mod edit;
mod glob;
mod grep;
mod list_files;
mod read_file;
mod remember;
mod write_file;

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use list_files::ListFilesTool;
pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;

use crate::memory_store::MemoryStore;
use crate::tool::ToolManager;
use remember::{RecallTool, RememberTool};
use std::sync::Arc;

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
}

/// メモリツールを登録
pub fn register_memory_tools(manager: &mut ToolManager, memory_store: Arc<MemoryStore>) {
    manager.register(RememberTool::new(memory_store.clone()));
    manager.register(RecallTool::new(memory_store));
}
