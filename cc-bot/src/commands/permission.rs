//! /permission - パーミッション管理Slash Command

use crate::Handler;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use tracing::error;

/// /permission コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("permission")
        .description("パーミッション管理")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "パーミッション一覧")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "user", "ユーザー（省略時は自分）")
                        .required(false)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "grant", "権限付与")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "user", "対象ユーザー")
                        .required(true)
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "permission", "権限名")
                        .required(true)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "revoke", "権限剥奪")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "user", "対象ユーザー")
                        .required(true)
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "permission", "権限名")
                        .required(true)
                )
        )
}

/// /permission コマンドの実行
pub async fn run(
    _ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> String {
    // サブコマンドを取得
    let subcommand = match command.data.options.first() {
        Some(opt) => opt,
        None => return "サブコマンドを指定してください。".to_string(),
    };

    match subcommand.name.as_str() {
        "list" => handle_list(command, handler, subcommand).await,
        "grant" => handle_grant(command, handler, subcommand).await,
        "revoke" => handle_revoke(command, handler, subcommand).await,
        _ => "不明なサブコマンドです。".to_string(),
    }
}

/// /permission list の処理
async fn handle_list(
    command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
) -> String {
    // サブコマンドの値を取得
    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return "サブコマンドの値を取得できませんでした。".to_string(),
    };

    // user オプションを取得（省略時は自分）
    let target_user_id = sub_options
        .iter()
        .find(|opt| opt.name == "user")
        .and_then(|opt| {
            if let CommandDataOptionValue::User(user_id) = &opt.value {
                Some(user_id.get())
            } else {
                None
            }
        })
        .unwrap_or_else(|| command.user.id.get());

    // パーミッションを取得
    let perms = {
        let manager = handler.permission_manager.read().await;
        manager.get_permissions(target_user_id)
    };

    let perm_list: Vec<String> = perms.iter().map(|p| format!("- {}", p)).collect();

    if perm_list.is_empty() {
        format!("<@{}> のパーミッションがありません。", target_user_id)
    } else {
        format!("<@{}> のパーミッション:\n{}", target_user_id, perm_list.join("\n"))
    }
}

/// /permission grant の処理
async fn handle_grant(
    command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
) -> String {
    let admin_id = command.user.id.get();

    // 管理者チェック
    let is_admin = {
        let manager = handler.permission_manager.read().await;
        manager.is_admin(admin_id)
    };

    if !is_admin {
        return "このコマンドは管理者のみ実行できます。".to_string();
    }

    // サブコマンドの値を取得
    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return "サブコマンドの値を取得できませんでした。".to_string(),
    };

    // user オプションを取得
    let target_user_id = match sub_options
        .iter()
        .find(|opt| opt.name == "user")
        .and_then(|opt| {
            if let CommandDataOptionValue::User(user_id) = &opt.value {
                Some(user_id.get())
            } else {
                None
            }
        }) {
        Some(id) => id,
        None => return "対象ユーザーを指定してください。".to_string(),
    };

    // permission オプションを取得
    let perm_name = sub_options
        .iter()
        .find(|opt| opt.name == "permission")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if perm_name.is_empty() {
        return "権限名を指定してください。".to_string();
    }

    // パーミッションを変換
    let perm = match crate::permission::Permission::from_str(perm_name) {
        Some(p) => p,
        None => return format!("無効なパーミッション: {}\n有効な権限: FileRead, FileWrite, Schedule", perm_name),
    };

    // 権限を付与
    let result = {
        let mut manager = handler.permission_manager.write().await;
        manager.grant_permission(admin_id, target_user_id, perm.clone())
    };

    match result {
        Ok(true) => {
            // 保存
            {
                let manager = handler.permission_manager.read().await;
                if let Err(e) = manager.save("data").await {
                    error!("Failed to save permissions: {}", e);
                    return format!("{}権限を付与しましたが、保存に失敗しました: {}", perm, e);
                }
            }
            format!("{}権限を <@{}> に付与しました。", perm, target_user_id)
        }
        Ok(false) => {
            format!("<@{}> は既に{}権限を持っています。", target_user_id, perm)
        }
        Err(e) => format!("エラー: {}", e),
    }
}

/// /permission revoke の処理
async fn handle_revoke(
    command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
) -> String {
    let admin_id = command.user.id.get();

    // 管理者チェック
    let is_admin = {
        let manager = handler.permission_manager.read().await;
        manager.is_admin(admin_id)
    };

    if !is_admin {
        return "このコマンドは管理者のみ実行できます。".to_string();
    }

    // サブコマンドの値を取得
    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return "サブコマンドの値を取得できませんでした。".to_string(),
    };

    // user オプションを取得
    let target_user_id = match sub_options
        .iter()
        .find(|opt| opt.name == "user")
        .and_then(|opt| {
            if let CommandDataOptionValue::User(user_id) = &opt.value {
                Some(user_id.get())
            } else {
                None
            }
        }) {
        Some(id) => id,
        None => return "対象ユーザーを指定してください。".to_string(),
    };

    // permission オプションを取得
    let perm_name = sub_options
        .iter()
        .find(|opt| opt.name == "permission")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if perm_name.is_empty() {
        return "権限名を指定してください。".to_string();
    }

    // パーミッションを変換
    let perm = match crate::permission::Permission::from_str(perm_name) {
        Some(p) => p,
        None => return format!("無効なパーミッション: {}\n有効な権限: FileRead, FileWrite, Schedule", perm_name),
    };

    // 権限を剥奪
    let result = {
        let mut manager = handler.permission_manager.write().await;
        manager.revoke_permission(admin_id, target_user_id, perm.clone())
    };

    match result {
        Ok(true) => {
            // 保存
            {
                let manager = handler.permission_manager.read().await;
                if let Err(e) = manager.save("data").await {
                    error!("Failed to save permissions: {}", e);
                    return format!("{}権限を剥奪しましたが、保存に失敗しました: {}", perm, e);
                }
            }
            format!("{}権限を <@{}> から剥奪しました。", perm, target_user_id)
        }
        Ok(false) => {
            format!("<@{}> は{}権限を持っていません。", target_user_id, perm)
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
