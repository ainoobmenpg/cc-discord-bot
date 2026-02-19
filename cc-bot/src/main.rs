mod commands;
mod glm;
mod history;
mod memory;
mod memory_store;
mod permission;
mod rate_limiter;
mod scheduler;
mod schedule_store;
mod session;
mod tool;
mod tools;
mod validation;

use memory_store::MemoryStore;
use scheduler::{Scheduler, ScheduledTask};
use schedule_store::ScheduleStore;
use serenity::http::Http;
use serenity::model::application::{Interaction, CommandInteraction};
use serenity::prelude::*;
use serenity::model::channel::Message;
use session::{SessionKey, SessionManager, SessionStore};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, broadcast};
use tracing::{debug, error, info, warn};

/// 管理者ユーザーIDのキャッシュ
static ADMIN_USER_IDS: std::sync::OnceLock<Vec<u64>> = std::sync::OnceLock::new();

/// 管理者かどうかを判定
fn is_admin(user_id: u64) -> bool {
    let admin_ids = ADMIN_USER_IDS.get_or_init(|| {
        match env::var("ADMIN_USER_IDS") {
            Ok(ids) => ids
                .split(',')
                .filter_map(|s| s.trim().parse::<u64>().ok())
                .collect(),
            Err(_) => {
                warn!("ADMIN_USER_IDS environment variable not set");
                Vec::new()
            }
        }
    });

    !admin_ids.is_empty() && admin_ids.contains(&user_id)
}

pub struct Handler {
    glm_client: glm::GLMClient,
    session_manager: Arc<Mutex<SessionManager>>,
    scheduler: Arc<Scheduler>,
    schedule_store: Arc<RwLock<ScheduleStore>>,
    rate_limiter: Arc<Mutex<rate_limiter::RateLimiter>>,
    permission_manager: Arc<RwLock<permission::PermissionManager>>,
    memory_store: Arc<MemoryStore>,
    http: Arc<Http>,
    /// 処理済みメッセージID（重複防止）
    processed_messages: Arc<Mutex<HashSet<u64>>>,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        // ボットメッセージは無視（Slash Commandsのみ使用）
        if msg.author.bot {
            return;
        }
        debug!("Received message (ignored - use Slash Commands): {}", msg.content);
    }

    async fn ready(&self, ctx: Context, ready: serenity::model::gateway::Ready) {
        info!("{} is connected!", ready.user.name);

        // Slash Commandsを登録
        commands::register_global_commands(&ctx).await;
        info!("Slash commands registered");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            self.handle_slash_command(&ctx, &command).await;
        }
    }
}

