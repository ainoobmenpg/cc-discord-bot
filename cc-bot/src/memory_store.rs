use crate::datetime_utils::parse_rfc3339_or_now;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use tracing::{debug, error, info};

/// メモリID
pub type MemoryId = i64;

/// メモリエラー
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Memory not found: {0}")]
    NotFound(MemoryId),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// メモリエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub user_id: u64,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新規メモリ作成用
#[derive(Debug, Clone, Default)]
pub struct NewMemory {
    pub user_id: u64,
    pub content: String,
    #[allow(dead_code)]
    pub category: Option<String>,
    #[allow(dead_code)]
    pub tags: Option<Vec<String>>,
    #[allow(dead_code)]
    pub metadata: Option<HashMap<String, String>>,
}

/// メモリストア（SQLite永続化）
pub struct MemoryStore {
    conn: Mutex<Connection>,
}

impl MemoryStore {
    /// Mutexロックを取得するヘルパー
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, MemoryError> {
        self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })
    }

    /// 新しいMemoryStoreを作成
    pub fn new() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            error!("Failed to create in-memory DB: {}", e);
            MemoryError::DatabaseError("Failed to create database".to_string())
        })?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.initialize()?;
        Ok(store)
    }

    /// ファイルパスから読み込み
    pub fn load(base_dir: &str) -> Result<Self, MemoryError> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading memory store from {:?}", path);

        // 親ディレクトリを作成（create_dir_allは内部で存在確認を行う）
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                error!("Failed to create directory: {}", e);
                MemoryError::DatabaseError("Failed to initialize storage".to_string())
            })?;
        }

        let conn = Connection::open(&path).map_err(|e| {
            error!("Failed to open database at {:?}: {}", path, e);
            MemoryError::DatabaseError("Failed to open database".to_string())
        })?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.initialize()?;
        info!("Memory store loaded successfully");
        Ok(store)
    }

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join("memories.db")
    }

    /// データベースを初期化
    fn initialize(&self) -> Result<(), MemoryError> {
        let conn = self.lock_conn()?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'general',
                tags TEXT NOT NULL DEFAULT '[]',
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create table: {}", e)))?;

        // マイグレーション: 古いテーブルに新しいカラムを追加
        self.run_migrations(&conn)?;

        // 検索用インデックス
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_id ON memories(user_id)",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON memories(created_at)",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_category ON memories(category)",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create index: {}", e)))?;

        debug!("Memory store initialized");
        Ok(())
    }

    /// マイグレーションを実行
    fn run_migrations(&self, conn: &Connection) -> Result<(), MemoryError> {
        // カラムの存在チェックを行い、存在しなければ追加
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(memories)")
            .and_then(|mut stmt| {
                let column_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
                column_iter.collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to get table info: {}", e)))?;

        // categoryカラムがない場合は追加
        if !columns.iter().any(|c| c == "category") {
            conn.execute(
                "ALTER TABLE memories ADD COLUMN category TEXT NOT NULL DEFAULT 'general'",
                [],
            ).map_err(|e| MemoryError::DatabaseError(format!("Failed to add category column: {}", e)))?;
            info!("Migration: Added category column");
        }

        // tagsカラムがない場合は追加
        if !columns.iter().any(|c| c == "tags") {
            conn.execute(
                "ALTER TABLE memories ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
                [],
            ).map_err(|e| MemoryError::DatabaseError(format!("Failed to add tags column: {}", e)))?;
            info!("Migration: Added tags column");
        }

        // metadataカラムがない場合は追加
        if !columns.iter().any(|c| c == "metadata") {
            conn.execute(
                "ALTER TABLE memories ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}'",
                [],
            ).map_err(|e| MemoryError::DatabaseError(format!("Failed to add metadata column: {}", e)))?;
            info!("Migration: Added metadata column");
        }

        Ok(())
    }

    /// メモリを追加
    pub fn add_memory(&self, new_memory: NewMemory) -> Result<Memory, MemoryError> {
        if new_memory.content.trim().is_empty() {
            return Err(MemoryError::InvalidInput("Content cannot be empty".to_string()));
        }

        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let updated_at = now.to_rfc3339();
        let category = new_memory.category.clone().unwrap_or_else(|| "general".to_string());
        let tags = serde_json::to_string(&new_memory.tags.clone().unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string());
        let metadata = serde_json::to_string(&new_memory.metadata.clone().unwrap_or_default())
            .unwrap_or_else(|_| "{}".to_string());

        let conn = self.lock_conn()?;

        conn.execute(
            "INSERT INTO memories (user_id, content, category, tags, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![new_memory.user_id as i64, new_memory.content, category, tags, metadata, created_at, updated_at],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to insert memory: {}", e)))?;

        let id = conn.last_insert_rowid();

        Ok(Memory {
            id,
            user_id: new_memory.user_id,
            content: new_memory.content,
            category,
            tags: new_memory.tags.unwrap_or_default(),
            metadata: new_memory.metadata.unwrap_or_default(),
            created_at: now,
            updated_at: now,
        })
    }

    /// ユーザーのメモリ一覧を取得（最新10件）
    pub fn list_memories(&self, user_id: u64, limit: usize) -> Result<Vec<Memory>, MemoryError> {
        self.list_memories_with_offset(user_id, limit, 0)
    }

    /// ユーザーのメモリ一覧を取得（ページネーション対応）
    ///
    /// # Arguments
    /// * `user_id` - ユーザーID
    /// * `limit` - 取得する最大件数
    /// * `offset` - スキップする件数（ページネーション用）
    pub fn list_memories_with_offset(
        &self,
        user_id: u64,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>, MemoryError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, content, category, tags, metadata, created_at, updated_at
                 FROM memories
                 WHERE user_id = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let memories = stmt
            .query_map(params![user_id as i64, limit as i64, offset as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    user_id: row.get::<_, i64>(1)? as u64,
                    content: row.get(2)?,
                    category: row.get(3)?,
                    tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    created_at: parse_rfc3339_or_now(&row.get::<_, String>(6)?),
                    updated_at: parse_rfc3339_or_now(&row.get::<_, String>(7)?),
                })
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memories: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to collect memories: {}", e)))?;

        Ok(memories)
    }

    /// カテゴリでフィルタリングしてメモリ一覧を取得
    pub fn list_memories_by_category(
        &self,
        user_id: u64,
        category: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, MemoryError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, content, category, tags, metadata, created_at, updated_at
                 FROM memories
                 WHERE user_id = ?1 AND category = ?2
                 ORDER BY created_at DESC
                 LIMIT ?3",
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let memories = stmt
            .query_map(params![user_id as i64, category, limit as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    user_id: row.get::<_, i64>(1)? as u64,
                    content: row.get(2)?,
                    category: row.get(3)?,
                    tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    created_at: parse_rfc3339_or_now(&row.get::<_, String>(6)?),
                    updated_at: parse_rfc3339_or_now(&row.get::<_, String>(7)?),
                })
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memories: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to collect memories: {}", e)))?;

        Ok(memories)
    }

    /// LIKEパターンの特殊文字をエスケープ
    fn escape_like_pattern(pattern: &str) -> String {
        pattern
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    }

    /// メモリを検索（前方一致検索でインデックスを活用）
    pub fn search_memories(&self, user_id: u64, query: &str) -> Result<Vec<Memory>, MemoryError> {
        if query.trim().is_empty() {
            return self.list_memories(user_id, 10);
        }

        let conn = self.lock_conn()?;

        let escaped_query = Self::escape_like_pattern(query.trim());
        // 前方一致検索を使用（インデックスが有効）
        let search_pattern = format!("{}%", escaped_query);

        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, content, category, tags, metadata, created_at, updated_at
                 FROM memories
                 WHERE user_id = ?1 AND content LIKE ?2 ESCAPE '\\'
                 ORDER BY created_at DESC
                 LIMIT 10",
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let memories = stmt
            .query_map(params![user_id as i64, search_pattern], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    user_id: row.get::<_, i64>(1)? as u64,
                    content: row.get(2)?,
                    category: row.get(3)?,
                    tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    created_at: parse_rfc3339_or_now(&row.get::<_, String>(6)?),
                    updated_at: parse_rfc3339_or_now(&row.get::<_, String>(7)?),
                })
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memories: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to collect memories: {}", e)))?;

        Ok(memories)
    }

    /// メモリを削除（ユーザー確認付き）
    pub fn delete_memory(&self, user_id: u64, id: MemoryId) -> Result<Memory, MemoryError> {
        let conn = self.lock_conn()?;

        // まず対象を取得してユーザー確認
        let memory: Option<Memory> = conn
            .query_row(
                "SELECT id, user_id, content, category, tags, metadata, created_at, updated_at FROM memories WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Memory {
                        id: row.get(0)?,
                        user_id: row.get::<_, i64>(1)? as u64,
                        content: row.get(2)?,
                        category: row.get(3)?,
                        tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                        metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .optional()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memory: {}", e)))?;

        let memory = memory.ok_or_else(|| MemoryError::NotFound(id))?;

        // ユーザー確認
        if memory.user_id != user_id {
            return Err(MemoryError::PermissionDenied(
                "Cannot delete another user's memory".to_string(),
            ));
        }

        // 削除実行
        conn.execute("DELETE FROM memories WHERE id = ?1", params![id])
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to delete memory: {}", e)))?;

        Ok(memory)
    }

    /// ユーザーの全メモリを削除
    #[allow(dead_code)]
    pub fn clear_memories(&self, user_id: u64) -> Result<usize, MemoryError> {
        let conn = self.lock_conn()?;

        let affected = conn
            .execute("DELETE FROM memories WHERE user_id = ?1", params![user_id as i64])
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to clear memories: {}", e)))?;

        Ok(affected)
    }

    /// ユーザーのメモリ数を取得
    #[allow(dead_code)]
    pub fn count_memories(&self, user_id: u64) -> Result<usize, MemoryError> {
        let conn = self.lock_conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE user_id = ?1",
                params![user_id as i64],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to count memories: {}", e)))?;

        Ok(count as usize)
    }

    /// ユーザーの全メモリを取得（エクスポート用）
    pub fn get_all_memories(&self, user_id: u64) -> Result<Vec<Memory>, MemoryError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, content, category, tags, metadata, created_at, updated_at
                 FROM memories
                 WHERE user_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let memories = stmt
            .query_map(params![user_id as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    user_id: row.get::<_, i64>(1)? as u64,
                    content: row.get(2)?,
                    category: row.get(3)?,
                    tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    created_at: parse_rfc3339_or_now(&row.get::<_, String>(6)?),
                    updated_at: parse_rfc3339_or_now(&row.get::<_, String>(7)?),
                })
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memories: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to collect memories: {}", e)))?;

        Ok(memories)
    }

    /// メモリをMarkdown形式でエクスポート
    pub fn export_to_markdown(&self, user_id: u64) -> Result<String, MemoryError> {
        let memories = self.get_all_memories(user_id)?;

        let mut output = String::new();
        output.push_str("# Memory Export\n\n");
        output.push_str(&format!("**User ID:** {}\n", user_id));
        output.push_str(&format!("**Export Date:** {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        output.push_str(&format!("**Total Memories:** {}\n\n", memories.len()));
        output.push_str("---\n\n");

        if memories.is_empty() {
            output.push_str("*No memories found.*\n");
        } else {
            for memory in &memories {
                output.push_str(&format!("## Memory #{}\n\n", memory.id));
                output.push_str(&format!("**Category:** {}\n\n", memory.category));

                if !memory.tags.is_empty() {
                    output.push_str(&format!("**Tags:** {}\n\n", memory.tags.join(", ")));
                }

                if !memory.metadata.is_empty() {
                    output.push_str("**Metadata:**\n");
                    for (key, value) in &memory.metadata {
                        output.push_str(&format!("- {}: {}\n", key, value));
                    }
                    output.push('\n');
                }

                output.push_str(&format!("**Content:**\n{}\n\n", memory.content));
                output.push_str(&format!("**Created:** {}\n", memory.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
                output.push_str(&format!("**Updated:** {}\n", memory.updated_at.format("%Y-%m-%d %H:%M:%S UTC")));
                output.push_str("\n---\n\n");
            }
        }

        Ok(output)
    }

    /// メモリをJSON形式でエクスポート
    pub fn export_to_json(&self, user_id: u64) -> Result<String, MemoryError> {
        let memories = self.get_all_memories(user_id)?;

        #[derive(Serialize)]
        struct ExportData {
            user_id: u64,
            export_date: String,
            total_memories: usize,
            memories: Vec<Memory>,
        }

        let export_data = ExportData {
            user_id,
            export_date: Utc::now().to_rfc3339(),
            total_memories: memories.len(),
            memories,
        };

        serde_json::to_string_pretty(&export_data)
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to serialize JSON: {}", e)))
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default MemoryStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_list_memory() {
        let store = MemoryStore::new().unwrap();

        let memory = store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Test memory".to_string(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(memory.user_id, 12345);
        assert_eq!(memory.content, "Test memory");

        let memories = store.list_memories(12345, 10).unwrap();
        assert_eq!(memories.len(), 1);
    }

    #[test]
    fn test_search_memory() {
        let store = MemoryStore::new().unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Hello world".to_string(),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Goodbye moon".to_string(),
                ..Default::default()
            })
            .unwrap();

        // 前方一致検索のテスト（"Hello"で始まるコンテンツを検索）
        let results = store.search_memories(12345, "Hello").unwrap();
        assert_eq!(results.len(), 1, "Should find memory starting with 'Hello'");
        assert!(results[0].content.contains("Hello"));

        // "world"で始まらないので見つからない
        let results = store.search_memories(12345, "world").unwrap();
        assert_eq!(results.len(), 0, "Should not find memory not starting with 'world'");
    }

    #[test]
    fn test_delete_memory() {
        let store = MemoryStore::new().unwrap();

        let memory = store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "To be deleted".to_string(),
                ..Default::default()
            })
            .unwrap();

        // 他のユーザーは削除できない
        let result = store.delete_memory(99999, memory.id);
        assert!(matches!(result, Err(MemoryError::PermissionDenied(_))));

        // 本人は削除できる
        let deleted = store.delete_memory(12345, memory.id).unwrap();
        assert_eq!(deleted.id, memory.id);

        // 削除後は存在しない
        let result = store.delete_memory(12345, memory.id);
        assert!(matches!(result, Err(MemoryError::NotFound(_))));
    }

    #[test]
    fn test_clear_memories() {
        let store = MemoryStore::new().unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Memory 1".to_string(),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Memory 2".to_string(),
                ..Default::default()
            })
            .unwrap();

        let count = store.clear_memories(12345).unwrap();
        assert_eq!(count, 2);

        let memories = store.list_memories(12345, 10).unwrap();
        assert!(memories.is_empty());
    }

    #[test]
    fn test_empty_content_rejected() {
        let store = MemoryStore::new().unwrap();

        let result = store.add_memory(NewMemory {
            user_id: 12345,
            content: "".to_string(),
            ..Default::default()
        });

        assert!(matches!(result, Err(MemoryError::InvalidInput(_))));
    }

    #[test]
    fn test_search_memory_with_special_chars() {
        let store = MemoryStore::new().unwrap();

        // 特殊文字を含むメモリを追加
        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "100% complete".to_string(),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "test_value".to_string(),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "path\\to\\file".to_string(),
                ..Default::default()
            })
            .unwrap();

        // %を含む検索 - エスケープされているので100%にマッチ
        let results = store.search_memories(12345, "100%").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("100%"));

        // _を含む検索 - エスケープされているのでtest_valueにマッチ
        let results = store.search_memories(12345, "test_value").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("test_value"));

        // バックスラッシュを含む検索
        let results = store.search_memories(12345, "path\\to").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("path\\to"));
    }

    #[test]
    fn test_memory_with_category_and_tags() {
        let store = MemoryStore::new().unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "test".to_string());

        let memory = store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Tagged memory".to_string(),
                category: Some("work".to_string()),
                tags: Some(vec!["important".to_string(), "project".to_string()]),
                metadata: Some(metadata),
            })
            .unwrap();

        assert_eq!(memory.category, "work");
        assert_eq!(memory.tags, vec!["important", "project"]);
        assert_eq!(memory.metadata.get("source"), Some(&"test".to_string()));
    }

    #[test]
    fn test_migration_from_old_schema() {
        // 古いスキーマ（category, tags, metadataなし）でDBを作成
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        // 古いスキーマでデータを挿入
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO memories (user_id, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![12345i64, "Old memory", now, now],
        ).unwrap();

        // MemoryStoreとして読み込み（マイグレーションが実行される）
        let store = MemoryStore {
            conn: Mutex::new(conn),
        };
        store.initialize().unwrap();

        // マイグレーション後、データが正しく読み込めることを確認
        let memories = store.list_memories(12345, 10).unwrap();
        assert_eq!(memories.len(), 1, "Should have 1 memory after migration");
        assert_eq!(memories[0].content, "Old memory", "Content should be preserved");
        assert_eq!(memories[0].category, "general", "Category should default to 'general'"); // デフォルト値
        assert!(memories[0].tags.is_empty(), "Tags should default to empty"); // デフォルト値
        assert!(memories[0].metadata.is_empty(), "Metadata should default to empty"); // デフォルト値
    }

    #[test]
    fn test_list_memories_by_category() {
        let store = MemoryStore::new().unwrap();

        // 異なるカテゴリのメモリを追加
        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Work memory 1".to_string(),
                category: Some("work".to_string()),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Work memory 2".to_string(),
                category: Some("work".to_string()),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Personal memory".to_string(),
                category: Some("personal".to_string()),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Default category memory".to_string(),
                ..Default::default()
            })
            .unwrap();

        // workカテゴリでフィルタリング
        let work_memories = store.list_memories_by_category(12345, "work", 10).unwrap();
        assert_eq!(work_memories.len(), 2, "Should have 2 work memories");
        assert!(work_memories.iter().all(|m| m.category == "work"), "All memories should be in 'work' category");

        // personalカテゴリでフィルタリング
        let personal_memories = store.list_memories_by_category(12345, "personal", 10).unwrap();
        assert_eq!(personal_memories.len(), 1, "Should have 1 personal memory");
        assert_eq!(personal_memories[0].content, "Personal memory", "Content should match");

        // generalカテゴリ（デフォルト）でフィルタリング
        let general_memories = store.list_memories_by_category(12345, "general", 10).unwrap();
        assert_eq!(general_memories.len(), 1, "Should have 1 general memory");
        assert_eq!(general_memories[0].content, "Default category memory", "Content should match");

        // 存在しないカテゴリは空
        let unknown_memories = store.list_memories_by_category(12345, "unknown", 10).unwrap();
        assert!(unknown_memories.is_empty(), "Unknown category should return empty");

        // 別のユーザーのメモリは取得できない
        let other_user_memories = store.list_memories_by_category(99999, "work", 10).unwrap();
        assert!(other_user_memories.is_empty(), "Other user's memories should not be returned");
    }

    #[test]
    fn test_list_memories_with_offset() {
        let store = MemoryStore::new().unwrap();

        // 5件のメモリを追加
        for i in 1..=5 {
            store
                .add_memory(NewMemory {
                    user_id: 12345,
                    content: format!("Memory {}", i),
                    ..Default::default()
                })
                .unwrap();
        }

        // ページネーションテスト
        // 1ページ目: 最新3件
        let page1 = store.list_memories_with_offset(12345, 3, 0).unwrap();
        assert_eq!(page1.len(), 3, "First page should have 3 memories");
        assert_eq!(page1[0].content, "Memory 5", "First item should be most recent");
        assert_eq!(page1[2].content, "Memory 3", "Last item of page 1");

        // 2ページ目: 残り2件
        let page2 = store.list_memories_with_offset(12345, 3, 3).unwrap();
        assert_eq!(page2.len(), 2, "Second page should have 2 memories");
        assert_eq!(page2[0].content, "Memory 2", "First item of page 2");
        assert_eq!(page2[1].content, "Memory 1", "Last item");

        // オフセットが総数を超える場合は空
        let page3 = store.list_memories_with_offset(12345, 3, 6).unwrap();
        assert!(page3.is_empty(), "Offset beyond total should return empty");
    }

    #[test]
    fn test_export_to_markdown() {
        let store = MemoryStore::new().unwrap();

        // テストデータを追加
        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Test memory 1".to_string(),
                category: Some("work".to_string()),
                tags: Some(vec!["important".to_string()]),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Test memory 2".to_string(),
                ..Default::default()
            })
            .unwrap();

        // Markdownエクスポート
        let result = store.export_to_markdown(12345).unwrap();

        // 結果を検証
        assert!(result.contains("# Memory Export"));
        assert!(result.contains("**User ID:** 12345"));
        assert!(result.contains("**Total Memories:** 2"));
        assert!(result.contains("Test memory 1"));
        assert!(result.contains("Test memory 2"));
        assert!(result.contains("**Category:** work"));
        assert!(result.contains("**Tags:** important"));
    }

    #[test]
    fn test_export_to_markdown_empty() {
        let store = MemoryStore::new().unwrap();

        // メモリがない状態でエクスポート
        let result = store.export_to_markdown(99999).unwrap();

        assert!(result.contains("# Memory Export"));
        assert!(result.contains("**Total Memories:** 0"));
        assert!(result.contains("*No memories found.*"));
    }

    #[test]
    fn test_export_to_json() {
        let store = MemoryStore::new().unwrap();

        // テストデータを追加
        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "JSON test memory".to_string(),
                category: Some("test".to_string()),
                tags: Some(vec!["json".to_string(), "export".to_string()]),
                ..Default::default()
            })
            .unwrap();

        // JSONエクスポート
        let result = store.export_to_json(12345).unwrap();

        // JSONとしてパースできることを確認
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["user_id"], 12345);
        assert_eq!(parsed["total_memories"], 1);
        assert!(parsed["memories"].is_array());
        assert_eq!(parsed["memories"][0]["content"], "JSON test memory");
        assert_eq!(parsed["memories"][0]["category"], "test");
    }

    #[test]
    fn test_export_to_json_empty() {
        let store = MemoryStore::new().unwrap();

        // メモリがない状態でエクスポート
        let result = store.export_to_json(99999).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["user_id"], 99999);
        assert_eq!(parsed["total_memories"], 0);
        assert!(parsed["memories"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_get_all_memories() {
        let store = MemoryStore::new().unwrap();

        // 複数ユーザーのデータを追加
        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "User 12345 memory".to_string(),
                ..Default::default()
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 67890,
                content: "User 67890 memory".to_string(),
                ..Default::default()
            })
            .unwrap();

        // ユーザーごとに正しく取得できるか確認
        let memories_12345 = store.get_all_memories(12345).unwrap();
        assert_eq!(memories_12345.len(), 1);
        assert_eq!(memories_12345[0].content, "User 12345 memory");

        let memories_67890 = store.get_all_memories(67890).unwrap();
        assert_eq!(memories_67890.len(), 1);
        assert_eq!(memories_67890[0].content, "User 67890 memory");
    }
}
