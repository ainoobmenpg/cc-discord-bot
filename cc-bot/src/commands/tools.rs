//! /tools - 利用可能なツール一覧を表示するSlash Command

use crate::tool::ToolManager;
use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::Handler;

/// /tools コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("tools")
        .description("利用可能なツール一覧を表示します")
}

/// /tools コマンドの実行
pub async fn run(_ctx: &Context, interaction: &CommandInteraction, handler: &Handler) -> String {
    let user_id = interaction.user.id.get();
    info!("Listing tools for user {}", user_id);

    // ツールマネージャーからツール一覧を取得
    let tm: Arc<RwLock<ToolManager>> = handler.glm_client.tool_manager();
    let mgr: tokio::sync::RwLockReadGuard<'_, ToolManager> = tm.read().await;
    let tools = mgr.list_tools();

    if tools.is_empty() {
        return "利用可能なツールがありません。".to_string();
    }

    let mut response = "📋 **利用可能なツール**\n\n".to_string();
    for tool_name in &tools {
        response.push_str(&format!("• `{}`\n", tool_name));
    }
    response.push_str(&format!("\n**計 {} 個のツール**", tools.len()));

    response
}
