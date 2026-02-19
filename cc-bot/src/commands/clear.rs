//! /clear - セッション履歴をクリアするSlash Command

use crate::session::{SessionKey, SessionManager};
use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::Handler;

/// /clear コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("clear")
        .description("セッション履歴をクリアします")
}

/// /clear コマンドの実行
pub async fn run(_ctx: &Context, interaction: &CommandInteraction, handler: &Handler) -> String {
    let user_id = interaction.user.id.get();
    let channel_id = interaction.channel_id.get();

    info!("Clearing session for user {} in channel {}", user_id, channel_id);

    // セッションキーを作成
    let session_key = SessionKey::new(user_id, channel_id);

    // セッションをクリア
    let manager: &Arc<Mutex<SessionManager>> = &handler.session_manager;
    let mut mgr: tokio::sync::MutexGuard<'_, SessionManager> = manager.lock().await;
    let cleared = mgr.clear(&session_key);

    if cleared {
        "セッション履歴をクリアしました。".to_string()
    } else {
        "クリアする履歴がありませんでした。".to_string()
    }
}
