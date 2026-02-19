//! /ask - GLM-4.7に質問するSlash Command

use crate::history::ChatMessage;
use crate::session::{SessionKey, SessionManager};
use crate::tool::ToolContext;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

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

/// /ask コマンドの実行
pub async fn run(_ctx: &Context, interaction: &CommandInteraction, handler: &Handler) -> String {
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
        return "質問内容を入力してください。".to_string();
    }

    let user_id = interaction.user.id.get();
    let channel_id = interaction.channel_id.get();
    let user_name = interaction.user.name.clone();

    info!("Processing /ask from user {} in channel {}: {}", user_id, channel_id, question);

    // セッションキーを作成
    let session_key = SessionKey::new(user_id, channel_id);

    // セッションから履歴を取得してメッセージを追加
    let messages = {
        let manager: &Arc<Mutex<SessionManager>> = &handler.session_manager;
        let mut mgr: tokio::sync::MutexGuard<'_, SessionManager> = manager.lock().await;
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
    };

    // GLM APIに問い合わせ
    match handler.glm_client.chat_with_tools(messages, &tool_context).await {
        Ok(response) => {
            // レスポンスをセッションに追加
            let manager: &Arc<Mutex<SessionManager>> = &handler.session_manager;
            let mut mgr: tokio::sync::MutexGuard<'_, SessionManager> = manager.lock().await;
            if let Some(session) = mgr.get_mut(&session_key) {
                session.history.push(ChatMessage::assistant(&response));
            }
            response
        }
        Err(e) => {
            error!("GLM API error: {}", e);
            format!("エラーが発生しました: {}", e)
        }
    }
}
