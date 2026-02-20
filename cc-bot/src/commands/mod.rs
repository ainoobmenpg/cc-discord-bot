//! Slash Commands module
//!
//! Discord Slash Commands (/) の実装を提供します。

pub mod admin;
pub mod ask;
pub mod clear;
pub mod memory_cmd;
pub mod permission;
pub mod schedule;
pub mod settings;
pub mod tools;

use serenity::builder::CreateCommand;
use serenity::model::application::Command;

/// 全てのSlash Commandsを登録
pub fn register_commands() -> Vec<CreateCommand> {
    vec![
        admin::register(),
        ask::register(),
        clear::register(),
        memory_cmd::register(),
        permission::register(),
        schedule::register(),
        settings::register(),
        tools::register(),
    ]
}

/// グローバルコマンドとして登録（Discord Developer Portalで設定）
pub async fn register_global_commands(ctx: &serenity::prelude::Context) {
    let commands = register_commands();

    match Command::set_global_commands(&ctx.http, commands).await {
        Ok(_) => {
            tracing::info!("Successfully registered global slash commands");
        }
        Err(e) => {
            tracing::error!("Failed to register global slash commands: {}", e);
        }
    }
}
