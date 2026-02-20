//! SQLite永続化ストアの共通トレイトとヘルパー関数

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use tracing::{debug, error};

/// 永続化ストアの共通エラー
#[derive(Debug, Error)]
pub enum PersistentStoreError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Lock error: {0}")]
    LockError(String),
}

/// 永続化ストアの共通操作を定義するトレイト
pub trait PersistentStore: Sized {
    /// ストア固有のエラー型
    type Error: std::fmt::Debug + std::fmt::Display + From<PersistentStoreError>;

    /// データベースファイル名を返す
    fn db_filename() -> &'static str;

    /// 新しいストアを作成（インメモリDB）
    fn new() -> Result<Self, Self::Error> {
        let conn = Connection::open_in_memory().map_err(|e| {
            error!("Failed to create in-memory DB: {}", e);
            Self::Error::from(PersistentStoreError::DatabaseError(
                "Failed to create database".to_string(),
            ))
        })?;

        let store = Self::from_connection(conn)?;
        Self::initialize(&store)?;
        Ok(store)
    }

    /// ファイルからストアを読み込む
    fn load(base_dir: &str) -> Result<Self, Self::Error> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading store from {:?}", path);

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                error!("Failed to create directory: {}", e);
                Self::Error::from(PersistentStoreError::DatabaseError(
                    "Failed to initialize storage".to_string(),
                ))
            })?;
        }

        let is_new = !path.exists();
        let conn = Connection::open(&path).map_err(|e| {
            error!("Failed to open database at {:?}: {}", path, e);
            Self::Error::from(PersistentStoreError::DatabaseError(
                "Failed to open database".to_string(),
            ))
        })?;

        // 新規作成時はパーミッションを設定（所有者のみ読み書き可能）
        if is_new {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| {
                        Self::Error::from(PersistentStoreError::DatabaseError(format!(
                            "Failed to set file permissions: {}",
                            e
                        )))
                    })?;
                debug!("Set database file permissions to 0600");
            }
        }

        let store = Self::from_connection(conn)?;
        Self::initialize(&store)?;
        debug!("Store loaded successfully");
        Ok(store)
    }

    /// Connectionからストアを作成
    fn from_connection(conn: Connection) -> Result<Self, Self::Error>;

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join(Self::db_filename())
    }

    /// データベースを初期化（テーブル作成など）
    fn initialize(&self) -> Result<(), Self::Error>;

    /// Mutexロックを取得するヘルパー
    fn lock_conn(conn: &Mutex<Connection>) -> Result<std::sync::MutexGuard<'_, Connection>, PersistentStoreError> {
        conn.lock().map_err(|e| {
            error!("Failed to lock connection: {}", e);
            PersistentStoreError::LockError(format!("Failed to lock connection: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    struct TestStore {
        conn: Mutex<Connection>,
    }

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("{0}")]
        Persistent(#[from] PersistentStoreError),
        #[error("Test error: {0}")]
        Test(String),
    }

    impl PersistentStore for TestStore {
        type Error = TestError;

        fn db_filename() -> &'static str {
            "test.db"
        }

        fn from_connection(conn: Connection) -> Result<Self, TestError> {
            Ok(Self {
                conn: Mutex::new(conn),
            })
        }

        fn initialize(&self) -> Result<(), TestError> {
            let conn = Self::lock_conn(&self.conn)?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, value TEXT)",
                [],
            )
            .map_err(|e| TestError::Test(format!("Failed to create table: {}", e)))?;
            Ok(())
        }
    }

    #[test]
    fn test_new_creates_in_memory_db() {
        let store = TestStore::new().unwrap();
        let conn = TestStore::lock_conn(&store.conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_file_path() {
        let path = TestStore::get_file_path("/data");
        assert_eq!(path.to_str().unwrap(), "/data/test.db");
    }

    #[test]
    fn test_lock_conn_success() {
        let store = TestStore::new().unwrap();
        let result = TestStore::lock_conn(&store.conn);
        assert!(result.is_ok());
    }
}
