mod list_files;
mod read_file;
mod remember;
mod write_file;

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
}

/// メモリツールを登録
pub fn register_memory_tools(manager: &mut ToolManager, memory_store: Arc<MemoryStore>) {
    manager.register(RememberTool::new(memory_store.clone()));
    manager.register(RecallTool::new(memory_store));
}