impl Handler {
    /// Slash Commandを処理
    async fn handle_slash_command(&self, ctx: &Context, command: &CommandInteraction) {
        let response = match command.data.name.as_str() {
            "admin" => commands::admin::run(ctx, command, self).await,
            "ask" => commands::ask::run(ctx, command, self).await,
            "clear" => commands::clear::run(ctx, command, self).await,
            "memory" => commands::memory_cmd::run(ctx, command, self).await,
            "permission" => commands::permission::run(ctx, command, self).await,
            "schedule" => commands::schedule::run(ctx, command, self).await,
            "tools" => commands::tools::run(ctx, command, self).await,
            _ => "不明なコマンドです。".to_string(),
        };

        // インタラクションに応答
        if let Err(e) = command
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new().content(&response),
                ),
            )
            .await
        {
            error!("Failed to respond to slash command: {}", e);
        }
    }

    /// 管理者コマンドを処理
    async fn handle_admin_command(&self, ctx: &Context, msg: &Message) {
        // 管理者チェック
        if !is_admin(msg.author.id.get()) {
            if let Err(e) = msg.reply(ctx, "このコマンドは管理者のみ実行できます。").await {
                error!("Failed to send admin denied message: {}", e);
            }
            return;
        }

        let content = msg.content.trim();
        let args = content["!admin".len()..].trim();

        if args.starts_with("status") {
            // !admin status - システム状態表示
            let session_count = self.session_manager.lock().await.len();
            let schedule_count = self.scheduler.list_tasks().await.len();

            let tm = self.glm_client.tool_manager();
            let tool_manager = tm.read().await;
            let tool_count = tool_manager.list_tools().len();

            let reply = format!(
                "**システム状態**\n\
                - セッション数: {}\n\
                - スケジュール数: {}\n\
                - ツール数: {}",
                session_count,
                schedule_count,
                tool_count
            );

            if let Err(e) = msg.reply(ctx, reply).await {
                error!("Failed to send admin status: {}", e);
            }
        } else if args.starts_with("reload") {
            // !admin reload - 設定再読み込み
            let mut reload_messages = Vec::new();

            // スケジュール再読み込み
            match ScheduleStore::load("data").await {
                Ok(store) => {
                    let task_count = store.len();
                    self.scheduler.set_tasks(store.get_tasks().to_vec()).await;
                    reload_messages.push(format!("スケジュール再読み込み完了 ({}件)", task_count));
                }
                Err(e) => {
                    error!("Failed to reload schedules: {}", e);
                    reload_messages.push(format!("スケジュール再読み込み失敗: {}", e));
                }
            }

            let reply = format!("**設定再読み込み**\n{}", reload_messages.join("\n"));
            if let Err(e) = msg.reply(ctx, reply).await {
                error!("Failed to send admin reload result: {}", e);
            }
        } else {
            let usage = "使い方:\n\
                - !admin status - システム状態表示\n\
                - !admin reload - 設定再読み込み";
            if let Err(e) = msg.reply(ctx, usage).await {
                error!("Failed to send admin usage: {}", e);
            }
        }
    }

    async fn handle_schedule_command(&self, ctx: &Context, msg: &Message) {
        let content = msg.content.trim();
        let args = content["!schedule".len()..].trim();

        if args.starts_with("add") {
            // !schedule add "<cron>" "<prompt>"
            let rest = args[3..].trim();
            if let Some((cron, prompt)) = Self::parse_schedule_add(rest) {
                match ScheduledTask::new(cron, prompt, msg.channel_id.get()) {
                    Ok(task) => {
                        let next_run = task.next_run();
                        let id = task.id;

                        // スケジューラーに追加
                        self.scheduler.add_task(task.clone()).await;

                        // ストアに保存
                        {
                            let mut store = self.schedule_store.write().await;
                            store.add_task(task);
                            if let Err(e) = store.save("data").await {
                                error!("Failed to save schedule: {}", e);
                            }
                        }

                        let reply = format!(
                            "スケジュールを追加しました。\nID: `{}`\n次回実行: {}",
                            id,
                            next_run.map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                .unwrap_or_else(|| "不明".to_string())
                        );
                        if let Err(e) = msg.reply(ctx, reply).await {
                            error!("Failed to send schedule add response: {}", e);
                        }
                    }
                    Err(e) => {
                        if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                            error!("Failed to send error: {}", err);
                        }
                    }
                }
            } else {
                let usage = "使い方: !schedule add \"<cron式>\" \"<プロンプト>\"\n例: !schedule add \"0 9 * * * *\" \"おはよう\"";
                if let Err(e) = msg.reply(ctx, usage).await {
                    error!("Failed to send usage: {}", e);
                }
            }
        } else if args.starts_with("list") {
            let tasks = self.scheduler.list_tasks().await;

            if tasks.is_empty() {
                if let Err(e) = msg.reply(ctx, "スケジュールはありません。").await {
                    error!("Failed to send empty list: {}", e);
                }
            } else {
                let list: Vec<String> = tasks.iter().map(|t| {
                    let next = t.next_run()
                        .map(|d| d.format("%m/%d %H:%M").to_string())
                        .unwrap_or_else(|| "?".to_string());
                    format!("- `{}` [{}] {}", t.id, next, t.prompt)
                }).collect();

                let reply = format!("スケジュール一覧 ({}件):\n{}", tasks.len(), list.join("\n"));
                if let Err(e) = msg.reply(ctx, reply).await {
                    error!("Failed to send schedule list: {}", e);
                }
            }
        } else if args.starts_with("remove") {
            let id_str = args[6..].trim();
            if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                match self.scheduler.remove_task(id).await {
                    Ok(removed) => {
                        // ストアからも削除
                        {
                            let mut store = self.schedule_store.write().await;
                            store.remove_task(id);
                            if let Err(e) = store.save("data").await {
                                error!("Failed to save after remove: {}", e);
                            }
                        }

                        let reply = format!("スケジュールを削除しました: `{}`", removed.prompt);
                        if let Err(e) = msg.reply(ctx, reply).await {
                            error!("Failed to send remove response: {}", e);
                        }
                    }
                    Err(e) => {
                        if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                            error!("Failed to send error: {}", err);
                        }
                    }
                }
            } else {
                if let Err(e) = msg.reply(ctx, "使い方: !schedule remove <ID>").await {
                    error!("Failed to send usage: {}", e);
                }
            }
        } else {
            let usage = "使い方:\n- !schedule add \"<cron式>\" \"<プロンプト>\"\n- !schedule list\n- !schedule remove <ID>";
            if let Err(e) = msg.reply(ctx, usage).await {
                error!("Failed to send usage: {}", e);
            }
        }
    }

    fn parse_schedule_add(input: &str) -> Option<(String, String)> {
        let input = input.trim();

        // "cron" "prompt" 形式をパース
        if !input.starts_with('"') {
            return None;
        }

        let rest = &input[1..];
        let cron_end = rest.find('"')?;
        let cron = rest[..cron_end].to_string();

        let rest = rest[cron_end + 1..].trim();
        if !rest.starts_with('"') {
            return None;
        }

        let rest = &rest[1..];
        let prompt_end = rest.rfind('"')?;
        let prompt = rest[..prompt_end].to_string();

        Some((cron, prompt))
    }

    /// パーミッションコマンドを処理
    async fn handle_permission_command(&self, ctx: &Context, msg: &Message) {
        let content = msg.content.trim();
        let args = content["!permission".len()..].trim();

        if args.starts_with("list") {
            // !permission list - 自分のパーミッションを表示
            let user_id = msg.author.id.get();
            let perms = {
                let manager = self.permission_manager.read().await;
                manager.get_permissions(user_id)
            };

            let perm_list: Vec<String> = perms.iter().map(|p| format!("- {}", p)).collect();
            let reply = if perm_list.is_empty() {
                "パーミッションがありません。".to_string()
            } else {
                format!("あなたのパーミッション:\n{}", perm_list.join("\n"))
            };

            if let Err(e) = msg.reply(ctx, reply).await {
                error!("Failed to send permission list: {}", e);
            }
        } else if args.starts_with("grant") {
            // !permission grant @user <permission> - 管理者のみ
            let admin_id = msg.author.id.get();

            // メンションからユーザーIDを取得
            let rest = args[5..].trim();
            let (target_user_id, permission_name) = if let Some(mention_start) = rest.find('<') {
                if let Some(mention_end) = rest.find('>') {
                    let mention = &rest[mention_start + 1..mention_end];
                    if mention.starts_with('@') {
                        let id_str = &mention[1..];
                        if let Ok(id) = id_str.parse::<u64>() {
                            let perm_name = rest[mention_end + 1..].trim();
                            (Some(id), perm_name)
                        } else {
                            (None, "")
                        }
                    } else {
                        (None, "")
                    }
                } else {
                    (None, "")
                }
            } else {
                (None, "")
            };

            if let (Some(target_id), perm_name) = (target_user_id, permission_name) {
                if perm_name.is_empty() {
                    if let Err(e) = msg.reply(ctx, "使い方: !permission grant @user <permission>").await {
                        error!("Failed to send usage: {}", e);
                    }
                    return;
                }

                if let Some(perm) = permission::Permission::from_str(perm_name) {
                    let result = {
                        let mut manager = self.permission_manager.write().await;
                        manager.grant_permission(admin_id, target_id, perm.clone())
                    };

                    match result {
                        Ok(true) => {
                            // 保存
                            {
                                let manager = self.permission_manager.read().await;
                                if let Err(e) = manager.save("data").await {
                                    error!("Failed to save permissions: {}", e);
                                }
                            }
                            let reply = format!("{}権限を <@{}> に付与しました。", perm, target_id);
                            if let Err(e) = msg.reply(ctx, reply).await {
                                error!("Failed to send grant response: {}", e);
                            }
                        }
                        Ok(false) => {
                            let reply = format!("<@{}> は既に{}権限を持っています。", target_id, perm);
                            if let Err(e) = msg.reply(ctx, reply).await {
                                error!("Failed to send grant response: {}", e);
                            }
                        }
                        Err(e) => {
                            if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                                error!("Failed to send error: {}", err);
                            }
                        }
                    }
                } else {
                    if let Err(e) = msg.reply(ctx, format!("無効なパーミッション: {}", perm_name)).await {
                        error!("Failed to send error: {}", e);
                    }
                }
            } else {
                if let Err(e) = msg.reply(ctx, "使い方: !permission grant @user <permission>").await {
                    error!("Failed to send usage: {}", e);
                }
            }
        } else if args.starts_with("revoke") {
            // !permission revoke @user <permission> - 管理者のみ
            let admin_id = msg.author.id.get();

            // メンションからユーザーIDを取得
            let rest = args[6..].trim();
            let (target_user_id, permission_name) = if let Some(mention_start) = rest.find('<') {
                if let Some(mention_end) = rest.find('>') {
                    let mention = &rest[mention_start + 1..mention_end];
                    if mention.starts_with('@') {
                        let id_str = &mention[1..];
                        if let Ok(id) = id_str.parse::<u64>() {
                            let perm_name = rest[mention_end + 1..].trim();
                            (Some(id), perm_name)
                        } else {
                            (None, "")
                        }
                    } else {
                        (None, "")
                    }
                } else {
                    (None, "")
                }
            } else {
                (None, "")
            };

            if let (Some(target_id), perm_name) = (target_user_id, permission_name) {
                if perm_name.is_empty() {
                    if let Err(e) = msg.reply(ctx, "使い方: !permission revoke @user <permission>").await {
                        error!("Failed to send usage: {}", e);
                    }
                    return;
                }

                if let Some(perm) = permission::Permission::from_str(perm_name) {
                    let result = {
                        let mut manager = self.permission_manager.write().await;
                        manager.revoke_permission(admin_id, target_id, perm.clone())
                    };

                    match result {
                        Ok(true) => {
                            // 保存
                            {
                                let manager = self.permission_manager.read().await;
                                if let Err(e) = manager.save("data").await {
                                    error!("Failed to save permissions: {}", e);
                                }
                            }
                            let reply = format!("{}権限を <@{}> から剥奪しました。", perm, target_id);
                            if let Err(e) = msg.reply(ctx, reply).await {
                                error!("Failed to send revoke response: {}", e);
                            }
                        }
                        Ok(false) => {
                            let reply = format!("<@{}> は{}権限を持っていません。", target_id, perm);
                            if let Err(e) = msg.reply(ctx, reply).await {
                                error!("Failed to send revoke response: {}", e);
                            }
                        }
                        Err(e) => {
                            if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                                error!("Failed to send error: {}", err);
                            }
                        }
                    }
                } else {
                    if let Err(e) = msg.reply(ctx, format!("無効なパーミッション: {}", perm_name)).await {
                        error!("Failed to send error: {}", e);
                    }
                }
            } else {
                if let Err(e) = msg.reply(ctx, "使い方: !permission revoke @user <permission>").await {
                    error!("Failed to send usage: {}", e);
                }
            }
        } else {
            let usage = "使い方:\n\
                - !permission list - 自分のパーミッションを表示\n\
                - !permission grant @user <permission> - パーミッション付与（管理者のみ）\n\
                - !permission revoke @user <permission> - パーミッション剥奪（管理者のみ）";
            if let Err(e) = msg.reply(ctx, usage).await {
                error!("Failed to send usage: {}", e);
            }
        }
    }

    /// メモリコマンドを処理
    async fn handle_memory_command(&self, ctx: &Context, msg: &Message) {
        let user_id = msg.author.id.get();
        let content = msg.content.trim();
        let args = content["!memory".len()..].trim();

        if args.starts_with("list") {
            // !memory list - 自分のメモリ一覧を表示（最新10件）
            match self.memory_store.list_memories(user_id, 10) {
                Ok(memories) => {
                    if memories.is_empty() {
                        if let Err(e) = msg.reply(ctx, "メモリがありません。").await {
                            error!("Failed to send empty memory list: {}", e);
                        }
                    } else {
                        let list: Vec<String> = memories.iter().map(|m| {
                            let date = m.created_at.format("%m/%d %H:%M");
                            format!("- [{}] {} (ID: {})", date, m.content, m.id)
                        }).collect();

                        let reply = format!("**あなたのメモリ ({}件)**\n{}", memories.len(), list.join("\n"));
                        if let Err(e) = msg.reply(ctx, reply).await {
                            error!("Failed to send memory list: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to list memories: {}", e);
                    if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                        error!("Failed to send error: {}", err);
                    }
                }
            }
        } else if args.starts_with("search") {
            // !memory search <query> - メモリを検索
            let query = args[6..].trim();

            if query.is_empty() {
                if let Err(e) = msg.reply(ctx, "使い方: !memory search <検索ワード>").await {
                    error!("Failed to send usage: {}", e);
                }
                return;
            }

            match self.memory_store.search_memories(user_id, query) {
                Ok(memories) => {
                    if memories.is_empty() {
                        let reply = format!("「{}」に一致するメモリがありません。", query);
                        if let Err(e) = msg.reply(ctx, reply).await {
                            error!("Failed to send empty search result: {}", e);
                        }
                    } else {
                        let list: Vec<String> = memories.iter().map(|m| {
                            let date = m.created_at.format("%m/%d %H:%M");
                            format!("- [{}] {} (ID: {})", date, m.content, m.id)
                        }).collect();

                        let reply = format!("**検索結果: {} ({}件)**\n{}", query, memories.len(), list.join("\n"));
                        if let Err(e) = msg.reply(ctx, reply).await {
                            error!("Failed to send search result: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to search memories: {}", e);
                    if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                        error!("Failed to send error: {}", err);
                    }
                }
            }
        } else if args.starts_with("delete") {
            // !memory delete <id> - メモリを削除
            let id_str = args[6..].trim();

            if id_str.is_empty() {
                if let Err(e) = msg.reply(ctx, "使い方: !memory delete <ID>").await {
                    error!("Failed to send usage: {}", e);
                }
                return;
            }

            match id_str.parse::<i64>() {
                Ok(id) => {
                    match self.memory_store.delete_memory(user_id, id) {
                        Ok(deleted) => {
                            let reply = format!("メモリを削除しました: {}", deleted.content);
                            if let Err(e) = msg.reply(ctx, reply).await {
                                error!("Failed to send delete response: {}", e);
                            }
                        }
                        Err(memory_store::MemoryError::NotFound(_)) => {
                            if let Err(e) = msg.reply(ctx, "指定されたメモリが見つかりません。").await {
                                error!("Failed to send not found: {}", e);
                            }
                        }
                        Err(memory_store::MemoryError::PermissionDenied(_)) => {
                            if let Err(e) = msg.reply(ctx, "他のユーザーのメモリは削除できません。").await {
                                error!("Failed to send permission denied: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to delete memory: {}", e);
                            if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                                error!("Failed to send error: {}", err);
                            }
                        }
                    }
                }
                Err(_) => {
                    if let Err(e) = msg.reply(ctx, "IDは数値で指定してください。").await {
                        error!("Failed to send invalid id: {}", e);
                    }
                }
            }
        } else if args.starts_with("clear confirm") {
            // !memory clear confirm - 全メモリ削除の実行
            match self.memory_store.clear_memories(user_id) {
                Ok(count) => {
                    let reply = format!("{} 件のメモリをすべて削除しました。", count);
                    if let Err(e) = msg.reply(ctx, reply).await {
                        error!("Failed to send clear response: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to clear memories: {}", e);
                    if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                        error!("Failed to send error: {}", err);
                    }
                }
            }
        } else if args.starts_with("clear") {
            // !memory clear - 自分の全メモリを削除（確認付き）
            let count = match self.memory_store.count_memories(user_id) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to count memories: {}", e);
                    if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                        error!("Failed to send error: {}", err);
                    }
                    return;
                }
            };

            if count == 0 {
                if let Err(e) = msg.reply(ctx, "削除するメモリがありません。").await {
                    error!("Failed to send no memories: {}", e);
                }
                return;
            }

            let reply = format!(
                "**警告: すべてのメモリを削除します**\n\
                現在 {} 件のメモリがあります。\n\
                削除を確定するには `!memory clear confirm` と入力してください。",
                count
            );
            if let Err(e) = msg.reply(ctx, reply).await {
                error!("Failed to send clear warning: {}", e);
            }
        } else if args.starts_with("add") {
            // !memory add <content> - メモリを追加
            let memory_content = args[3..].trim();

            if memory_content.is_empty() {
                if let Err(e) = msg.reply(ctx, "使い方: !memory add <メモリ内容>").await {
                    error!("Failed to send usage: {}", e);
                }
                return;
            }

            let new_memory = memory_store::NewMemory {
                user_id,
                content: memory_content.to_string(),
            };

            match self.memory_store.add_memory(new_memory) {
                Ok(memory) => {
                    let reply = format!("メモリを追加しました (ID: {}):\n{}", memory.id, memory.content);
                    if let Err(e) = msg.reply(ctx, reply).await {
                        error!("Failed to send add response: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to add memory: {}", e);
                    if let Err(err) = msg.reply(ctx, format!("エラー: {}", e)).await {
                        error!("Failed to send error: {}", err);
                    }
                }
            }
        } else {
            let usage = "使い方:\n\
                - !memory add <メモリ内容> - メモリを追加\n\
                - !memory list - メモリ一覧（最新10件）\n\
                - !memory search <検索ワード> - メモリを検索\n\
                - !memory delete <ID> - メモリを削除\n\
                - !memory clear - 全メモリを削除（確認付き）";
            if let Err(e) = msg.reply(ctx, usage).await {
                error!("Failed to send usage: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // トレーシング初期化
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // 環境変数を取得
    let discord_token = match env::var("DISCORD_BOT_TOKEN") {
        Ok(token) => {
            info!("Discord token loaded");
            token
        }
        Err(e) => {
            error!("DISCORD_BOT_TOKEN not set: {}", e);
            return;
        }
    };

    // GLMクライアントを作成
    let glm_client = match glm::GLMClient::new() {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create GLM client: {}", e);
            return;
        }
    };

    // デフォルトツールを登録
    {
        let tm = glm_client.tool_manager();
        let mut tool_manager = tm.write().await;
        tools::register_default_tools(&mut tool_manager);
        info!("Registered {} tools", tool_manager.list_tools().len());
    }

    // セッションマネージャーを作成（50件履歴、30分タイムアウト）
    let mut session_manager = SessionManager::new(50, Duration::from_secs(30 * 60));

    // セッションストアを初期化し、保存されたセッションを復元
    let session_store = Arc::new(Mutex::new(
        match SessionStore::new("data/sessions.db") {
            Ok(store) => {
                info!("Session store initialized");
                // 保存されたセッションを復元
                match session_manager.load_from_store(&store) {
                    Ok(count) => info!("Restored {} sessions from store", count),
                    Err(e) => error!("Failed to restore sessions: {}", e),
                }
                Some(store)
            }
            Err(e) => {
                error!("Failed to initialize session store: {}", e);
                None
            }
        }
    ));

    let session_manager = Arc::new(Mutex::new(session_manager));

    // 定期的に期限切れセッションをクリーンアップし、保存
    let cleanup_manager = session_manager.clone();
    let cleanup_store = session_store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分ごと
        loop {
            interval.tick().await;
            let mut manager = cleanup_manager.lock().await;
            manager.cleanup_expired();

            // セッションを保存
            let store_guard = cleanup_store.lock().await;
            if let Some(ref store) = *store_guard {
                if let Err(e) = manager.save_to_store(store) {
                    error!("Failed to save sessions: {}", e);
                }
            }
        }
    });

    // スケジューラーを作成
    let scheduler = Arc::new(Scheduler::new());

    // スケジュールストアを読み込み
    let schedule_store = Arc::new(RwLock::new(
        ScheduleStore::load("data").await.unwrap_or_else(|e| {
            error!("Failed to load schedule store: {}, creating new", e);
            ScheduleStore::new()
        })
    ));

    // 保存されたタスクをスケジューラーに復元
    {
        let store = schedule_store.read().await;
        let scheduler_clone = scheduler.clone();
        scheduler_clone.set_tasks(store.get_tasks().to_vec()).await;
    }

    // スケジューラーを開始
    let scheduler_clone = scheduler.clone();

    // HTTPクライアント
    let http = Arc::new(Http::new(&discord_token));

    // スケジュールイベントリスナーを開始（startの前にsubscribe）
    let event_http = http.clone();
    let event_glm = glm_client.clone();
    let mut event_receiver = scheduler_clone.subscribe();

    // スケジューラーを開始
    scheduler_clone.start();

    tokio::spawn(async move {
        info!("Schedule event listener started");
        loop {
            match event_receiver.recv().await {
                Ok(event) => {
                    let task = &event.task;
                    info!("Executing scheduled task: {} in channel {}", task.id, task.channel_id);

                    // GLMに送信
                    let messages = vec![history::ChatMessage::user(&task.prompt)];
                    let tool_context = tool::ToolContext::new(
                        0,  // システム実行
                        "scheduler".to_string(),
                        task.channel_id,
                    );

                    match event_glm.chat_with_tools(messages, &tool_context).await {
                        Ok(response) => {
                            // Discordに送信
                            let channel_id = serenity::model::id::ChannelId::new(task.channel_id);
                            if let Err(e) = channel_id.say(&event_http, &response).await {
                                error!("Failed to send scheduled message: {}", e);
                            } else {
                                info!("Scheduled message sent successfully");
                            }
                        }
                        Err(e) => {
                            error!("GLM error in scheduled task: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Schedule event channel closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Schedule event receiver lagged by {} messages", n);
                }
            }
        }
    });

    // インテントを設定
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    info!("Creating handler...");

    // ハンドラーを作成
    let rate_limiter = Arc::new(Mutex::new(rate_limiter::RateLimiter::new()));

    // パーミッションマネージャーを読み込み
    let mut permission_manager = permission::PermissionManager::load("data").await.unwrap_or_else(|e| {
        error!("Failed to load permission manager: {}, creating new", e);
        permission::PermissionManager::new()
    });
    // 環境変数から管理者を読み込み
    permission_manager.load_admins_from_env();
    let permission_manager = Arc::new(RwLock::new(permission_manager));

    // メモリストアを読み込み
    let memory_store = Arc::new(MemoryStore::load("data").unwrap_or_else(|e| {
        error!("Failed to load memory store: {}, creating new", e);
        MemoryStore::new().expect("Failed to create memory store")
    }));

    // メモリツールを登録
    {
        let tm = glm_client.tool_manager();
        let mut tool_manager = tm.write().await;
        tools::register_memory_tools(&mut tool_manager, memory_store.clone());
        info!("Registered {} tools total", tool_manager.list_tools().len());
    }

    let handler = Handler {
        glm_client,
        session_manager,
        scheduler,
        schedule_store,
        rate_limiter,
        permission_manager,
        memory_store,
        http,
        processed_messages: Arc::new(Mutex::new(HashSet::new())),
    };

    info!("Creating client...");

    // クライアントを作成
    let mut client = match Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await
    {
        Ok(client) => {
            info!("Client created");
            client
        }
        Err(why) => {
            error!("Error creating client: {:?}", why);
            return;
        }
    };

    info!("Starting bot...");

    // 起動
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
