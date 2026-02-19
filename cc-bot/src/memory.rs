//! Memory persistence module using SQLite.
//!
//! Provides CRUD operations for storing user memories.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use thiserror::Error;
use tracing::{debug, error, info};

/// Errors that can occur during memory operations.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Memory with specified ID was not found.
    #[error("Memory not found: {0}")]
    NotFound(i64),
}

/// A stored memory entry.
#[derive(Debug, Clone)]
pub struct Memory {
    /// Unique identifier for this memory.
    pub id: i64,
    /// Discord user ID who owns this memory.
    pub user_id: u64,
    /// Key/tag for categorizing this memory.
    pub key: String,
    /// The actual memory content.
    pub value: String,
    /// Timestamp when this memory was created.
    pub created_at: DateTime<Utc>,
}

/// SQLite-based memory storage.
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    /// Creates a new MemoryStore with the given database path.
    ///
    /// Creates the database file and tables if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the SQLite database file.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if database initialization fails.
    pub fn new(db_path: &str) -> Result<Self, MemoryError> {
        debug!("Initializing MemoryStore at: {}", db_path);

        let conn = Connection::open(db_path)?;

        // Create tables
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memories_user_id ON memories(user_id);
            CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);
            "#,
        )?;

        info!("MemoryStore initialized successfully");
        Ok(Self { conn })
    }

    /// Creates an in-memory MemoryStore for testing.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if database initialization fails.
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, MemoryError> {
        debug!("Initializing in-memory MemoryStore");

        let conn = Connection::open_in_memory()?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memories_user_id ON memories(user_id);
            CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);
            "#,
        )?;

        info!("In-memory MemoryStore initialized successfully");
        Ok(Self { conn })
    }

    /// Saves a new memory for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Discord user ID.
    /// * `key` - Key/tag for categorizing the memory.
    /// * `value` - The memory content.
    ///
    /// # Returns
    ///
    /// The ID of the newly created memory.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if the insert operation fails.
    pub fn save_memory(&self, user_id: u64, key: &str, value: &str) -> Result<i64, MemoryError> {
        let now = Utc::now().to_rfc3339();

        debug!("Saving memory for user {}: key={}, value={}", user_id, key, value);

        self.conn.execute(
            "INSERT INTO memories (user_id, key, value, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![user_id as i64, key, value, now],
        )?;

        let id = self.conn.last_insert_rowid();
        info!("Saved memory {} for user {}", id, user_id);
        Ok(id)
    }

    /// Retrieves a memory by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The memory ID.
    ///
    /// # Returns
    ///
    /// `Some(Memory)` if found, `None` if not found.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if the query fails.
    pub fn get_memory(&self, id: i64) -> Result<Option<Memory>, MemoryError> {
        debug!("Getting memory {}", id);

        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, key, value, created_at FROM memories WHERE id = ?1"
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Memory {
                id: row.get(0)?,
                user_id: row.get::<_, i64>(1)? as u64,
                key: row.get(2)?,
                value: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(memory) => {
                debug!("Found memory {}", id);
                Ok(Some(memory))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!("Memory {} not found", id);
                Ok(None)
            }
            Err(e) => {
                error!("Failed to get memory {}: {}", id, e);
                Err(MemoryError::from(e))
            }
        }
    }

    /// Lists all memories for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Discord user ID.
    ///
    /// # Returns
    ///
    /// A vector of memories belonging to the user.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if the query fails.
    pub fn list_memories(&self, user_id: u64) -> Result<Vec<Memory>, MemoryError> {
        debug!("Listing memories for user {}", user_id);

        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, key, value, created_at FROM memories WHERE user_id = ?1 ORDER BY created_at DESC"
        )?;

        let memories = stmt.query_map(params![user_id as i64], |row| {
            Ok(Memory {
                id: row.get(0)?,
                user_id: row.get::<_, i64>(1)? as u64,
                key: row.get(2)?,
                value: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        debug!("Found {} memories for user {}", memories.len(), user_id);
        Ok(memories)
    }

    /// Searches memories for a user by query string.
    ///
    /// Searches in both the key and value fields using LIKE pattern matching.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Discord user ID.
    /// * `query` - Search query string.
    ///
    /// # Returns
    ///
    /// A vector of memories matching the query.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if the query fails.
    pub fn search_memories(&self, user_id: u64, query: &str) -> Result<Vec<Memory>, MemoryError> {
        debug!("Searching memories for user {} with query: {}", user_id, query);

        let pattern = format!("%{}%", query);

        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, key, value, created_at FROM memories WHERE user_id = ?1 AND (key LIKE ?2 OR value LIKE ?2) ORDER BY created_at DESC"
        )?;

        let memories = stmt.query_map(params![user_id as i64, pattern], |row| {
            Ok(Memory {
                id: row.get(0)?,
                user_id: row.get::<_, i64>(1)? as u64,
                key: row.get(2)?,
                value: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        debug!("Found {} matching memories for user {}", memories.len(), user_id);
        Ok(memories)
    }

    /// Deletes a memory by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The memory ID to delete.
    ///
    /// # Returns
    ///
    /// `true` if a memory was deleted, `false` if no memory was found.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if the delete operation fails.
    pub fn delete_memory(&self, id: i64) -> Result<bool, MemoryError> {
        debug!("Deleting memory {}", id);

        let rows_affected = self.conn.execute(
            "DELETE FROM memories WHERE id = ?1",
            params![id],
        )?;

        if rows_affected > 0 {
            info!("Deleted memory {}", id);
            Ok(true)
        } else {
            debug!("Memory {} not found for deletion", id);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> MemoryStore {
        MemoryStore::new_in_memory().expect("Failed to create test store")
    }

    #[test]
    fn test_new_creates_tables() {
        let store = create_test_store();
        // If we get here, tables were created successfully
        drop(store);
    }

    #[test]
    fn test_save_memory_returns_id() {
        let store = create_test_store();

        let id = store.save_memory(12345, "test_key", "test_value")
            .expect("Failed to save memory");

        assert!(id > 0);
    }

    #[test]
    fn test_get_memory_returns_saved_memory() {
        let store = create_test_store();

        let id = store.save_memory(12345, "test_key", "test_value")
            .expect("Failed to save memory");

        let memory = store.get_memory(id)
            .expect("Failed to get memory")
            .expect("Memory not found");

        assert_eq!(memory.id, id);
        assert_eq!(memory.user_id, 12345);
        assert_eq!(memory.key, "test_key");
        assert_eq!(memory.value, "test_value");
    }

    #[test]
    fn test_get_memory_returns_none_for_nonexistent() {
        let store = create_test_store();

        let result = store.get_memory(99999)
            .expect("Query failed");

        assert!(result.is_none());
    }

    #[test]
    fn test_list_memories_returns_user_memories() {
        let store = create_test_store();

        store.save_memory(12345, "key1", "value1").expect("Failed to save");
        store.save_memory(12345, "key2", "value2").expect("Failed to save");
        store.save_memory(67890, "key3", "value3").expect("Failed to save");

        let memories = store.list_memories(12345)
            .expect("Failed to list memories");

        assert_eq!(memories.len(), 2);
        // Should be ordered by created_at DESC, so key2 comes first
        assert_eq!(memories[0].key, "key2");
        assert_eq!(memories[1].key, "key1");
    }

    #[test]
    fn test_list_memories_returns_empty_for_unknown_user() {
        let store = create_test_store();

        let memories = store.list_memories(99999)
            .expect("Failed to list memories");

        assert!(memories.is_empty());
    }

    #[test]
    fn test_search_memories_finds_matching_key() {
        let store = create_test_store();

        store.save_memory(12345, "shopping", "buy groceries").expect("Failed to save");
        store.save_memory(12345, "work", "meeting at 3pm").expect("Failed to save");
        store.save_memory(12345, "personal", "doctor appointment").expect("Failed to save");

        let results = store.search_memories(12345, "shop")
            .expect("Failed to search memories");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "shopping");
    }

    #[test]
    fn test_search_memories_finds_matching_value() {
        let store = create_test_store();

        store.save_memory(12345, "reminder1", "buy milk and eggs").expect("Failed to save");
        store.save_memory(12345, "reminder2", "pick up dry cleaning").expect("Failed to save");

        let results = store.search_memories(12345, "milk")
            .expect("Failed to search memories");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "reminder1");
    }

    #[test]
    fn test_search_memories_is_case_insensitive() {
        let store = create_test_store();

        store.save_memory(12345, "Important", "URGENT task").expect("Failed to save");

        // SQLite LIKE is case-insensitive by default for ASCII
        let results = store.search_memories(12345, "urgent")
            .expect("Failed to search memories");

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_memories_only_returns_user_memories() {
        let store = create_test_store();

        store.save_memory(12345, "secret", "my secret data").expect("Failed to save");
        store.save_memory(67890, "secret", "other secret data").expect("Failed to save");

        let results = store.search_memories(12345, "secret")
            .expect("Failed to search memories");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].user_id, 12345);
    }

    #[test]
    fn test_delete_memory_removes_memory() {
        let store = create_test_store();

        let id = store.save_memory(12345, "to_delete", "value")
            .expect("Failed to save memory");

        let deleted = store.delete_memory(id)
            .expect("Failed to delete memory");

        assert!(deleted);

        let result = store.get_memory(id)
            .expect("Query failed");
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_memory_returns_false_for_nonexistent() {
        let store = create_test_store();

        let deleted = store.delete_memory(99999)
            .expect("Failed to delete memory");

        assert!(!deleted);
    }

    #[test]
    fn test_memory_created_at_is_set() {
        let store = create_test_store();

        let before = Utc::now();
        let id = store.save_memory(12345, "test", "value")
            .expect("Failed to save memory");
        let after = Utc::now();

        let memory = store.get_memory(id)
            .expect("Failed to get memory")
            .expect("Memory not found");

        // created_at should be between before and after (with some tolerance for parsing)
        assert!(memory.created_at >= before - chrono::Duration::seconds(1));
        assert!(memory.created_at <= after + chrono::Duration::seconds(1));
    }

    #[test]
    fn test_multiple_saves_have_different_ids() {
        let store = create_test_store();

        let id1 = store.save_memory(12345, "key1", "value1").expect("Failed to save");
        let id2 = store.save_memory(12345, "key2", "value2").expect("Failed to save");
        let id3 = store.save_memory(12345, "key3", "value3").expect("Failed to save");

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
}
