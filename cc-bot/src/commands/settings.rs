//! /settings - ユーザー設定・チャンネル設定Slash Command

use crate::Handler;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use tracing::error;

/// /settings コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("settings")
        .description("ユーザー設定・チャンネル設定")
        // ユーザー設定
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "output", "出力先設定（ユーザー）")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "path", "出力先サブディレクトリ")
                        .required(true)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "show", "現在の設定表示（ユーザー）")
        )
        // チャンネル設定
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommandGroup, "channel", "チャンネル設定")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::SubCommand, "output", "チャンネルのワーキングディレクトリ設定")
                        .add_sub_option(
                            CreateCommandOption::new(CommandOptionType::String, "path", "ワーキングディレクトリパス")
                                .required(true)
                        )
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::SubCommand, "show", "現在のチャンネル設定表示")
                )
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

    // サブコマンドグループかどうかを判定
    match subcommand.name.as_str() {
        "channel" => handle_channel_group(command, handler, subcommand).await,
        "output" => handle_output(command, handler, subcommand).await,
        "show" => handle_show(command, handler).await,
        _ => "不明なサブコマンドです。".to_string(),
    }
}

/// /settings channel グループの処理
async fn handle_channel_group(
    command: &CommandInteraction,
    handler: &Handler,
    group: &serenity::model::application::CommandDataOption,
) -> String {
    // チャンネルIDを取得
    let channel_id = command.channel_id.get();

    // サブコマンドを取得
    let sub_options = match &group.value {
        CommandDataOptionValue::SubCommandGroup(options) => options,
        _ => return "サブコマンドグループの値を取得できませんでした。".to_string(),
    };

    let subcommand = match sub_options.first() {
        Some(opt) => opt,
        None => return "サブコマンドを指定してください。".to_string(),
    };

    match subcommand.name.as_str() {
        "output" => handle_channel_output(command, handler, subcommand, channel_id).await,
        "show" => handle_channel_show(command, handler, channel_id).await,
        _ => "不明なチャンネル設定サブコマンドです。".to_string(),
    }
}

/// /settings channel output の処理
async fn handle_channel_output(
    _command: &CommandInteraction,
    handler: &Handler,
    subcommand: &serenity::model::application::CommandDataOption,
    channel_id: u64,
) -> String {
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
        None => return "ワーキングディレクトリパスを指定してください。".to_string(),
    };

    // パスの検証
    if path.is_empty() || path.len() > 4096 {
        return "パスは1文字以上4096文字以内で指定してください。".to_string();
    }
    if path.contains("..") || path.starts_with('/') || path.starts_with('~') {
        return "無効なパスです。相対パス（..）や絶対パス（/, ~）は使用できません。".to_string();
    }
    if path.contains('\\') {
        return "無効なパスです。バックスラッシュは使用できません。".to_string();
    }
    if path.contains('\0') {
        return "無効なパスです。Nullバイトを含むパスは使用できません。".to_string();
    }

    // チャンネル設定を保存
    let channel_settings_store = match handler.channel_settings_store.as_ref() {
        Some(store) => store,
        None => return "チャンネル設定ストアが初期化されていません。".to_string(),
    };

    if let Err(e) = channel_settings_store.set_setting(channel_id, "output_dir", path) {
        error!("Failed to save channel output_dir setting: {}", e);
        return format!("チャンネル設定の保存に失敗しました: {}", e);
    }

    format!("<#{}> のワーキングディレクトリを `{}` に設定しました。", channel_id, path)
}

/// /settings channel show の処理
async fn handle_channel_show(
    _command: &CommandInteraction,
    handler: &Handler,
    channel_id: u64,
) -> String {
    // チャンネル設定ストアを取得
    let channel_settings_store = match handler.channel_settings_store.as_ref() {
        Some(store) => store,
        None => return "チャンネル設定ストアが初期化されていません。".to_string(),
    };

    // チャンネル設定を取得
    let settings = match channel_settings_store.get_channel_settings(channel_id) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get channel settings: {}", e);
            return format!("チャンネル設定の取得に失敗しました: {}", e);
        }
    };

    let mut lines = vec![format!("**<#{}> のチャンネル設定**", channel_id)];

    // 出力先
    match settings.output_dir {
        Some(ref dir) => lines.push(format!("- ワーキングディレクトリ: `{}`", dir)),
        None => lines.push("- ワーキングディレクトリ: （デフォルト）".to_string()),
    }

    // 許可ロール
    match settings.allowed_roles {
        Some(ref roles) => lines.push(format!("- 許可ロール: `{}`", roles)),
        None => lines.push("- 許可ロール: （全員）".to_string()),
    }

    // 最大履歴数
    match settings.max_history {
        Some(ref max) => lines.push(format!("- 最大履歴数: `{}`", max)),
        None => lines.push("- 最大履歴数: （デフォルト）".to_string()),
    }

    lines.join("\n")
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
    if path.is_empty() || path.len() > 4096 {
        return "パスは1文字以上4096文字以内で指定してください。".to_string();
    }
    if path.contains("..") || path.starts_with('/') || path.starts_with('~') {
        return "無効なパスです。相対パス（..）や絶対パス（/, ~）は使用できません。".to_string();
    }
    if path.contains('\\') {
        return "無効なパスです。バックスラッシュは使用できません。".to_string();
    }
    if path.contains('\0') {
        return "無効なパスです。Nullバイトを含むパスは使用できません。".to_string();
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
