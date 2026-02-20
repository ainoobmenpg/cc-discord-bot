mod api;
mod commands;
mod datetime_utils;
mod glm;
mod history;
mod memory;
mod memory_store;
mod permission;
mod persistent_store;
mod rate_limiter;
mod role_config;
mod scheduler;
mod schedule_store;
mod session;
mod tool;
mod tools;
mod user_roles;
mod user_settings;
mod validation;

use memory_store::MemoryStore;
use scheduler::Scheduler;
use schedule_store::ScheduleStore;
use serenity::http::Http;
use serenity::model::application::{Interaction, CommandInteraction};
use serenity::prelude::*;
use serenity::model::channel::Message;
use session::{SessionManager, SessionStore};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, broadcast};
use tracing::{debug, error, info, warn};

pub struct Handler {
    glm_client: glm::GLMClient,
    session_manager: Arc<Mutex<SessionManager>>,
    scheduler: Arc<Scheduler>,
    schedule_store: Arc<RwLock<ScheduleStore>>,
    #[allow(dead_code)]
    rate_limiter: Arc<Mutex<rate_limiter::RateLimiter>>,
    permission_manager: Arc<RwLock<permission::PermissionManager>>,
    memory_store: Arc<MemoryStore>,
    /// ユーザー設定ストア
    pub user_settings_store: Arc<user_settings::UserSettingsStore>,
    #[allow(dead_code)]
    http: Arc<Http>,
    /// 処理済みメッセージID（重複防止）
    #[allow(dead_code)]
    processed_messages: Arc<Mutex<HashSet<u64>>>,
    /// ツール出力のベースディレクトリ
    pub base_output_dir: String,
    /// ロール設定
    pub role_config: Arc<RwLock<role_config::RoleConfig>>,
    /// ユーザーロールキャッシュ
    pub user_role_cache: user_roles::UserRoleCache,
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
            "settings" => commands::settings::run(ctx, command, self).await,
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

    // ツール出力ディレクトリを環境変数から取得（デフォルト: /tmp/cc-bot）
    let base_output_dir = env::var("BASE_OUTPUT_DIR").unwrap_or_else(|_| "/tmp/cc-bot".to_string());
    debug!("Base output directory: {}", base_output_dir);

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
                        "output".to_string(),  // base_output_dir
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

    // ロール設定を読み込み
    let role_config = Arc::new(RwLock::new(
        role_config::RoleConfig::load("data").await.unwrap_or_else(|e| {
            error!("Failed to load role config: {}, creating new", e);
            role_config::RoleConfig::new()
        })
    ));

    // ユーザー設定ストアを読み込み
    let user_settings_store = Arc::new(
        user_settings::UserSettingsStore::load("data").unwrap_or_else(|e| {
            error!("Failed to load user settings store: {}, creating new", e);
            user_settings::UserSettingsStore::new().expect("Failed to create user settings store")
        })
    );

    // ユーザーロールキャッシュを作成
    let user_role_cache = user_roles::UserRoleCache::new();

    // 定期的にユーザーロールキャッシュをクリーンアップ
    let role_cache_cleanup = user_role_cache.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分ごと
        loop {
            interval.tick().await;
            role_cache_cleanup.cleanup_expired().await;
        }
    });

    let handler = Handler {
        glm_client: glm_client.clone(),
        session_manager: session_manager.clone(),
        scheduler: scheduler.clone(),
        schedule_store: schedule_store.clone(),
        rate_limiter,
        permission_manager,
        memory_store: memory_store.clone(),
        user_settings_store: user_settings_store.clone(),
        http,
        processed_messages: Arc::new(Mutex::new(HashSet::new())),
        base_output_dir: base_output_dir.clone(),
        role_config,
        user_role_cache,
    };

    // APIサーバーを並行起動
    let api_port: u16 = env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let api_state = api::ApiState {
        glm_client,
        session_manager,
        scheduler,
        schedule_store,
        memory_store,
        base_output_dir,
    };

    tokio::spawn(async move {
        api::start_server(api_state, api_port).await;
    });

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
