//! /settings - ユーザー設定Slash Command

use crate::Handler;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use tracing::error;

/// /settings コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("settings")
        .description("ユーザー設定")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "output", "出力先設定")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "path", "出力先サブディレクトリ")
                        .required(true)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "show", "現在の設定表示")
        )
}

/// /settings コマンドの実行
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
        "output" => handle_output(command, handler, subcommand).await,
        "show" => handle_show(command, handler).await,
        _ => "不明なサブコマンドです。".to_string(),
    }
}

/// /settings output の処理
async fn handle_output(
    command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
) -> String {
    let user_id = command.user.id.get();

    // サブコマンドの値を取得
    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return "サブコマンドの値を取得できませんでした。".to_string(),
    };

    // path オプションを取得
    let path = match sub_options
        .iter()
        .find(|opt| opt.name == "path")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        }) {
        Some(p) => p,
        None => return "出力先パスを指定してください。".to_string(),
    };

    // パスの検証
    if path.is_empty() {
        return "パスを空にすることはできません。".to_string();
    }

    // 危険なパスをチェック
    if path.contains("..") || path.starts_with('/') || path.starts_with('~') {
        return "無効なパスです。相対パス（..）や絶対パス（/, ~）は使用できません。".to_string();
    }

    // 設定を保存
    if let Err(e) = handler.user_settings_store.set_setting(user_id, "output_dir", path) {
        error!("Failed to save output_dir setting: {}", e);
        return format!("設定の保存に失敗しました: {}", e);
    }

    format!("出力先を `{}` に設定しました。", path)
}

/// /settings show の処理
async fn handle_show(
    command: &CommandInteraction,
    handler: &Handler,
) -> String {
    let user_id = command.user.id.get();

    // ユーザー設定を取得
    let settings = match handler.user_settings_store.get_user_settings(user_id) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get user settings: {}", e);
            return format!("設定の取得に失敗しました: {}", e);
        }
    };

    let mut lines = vec![format!("**<@{}> の設定**", user_id)];

    // 出力先
    match settings.output_subdir {
        Some(ref dir) => lines.push(format!("- 出力先: `{}`", dir)),
        None => lines.push("- 出力先: （デフォルト）".to_string()),
    }

    // 言語
    match settings.language {
        Some(ref lang) => lines.push(format!("- 言語: `{}`", lang)),
        None => lines.push("- 言語: （デフォルト）".to_string()),
    }

    // タイムゾーン
    match settings.timezone {
        Some(ref tz) => lines.push(format!("- タイムゾーン: `{}`", tz)),
        None => lines.push("- タイムゾーン: （デフォルト）".to_string()),
    }

    // 通知設定
    match settings.notifications {
        Some(ref notif) => lines.push(format!("- 通知: `{}`", notif)),
        None => lines.push("- 通知: （デフォルト）".to_string()),
    }

    // 最大履歴数
    match settings.max_history {
        Some(ref max) => lines.push(format!("- 最大履歴数: `{}`", max)),
        None => lines.push("- 最大履歴数: （デフォルト）".to_string()),
    }

    lines.join("\n")
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
