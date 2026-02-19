//! /memory - メモリ管理Slash Command

use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;

use crate::memory_store;
use crate::Handler;

/// /memory コマンドの定義
pub fn register() -> CreateCommand {
    CreateCommand::new("memory")
        .description("メモリ管理")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "メモリ追加")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "content", "内容")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "メモリ一覧"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "search", "メモリ検索")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "query", "検索ワード")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "メモリ削除")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Integer, "id", "メモリID")
                        .required(true),
                ),
        )
}

/// /memory コマンドの実行
pub async fn run(_ctx: &Context, interaction: &CommandInteraction, handler: &Handler) -> String {
    let user_id = interaction.user.id.get();

    // サブコマンドを取得
    let subcommand = interaction
        .data
        .options
        .first()
        .and_then(|opt| {
            if let CommandDataOptionValue::SubCommand(sub_opts) = &opt.value {
                Some((opt.name.as_str(), sub_opts))
            } else {
                None
            }
        });

    match subcommand {
        Some(("add", sub_opts)) => handle_add(user_id, sub_opts, &handler.memory_store),
        Some(("list", _)) => handle_list(user_id, &handler.memory_store),
        Some(("search", sub_opts)) => handle_search(user_id, sub_opts, &handler.memory_store),
        Some(("delete", sub_opts)) => handle_delete(user_id, sub_opts, &handler.memory_store),
        _ => "不明なサブコマンドです。".to_string(),
    }
}

/// メモリ追加
fn handle_add(
    user_id: u64,
    sub_opts: &[serenity::model::application::CommandDataOption],
    memory_store: &memory_store::MemoryStore,
) -> String {
    // contentオプションを取得
    let content = sub_opts
        .iter()
        .find(|opt| opt.name == "content")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if content.trim().is_empty() {
        return "メモリ内容を入力してください。".to_string();
    }

    let new_memory = memory_store::NewMemory {
        user_id,
        content: content.to_string(),
    };

    match memory_store.add_memory(new_memory) {
        Ok(memory) => format!("メモリを追加しました (ID: {}):\n{}", memory.id, memory.content),
        Err(e) => format!("エラー: {}", e),
    }
}

/// メモリ一覧
fn handle_list(user_id: u64, memory_store: &memory_store::MemoryStore) -> String {
    match memory_store.list_memories(user_id, 10) {
        Ok(memories) => {
            if memories.is_empty() {
                "メモリがありません。".to_string()
            } else {
                let list: Vec<String> = memories
                    .iter()
                    .map(|m| {
                        let date = m.created_at.format("%m/%d %H:%M");
                        format!("- [{}] {} (ID: {})", date, m.content, m.id)
                    })
                    .collect();

                format!(
                    "**あなたのメモリ ({}件)**\n{}",
                    memories.len(),
                    list.join("\n")
                )
            }
        }
        Err(e) => format!("エラー: {}", e),
    }
}

/// メモリ検索
fn handle_search(
    user_id: u64,
    sub_opts: &[serenity::model::application::CommandDataOption],
    memory_store: &memory_store::MemoryStore,
) -> String {
    // queryオプションを取得
    let query = sub_opts
        .iter()
        .find(|opt| opt.name == "query")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    if query.trim().is_empty() {
        return "検索ワードを入力してください。".to_string();
    }

    match memory_store.search_memories(user_id, query) {
        Ok(memories) => {
            if memories.is_empty() {
                format!("「{}」に一致するメモリがありません。", query)
            } else {
                let list: Vec<String> = memories
                    .iter()
                    .map(|m| {
                        let date = m.created_at.format("%m/%d %H:%M");
                        format!("- [{}] {} (ID: {})", date, m.content, m.id)
                    })
                    .collect();

                format!(
                    "**検索結果: {} ({}件)**\n{}",
                    query,
                    memories.len(),
                    list.join("\n")
                )
            }
        }
        Err(e) => format!("エラー: {}", e),
    }
}

/// メモリ削除
fn handle_delete(
    user_id: u64,
    sub_opts: &[serenity::model::application::CommandDataOption],
    memory_store: &memory_store::MemoryStore,
) -> String {
    // idオプションを取得
    let id = sub_opts
        .iter()
        .find(|opt| opt.name == "id")
        .and_then(|opt| {
            if let CommandDataOptionValue::Integer(i) = opt.value {
                Some(i)
            } else {
                None
            }
        });

    match id {
        Some(memory_id) => match memory_store.delete_memory(user_id, memory_id as i64) {
            Ok(deleted) => format!("メモリを削除しました: {}", deleted.content),
            Err(memory_store::MemoryError::NotFound(_)) => {
                "指定されたメモリが見つかりません。".to_string()
            }
            Err(memory_store::MemoryError::PermissionDenied(_)) => {
                "他のユーザーのメモリは削除できません。".to_string()
            }
            Err(e) => format!("エラー: {}", e),
        },
        None => "IDを指定してください。".to_string(),
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
