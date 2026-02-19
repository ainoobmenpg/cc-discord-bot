//! cc-cli - cc-discord-bot CLIツール
//!
//! HTTP APIを叩いてGLM-4.7と対話したり、スケジュール・メモリを管理する

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

/// cc-discord-bot CLI
#[derive(Parser)]
#[command(name = "cc-cli")]
#[command(about = "CLI tool for cc-discord-bot", long_about = None)]
struct Cli {
    /// API server URL (default: http://localhost:3000)
    #[arg(short, long, env = "CC_API_URL", default_value = "http://localhost:3000")]
    url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Chat with GLM-4.7
    Chat {
        /// Message to send
        message: String,
        /// User ID (optional)
        #[arg(short, long, default_value = "0")]
        user: u64,
        /// Channel ID (optional)
        #[arg(short, long, default_value = "0")]
        channel: u64,
    },
    /// Manage schedules
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
    },
    /// Manage memories
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },
    /// Check API health
    Health,
}

#[derive(Subcommand)]
enum ScheduleCommands {
    /// List all schedules
    List,
    /// Create a new schedule
    Add {
        /// Cron expression
        cron: String,
        /// Prompt to execute
        prompt: String,
        /// Channel ID
        #[arg(short, long, default_value = "0")]
        channel: u64,
    },
    /// Delete a schedule
    Delete {
        /// Schedule ID
        id: String,
    },
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// List memories
    List {
        /// User ID filter
        #[arg(short, long, default_value = "0")]
        user: u64,
        /// Limit
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Add a memory
    Add {
        /// User ID
        #[arg(short, long)]
        user: u64,
        /// Memory content
        content: String,
    },
    /// Search memories
    Search {
        /// Search query
        query: String,
        /// User ID filter
        #[arg(short, long, default_value = "0")]
        user: u64,
    },
    /// Delete a memory
    Delete {
        /// Memory ID
        id: i64,
    },
}

// ===== API Response Types =====

#[derive(Deserialize)]
struct ChatResponse {
    response: String,
}

#[derive(Deserialize)]
struct ScheduleResponse {
    id: String,
    cron: String,
    prompt: String,
    channel_id: u64,
    next_run: Option<String>,
}

#[derive(Serialize)]
struct CreateScheduleRequest {
    cron: String,
    prompt: String,
    channel_id: u64,
}

#[derive(Deserialize)]
struct MemoryResponse {
    id: i64,
    user_id: u64,
    content: String,
    created_at: String,
}

#[derive(Serialize)]
struct CreateMemoryRequest {
    user_id: u64,
    content: String,
}

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Chat { message, user, channel } => {
            chat_command(&client, &cli.url, &message, user, channel).await?;
        }
        Commands::Schedule { command } => {
            handle_schedule(&client, &cli.url, command).await?;
        }
        Commands::Memory { command } => {
            handle_memory(&client, &cli.url, command).await?;
        }
        Commands::Health => {
            health_command(&client, &cli.url).await?;
        }
    }

    Ok(())
}

async fn chat_command(client: &Client, base_url: &str, message: &str, user_id: u64, channel_id: u64) -> Result<()> {
    #[derive(Serialize)]
    struct Request {
        message: String,
        user_id: u64,
        channel_id: u64,
    }

    let resp = client
        .post(&format!("{}/api/chat", base_url))
        .json(&Request {
            message: message.to_string(),
            user_id,
            channel_id,
        })
        .send()
        .await?;

    if resp.status().is_success() {
        let chat: ChatResponse = resp.json().await?;
        println!("{}", chat.response);
    } else {
        let text = resp.text().await?;
        eprintln!("{}: {}", "Error".red(), text);
    }

    Ok(())
}

