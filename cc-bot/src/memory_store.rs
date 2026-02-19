use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use tracing::{debug, info};

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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新規メモリ作成用
#[derive(Debug, Clone)]
pub struct NewMemory {
    pub user_id: u64,
    pub content: String,
}

/// メモリストア（SQLite永続化）
pub struct MemoryStore {
    conn: Mutex<Connection>,
}

impl MemoryStore {
    /// 新しいMemoryStoreを作成
    pub fn new() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to create in-memory DB: {}", e)))?;

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

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| MemoryError::DatabaseError(format!("Failed to create directory: {}", e)))?;
            }
        }

        let conn = Connection::open(&path)
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to open database: {}", e)))?;

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
        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create table: {}", e)))?;

        // 検索用インデックス
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_id ON memories(user_id)",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON memories(created_at)",
            [],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to create index: {}", e)))?;

        debug!("Memory store initialized");
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

        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        conn.execute(
            "INSERT INTO memories (user_id, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![new_memory.user_id as i64, new_memory.content, created_at, updated_at],
        ).map_err(|e| MemoryError::DatabaseError(format!("Failed to insert memory: {}", e)))?;

        let id = conn.last_insert_rowid();

        Ok(Memory {
            id,
            user_id: new_memory.user_id,
            content: new_memory.content,
            created_at: now,
            updated_at: now,
        })
    }

    /// ユーザーのメモリ一覧を取得（最新10件）
    pub fn list_memories(&self, user_id: u64, limit: usize) -> Result<Vec<Memory>, MemoryError> {
        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, content, created_at, updated_at
                 FROM memories
                 WHERE user_id = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let memories = stmt
            .query_map(params![user_id as i64, limit as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    user_id: row.get::<_, i64>(1)? as u64,
                    content: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memories: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to collect memories: {}", e)))?;

        Ok(memories)
    }

    /// メモリを検索
    pub fn search_memories(&self, user_id: u64, query: &str) -> Result<Vec<Memory>, MemoryError> {
        if query.trim().is_empty() {
            return self.list_memories(user_id, 10);
        }

        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        let search_pattern = format!("%{}%", query.trim());

        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, content, created_at, updated_at
                 FROM memories
                 WHERE user_id = ?1 AND content LIKE ?2
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
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to query memories: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to collect memories: {}", e)))?;

        Ok(memories)
    }

    /// メモリを削除（ユーザー確認付き）
    pub fn delete_memory(&self, user_id: u64, id: MemoryId) -> Result<Memory, MemoryError> {
        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        // まず対象を取得してユーザー確認
        let memory: Option<Memory> = conn
            .query_row(
                "SELECT id, user_id, content, created_at, updated_at FROM memories WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Memory {
                        id: row.get(0)?,
                        user_id: row.get::<_, i64>(1)? as u64,
                        content: row.get(2)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
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
    pub fn clear_memories(&self, user_id: u64) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        let affected = conn
            .execute("DELETE FROM memories WHERE user_id = ?1", params![user_id as i64])
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to clear memories: {}", e)))?;

        Ok(affected)
    }

    /// ユーザーのメモリ数を取得
    pub fn count_memories(&self, user_id: u64) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().map_err(|e| {
            MemoryError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE user_id = ?1",
                params![user_id as i64],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::DatabaseError(format!("Failed to count memories: {}", e)))?;

        Ok(count as usize)
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
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Goodbye moon".to_string(),
            })
            .unwrap();

        let results = store.search_memories(12345, "world").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("world"));
    }

    #[test]
    fn test_delete_memory() {
        let store = MemoryStore::new().unwrap();

        let memory = store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "To be deleted".to_string(),
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
            })
            .unwrap();

        store
            .add_memory(NewMemory {
                user_id: 12345,
                content: "Memory 2".to_string(),
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
        });

        assert!(matches!(result, Err(MemoryError::InvalidInput(_))));
    }
}
