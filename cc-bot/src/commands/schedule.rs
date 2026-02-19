//! /schedule - スケジュール管理Slash Command

use crate::scheduler::ScheduledTask;
use crate::Handler;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use tracing::error;

/// /schedule コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("schedule")
        .description("スケジュール管理")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "スケジュール追加")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "cron", "Cron式 (例: 0 9 * * * *)")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "prompt", "実行するプロンプト")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "スケジュール一覧"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "remove", "スケジュール削除")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "id", "スケジュールID")
                        .required(true),
                ),
        )
}

/// /schedule コマンドの実行
pub async fn run(
    ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> String {
    // サブコマンドを取得
    let subcommand = match command.data.options.first() {
        Some(opt) => opt,
        None => return "サブコマンドを指定してください。".to_string(),
    };

    match subcommand.name.as_str() {
        "add" => handle_add(ctx, command, handler, subcommand).await,
        "list" => handle_list(handler).await,
        "remove" => handle_remove(command, handler, subcommand).await,
        _ => "不明なサブコマンドです。".to_string(),
    }
}

/// /schedule add の処理
async fn handle_add(
    _ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
) -> String {
    // サブコマンドの値を取得（SubCommandの場合は値の中にオプションがある）
    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return "サブコマンドの値を取得できませんでした。".to_string(),
    };

    // cron オプションを取得
    let cron = sub_options
        .iter()
        .find(|opt| opt.name == "cron")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    // prompt オプションを取得
    let prompt = sub_options
        .iter()
        .find(|opt| opt.name == "prompt")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if cron.is_empty() || prompt.is_empty() {
        return "cron式とプロンプトを指定してください。".to_string();
    }

    // チャンネルIDを取得
    let channel_id = command.channel_id.get();

    // タスクを作成
    let task = match ScheduledTask::new(cron.to_string(), prompt.to_string(), channel_id) {
        Ok(t) => t,
        Err(e) => return format!("エラー: {}", e),
    };

    let next_run = task.next_run();
    let id = task.id;

    // スケジューラーに追加
    handler.scheduler.add_task(task.clone()).await;

    // ストアに保存
    {
        let mut store = handler.schedule_store.write().await;
        store.add_task(task);
        if let Err(e) = store.save("data").await {
            error!("Failed to save schedule: {}", e);
            return format!("スケジュールは追加されましたが、保存に失敗しました: {}", e);
        }
    }

    format!(
        "スケジュールを追加しました。\nID: `{}`\n次回実行: {}",
        id,
        next_run
            .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "不明".to_string())
    )
}

/// /schedule list の処理
async fn handle_list(handler: &Handler) -> String {
    let tasks = handler.scheduler.list_tasks().await;

    if tasks.is_empty() {
        return "スケジュールはありません。".to_string();
    }

    let list: Vec<String> = tasks
        .iter()
        .map(|t| {
            let next = t
                .next_run()
                .map(|d| d.format("%m/%d %H:%M").to_string())
                .unwrap_or_else(|| "?".to_string());
            format!("- `{}` [{}] {}", t.id, next, t.prompt)
        })
        .collect();

    format!("スケジュール一覧 ({}件):\n{}", tasks.len(), list.join("\n"))
}

/// /schedule remove の処理
async fn handle_remove(
    _command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
) -> String {
    // サブコマンドの値を取得
    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return "サブコマンドの値を取得できませんでした。".to_string(),
    };

    // id オプションを取得
    let id_str = sub_options
        .iter()
        .find(|opt| opt.name == "id")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if id_str.is_empty() {
        return "IDを指定してください。".to_string();
    }

    // UUIDをパース
    let id = match uuid::Uuid::parse_str(id_str) {
        Ok(id) => id,
        Err(_) => return "無効なID形式です。".to_string(),
    };

    // スケジューラーから削除
    match handler.scheduler.remove_task(id).await {
        Ok(removed) => {
            // ストアからも削除
            {
                let mut store = handler.schedule_store.write().await;
                store.remove_task(id);
                if let Err(e) = store.save("data").await {
                    error!("Failed to save after remove: {}", e);
                    return format!(
                        "スケジュールは削除されましたが、保存に失敗しました: {}",
                        e
                    );
                }
            }

            format!("スケジュールを削除しました: `{}`", removed.prompt)
        }
        Err(e) => format!("エラー: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_command() {
        // register() が CreateCommand を返すことを確認
        let _cmd = register();
        // CreateCommand は builder パターンのため、
        // 直接 name にアクセスできないが、正常に作成されることを確認
    }
}
