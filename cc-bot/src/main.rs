mod glm;

use serenity::prelude::*;
use serenity::model::channel::Message;
use std::env;
use tracing::{debug, error, info};

struct Handler {
    glm_client: glm::GLMClient,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        info!("Received message: {}", msg.content);

        // メッセージがボット自身の場合は無視
        if msg.author.bot {
            debug!("Ignoring bot message");
            return;
        }

        // !ask で始まるメッセージに応答
        if msg.content.starts_with("!ask") {
            let prompt = msg.content[4..].trim();
            info!("Prompt: {}", prompt);

            if prompt.is_empty() {
                if let Err(e) = msg.reply(&ctx, "使い方: !ask <質問>").await {
                    error!("Failed to send usage message: {}", e);
                }
                return;
            }

            info!("Sending to GLM-4.7...");

            // GLM-4.7に送信
            let response = self.glm_client.chat(prompt).await;

            match response {
                Ok(reply) => {
                    info!("Response: {}", reply);
                    if let Err(e) = msg.reply(&ctx, reply).await {
                        error!("Failed to send response: {}", e);
                    }
                }
                Err(e) => {
                    error!("GLM error: {}", e);
                    let error_msg = format!("エラーが発生しました: {}", e);
                    if let Err(send_err) = msg.reply(&ctx, error_msg).await {
                        error!("Failed to send error message: {}", send_err);
                    }
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: serenity::model::gateway::Ready) {
        info!("{} is connected!", ready.user.name);
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

    // インテントを設定
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    info!("Creating handler...");

    // ハンドラーを作成
    let handler = Handler { glm_client };

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
