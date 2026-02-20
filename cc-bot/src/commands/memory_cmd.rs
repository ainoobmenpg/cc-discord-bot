//! /memory - メモリ管理Slash Command

use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use std::fs;
use std::path::Path;

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
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "category", "カテゴリ (デフォルト: general)")
                        .required(false),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "tag", "タグ (カンマ区切りで複数指定可)")
                        .required(false),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "メモリ一覧")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "category", "カテゴリでフィルタ")
                        .required(false),
                ),
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
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "export", "メモリエクスポート")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "format", "形式 (markdown または json)")
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
        Some(("list", sub_opts)) => handle_list(user_id, sub_opts, &handler.memory_store),
        Some(("search", sub_opts)) => handle_search(user_id, sub_opts, &handler.memory_store),
        Some(("delete", sub_opts)) => handle_delete(user_id, sub_opts, &handler.memory_store),
        Some(("export", sub_opts)) => handle_export(user_id, sub_opts, &handler.memory_store, &handler.base_output_dir),
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

    // categoryオプションを取得（オプション）
    let category = sub_opts
        .iter()
        .find(|opt| opt.name == "category")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        });

    // tagオプションを取得（オプション、カンマ区切りで複数指定可）
    let tags: Option<Vec<String>> = sub_opts
        .iter()
        .find(|opt| opt.name == "tag")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                let parsed: Vec<String> = s
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                if parsed.is_empty() {
                    None
                } else {
                    Some(parsed)
                }
            } else {
                None
            }
        });

    let new_memory = memory_store::NewMemory {
        user_id,
        content: content.to_string(),
        category: category.map(|s| s.to_string()),
        tags: tags.clone(),
        ..Default::default()
    };

    match memory_store.add_memory(new_memory) {
        Ok(memory) => {
            let category_display = if memory.category == "general" {
                String::new()
            } else {
                format!(" [{}]", memory.category)
            };
            let tags_display = if memory.tags.is_empty() {
                String::new()
            } else {
                format!(" #{}", memory.tags.join(" #"))
            };
            format!(
                "メモリを追加しました (ID: {}){}{}:\n{}",
                memory.id, category_display, tags_display, memory.content
            )
        }
        Err(e) => format!("エラー: {}", e),
    }
}

/// メモリ一覧
fn handle_list(
    user_id: u64,
    sub_opts: &[serenity::model::application::CommandDataOption],
    memory_store: &memory_store::MemoryStore,
) -> String {
    // categoryオプションを取得（オプション）
    let category = sub_opts
        .iter()
        .find(|opt| opt.name == "category")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.as_str())
            } else {
                None
            }
        });

    let memories = match category {
        Some(cat) => memory_store.list_memories_by_category(user_id, cat, 10),
        None => memory_store.list_memories(user_id, 10),
    };

    match memories {
        Ok(memories) => {
            if memories.is_empty() {
                match category {
                    Some(cat) => format!("カテゴリ「{}」のメモリがありません。", cat),
                    None => "メモリがありません。".to_string(),
                }
            } else {
                let list: Vec<String> = memories
                    .iter()
                    .map(|m| {
                        let date = m.created_at.format("%m/%d %H:%M");
                        let category_display = if m.category == "general" {
                            String::new()
                        } else {
                            format!("[{}] ", m.category)
                        };
                        format!("- [{}] {}{} (ID: {})", date, category_display, m.content, m.id)
                    })
                    .collect();

                let header = match category {
                    Some(cat) => format!("**あなたのメモリ [{}] ({}件)**", cat, memories.len()),
                    None => format!("**あなたのメモリ ({}件)**", memories.len()),
                };

                format!("{}\n{}", header, list.join("\n"))
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

/// メモリエクスポート
fn handle_export(
    user_id: u64,
    sub_opts: &[serenity::model::application::CommandDataOption],
    memory_store: &memory_store::MemoryStore,
    base_output_dir: &str,
) -> String {
    // formatオプションを取得
    let format = sub_opts
        .iter()
        .find(|opt| opt.name == "format")
        .and_then(|opt| {
            if let CommandDataOptionValue::String(s) = &opt.value {
                Some(s.to_lowercase())
            } else {
                None
            }
        })
        .unwrap_or_default();

    // エクスポートディレクトリを作成
    let exports_dir = Path::new(base_output_dir).join("exports");
    if let Err(e) = fs::create_dir_all(&exports_dir) {
        return format!("エクスポートディレクトリの作成に失敗しました: {}", e);
    }

    // タイムスタンプ生成
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

    match format.as_str() {
        "markdown" | "md" => {
            match memory_store.export_to_markdown(user_id) {
                Ok(content) => {
                    let filename = format!("memories_{}_{}.md", user_id, timestamp);
                    let filepath = exports_dir.join(&filename);

                    match fs::write(&filepath, &content) {
                        Ok(_) => {
                            format!(
                                "メモリをMarkdown形式でエクスポートしました。\nファイル: {}",
                                filepath.display()
                            )
                        }
                        Err(e) => format!("ファイルの書き込みに失敗しました: {}", e),
                    }
                }
                Err(e) => format!("エクスポートエラー: {}", e),
            }
        }
        "json" => {
            match memory_store.export_to_json(user_id) {
                Ok(content) => {
                    let filename = format!("memories_{}_{}.json", user_id, timestamp);
                    let filepath = exports_dir.join(&filename);

                    match fs::write(&filepath, &content) {
                        Ok(_) => {
                            format!(
                                "メモリをJSON形式でエクスポートしました。\nファイル: {}",
                                filepath.display()
                            )
                        }
                        Err(e) => format!("ファイルの書き込みに失敗しました: {}", e),
                    }
                }
                Err(e) => format!("エクスポートエラー: {}", e),
            }
        }
        _ => "形式は 'markdown' または 'json' を指定してください。".to_string(),
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