async fn handle_schedule(client: &Client, base_url: &str, command: ScheduleCommands) -> Result<()> {
    match command {
        ScheduleCommands::List => {
            let resp = client
                .get(&format!("{}/api/schedules", base_url))
                .send()
                .await?;

            if resp.status().is_success() {
                let schedules: Vec<ScheduleResponse> = resp.json().await?;
                if schedules.is_empty() {
                    println!("No schedules found.");
                } else {
                    println!("{}", "Schedules:".green().bold());
                    for s in schedules {
                        let next = s.next_run.as_deref().unwrap_or("N/A");
                        println!("  {} [{}] {} (next: {})",
                            s.id.yellow(),
                            s.cron.cyan(),
                            s.prompt,
                            next.dimmed()
                        );
                    }
                }
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
        ScheduleCommands::Add { cron, prompt, channel } => {
            let resp = client
                .post(&format!("{}/api/schedules", base_url))
                .json(&CreateScheduleRequest {
                    cron,
                    prompt,
                    channel_id: channel,
                })
                .send()
                .await?;

            if resp.status().is_success() {
                let schedule: ScheduleResponse = resp.json().await?;
                println!("{} Schedule created:", "✓".green());
                println!("  ID: {}", schedule.id.yellow());
                println!("  Next run: {}", schedule.next_run.as_deref().unwrap_or("N/A"));
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
        ScheduleCommands::Delete { id } => {
            let resp = client
                .delete(&format!("{}/api/schedules/{}", base_url, id))
                .send()
                .await?;

            if resp.status().is_success() {
                println!("{} Schedule deleted: {}", "✓".green(), id);
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
    }

    Ok(())
}

async fn handle_memory(client: &Client, base_url: &str, command: MemoryCommands) -> Result<()> {
    match command {
        MemoryCommands::List { user, limit } => {
            let resp = client
                .get(&format!("{}/api/memories?user_id={}&limit={}", base_url, user, limit))
                .send()
                .await?;

            if resp.status().is_success() {
                let memories: Vec<MemoryResponse> = resp.json().await?;
                if memories.is_empty() {
                    println!("No memories found.");
                } else {
                    println!("{}", "Memories:".green().bold());
                    for m in memories {
                        println!("  [{}] (user {}) {}",
                            m.id.to_string().yellow(),
                            m.user_id,
                            m.content
                        );
                    }
                }
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
        MemoryCommands::Add { user, content } => {
            let resp = client
                .post(&format!("{}/api/memories", base_url))
                .json(&CreateMemoryRequest {
                    user_id: user,
                    content,
                })
                .send()
                .await?;

            if resp.status().is_success() {
                let memory: MemoryResponse = resp.json().await?;
                println!("{} Memory created:", "✓".green());
                println!("  ID: {}", memory.id.to_string().yellow());
                println!("  Content: {}", memory.content);
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
        MemoryCommands::Search { query, user } => {
            let resp = client
                .get(&format!("{}/api/memories/search?q={}&user_id={}", base_url, query, user))
                .send()
                .await?;

            if resp.status().is_success() {
                let memories: Vec<MemoryResponse> = resp.json().await?;
                if memories.is_empty() {
                    println!("No memories found for query: {}", query);
                } else {
                    println!("{} results for '{}':", memories.len().to_string().green(), query.cyan());
                    for m in memories {
                        println!("  [{}] {}", m.id.to_string().yellow(), m.content);
                    }
                }
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
        MemoryCommands::Delete { id } => {
            let resp = client
                .delete(&format!("{}/api/memories/{}", base_url, id))
                .send()
                .await?;

            if resp.status().is_success() {
                println!("{} Memory deleted: {}", "✓".green(), id);
            } else {
                let text = resp.text().await?;
                eprintln!("{}: {}", "Error".red(), text);
            }
        }
    }

    Ok(())
}

async fn health_command(client: &Client, base_url: &str) -> Result<()> {
    let resp = client
        .get(&format!("{}/api/health", base_url))
        .send()
        .await?;

    if resp.status().is_success() {
        let health: HealthResponse = resp.json().await?;
        println!("{} API is healthy", "✓".green());
        println!("  Status: {}", health.status);
        println!("  Version: {}", health.version);
    } else {
        let text = resp.text().await?;
        eprintln!("{}: {}", "Error".red(), text);
    }

    Ok(())
}
