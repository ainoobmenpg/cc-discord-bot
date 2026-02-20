//! /ask - GLM-4.7に質問するSlash Command

use crate::history::ChatMessage;
use crate::session::{SessionKey, SessionManager};
use crate::tool::ToolContext;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::Handler;

/// /ask コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("ask")
        .description("GLM-4.7に質問します")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "question", "質問内容")
                .required(true),
        )
}

/// /ask コマンドの実行（deferred responseパターン）
/// Discordの3秒タイムアウトを回避するため、まず遅延応答を返してから処理を行う
pub async fn run(ctx: &Context, interaction: &CommandInteraction, handler: &Handler) {
    // オプションから質問内容を取得
    let options = &interaction.data.options;

    let question = options
        .iter()
        .find(|opt| opt.name == "question")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if question.is_empty() {
        let _ = interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("質問内容を入力してください。"),
                ),
            )
            .await;
        return;
    }

    let user_id = interaction.user.id.get();
    let channel_id = interaction.channel_id.get();
    let user_name = interaction.user.name.clone();

    info!("Processing /ask from user {} in channel {}: {}", user_id, channel_id, question);

    // まず遅延応答（Defer）を返す - これで3秒制限をクリア
    if let Err(e) = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().content("🤔 考え中..."),
            ),
        )
        .await
    {
        error!("Failed to defer response: {}", e);
        return;
    }

    // セッションキーを作成
    let session_key = SessionKey::new(user_id, channel_id);

    // セッションから履歴を取得してメッセージを追加
    let messages = {
        let manager: &Arc<Mutex<SessionManager>> = &handler.session_manager;
        let mut mgr = manager.lock().await;
        let session = mgr.get_or_create(session_key.clone());

        // ユーザーメッセージを追加
        session.history.push(ChatMessage::user(question.to_string()));

        // 全メッセージをVecで取得
        session.history.to_vec()
    };

    // ツールコンテキストを作成
    let tool_context = ToolContext {
        user_id,
        user_name,
        channel_id,
        base_output_dir: handler.base_output_dir.clone(),
        custom_output_subdir: None,
    };

    // GLM APIに問い合わせ
    let response = match handler.glm_client.chat_with_tools(messages, &tool_context).await {
        Ok(response) => {
            // レスポンスをセッションに追加
            let manager = &handler.session_manager;
            let mut mgr = manager.lock().await;
            if let Some(session) = mgr.get_mut(&session_key) {
                session.history.push(ChatMessage::assistant(&response));
            }
            response
        }
        Err(e) => {
            error!("GLM API error: {}", e);
            format!("エラーが発生しました: {}", e)
        }
    };

    // 応答を分割（Discordは2000文字制限）
    let responses = split_response(&response);

    // 最初のメッセージで元の応答を編集
    if let Err(e) = interaction
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(&responses[0]),
        )
        .await
    {
        error!("Failed to edit response: {}", e);
        return;
    }

    // 追加メッセージがあれば送信
    for additional in responses.iter().skip(1) {
        if let Err(e) = interaction
            .channel_id
            .say(&ctx.http, additional)
            .await
        {
            error!("Failed to send additional message: {}", e);
        }
    }

    debug!("Response sent successfully");
}

/// 応答を2000文字以内に分割
fn split_response(response: &str) -> Vec<String> {
    const MAX_LENGTH: usize = 2000;

    if response.len() <= MAX_LENGTH {
        return vec![response.to_string()];
    }

    let mut result = Vec::new();
    let mut remaining = response;

    while !remaining.is_empty() {
        if remaining.len() <= MAX_LENGTH {
            result.push(remaining.to_string());
            break;
        }

        // 改行位置で分割を試みる
        let split_pos = remaining[..MAX_LENGTH]
            .rfind('\n')
            .unwrap_or(MAX_LENGTH.min(remaining.len()));

        result.push(remaining[..split_pos].to_string());
        remaining = &remaining[split_pos..];
    }

    result
}
