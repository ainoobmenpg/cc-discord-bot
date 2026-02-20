//! /admin - 管理者Slash Command

use crate::schedule_store::ScheduleStore;
use crate::Handler;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use tracing::error;

/// /admin コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("admin")
        .description("管理者コマンド")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "status", "システム状態を表示"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "reload", "設定を再読み込み"),
        )
}

/// /admin コマンドの実行
pub async fn run(
    _ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> String {
    // 管理者チェック（PermissionManagerを使用）
    let user_id = command.user.id.get();
    let is_admin = {
        let manager = handler.permission_manager.read().await;
        manager.is_admin(user_id) || manager.is_super_user(user_id)
    };

    if !is_admin {
        return "このコマンドは管理者のみ実行できます。".to_string();
    }

    // サブコマンドを取得
    let subcommand = match command.data.options.first() {
        Some(opt) => opt,
        None => return "サブコマンドを指定してください。".to_string(),
    };

    match subcommand.name.as_str() {
        "status" => handle_status(handler).await,
        "reload" => handle_reload(handler).await,
        _ => "不明なサブコマンドです。".to_string(),
    }
}

/// /admin status の処理
async fn handle_status(handler: &Handler) -> String {
    // セッション数を取得
    let session_count = handler.session_manager.lock().await.len();

    // スケジュール数を取得
    let schedule_count = handler.scheduler.list_tasks().await.len();

    // ツール数を取得
    let tm = handler.glm_client.tool_manager();
    let tool_manager = tm.read().await;
    let tool_count = tool_manager.list_tools().len();

    format!(
        "**システム状態**\n\
        - セッション数: {}\n\
        - スケジュール数: {}\n\
        - ツール数: {}",
        session_count, schedule_count, tool_count
    )
}

/// /admin reload の処理
async fn handle_reload(handler: &Handler) -> String {
    let mut reload_messages = Vec::new();

    // スケジュール再読み込み
    match ScheduleStore::load("data").await {
        Ok(store) => {
            let task_count = store.len();
            handler
                .scheduler
                .set_tasks(store.get_tasks().to_vec())
                .await;
            reload_messages.push(format!("スケジュール再読み込み完了 ({}件)", task_count));
        }
        Err(e) => {
            error!("Failed to reload schedules: {}", e);
            reload_messages.push(format!("スケジュール再読み込み失敗: {}", e));
        }
    }

    format!("**設定再読み込み**\n{}", reload_messages.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_command() {
        // register() が CreateCommand を返すことを確認
        let _cmd = register();
    }
}
