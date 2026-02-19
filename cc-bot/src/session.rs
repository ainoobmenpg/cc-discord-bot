use crate::history::ChatHistory;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// セッションストアのエラー型
#[derive(Debug, Error)]
pub enum SessionStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// セッションを一意に識別するキー
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionKey {
    pub user_id: u64,
    pub channel_id: u64,
}

impl SessionKey {
    pub fn new(user_id: u64, channel_id: u64) -> Self {
        Self { user_id, channel_id }
    }
}

/// セッション
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub key: SessionKey,
    pub history: ChatHistory,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

impl Session {
    pub fn new(key: SessionKey, max_history: usize) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            key,
            history: ChatHistory::new(max_history),
            created_at: now,
            last_active: now,
        }
    }

    /// 最終活動時刻を更新
    pub fn touch(&mut self) {
        self.last_active = Utc::now();
    }

    /// タイムアウトしたかどうか
    pub fn is_expired(&self, timeout: Duration) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_active)
            .to_std()
            .unwrap_or(Duration::ZERO);
        elapsed > timeout
    }
}

/// ChatHistoryのシリアライズ用ヘルパー構造体
#[derive(Debug, Serialize, Deserialize)]
struct ChatHistoryData {
    messages: Vec<crate::history::ChatMessage>,
    max_size: usize,
}

impl From<&ChatHistory> for ChatHistoryData {
    fn from(history: &ChatHistory) -> Self {
        Self {
            messages: history.to_vec(),
            max_size: history.max_size(),
        }
    }
}

impl From<ChatHistoryData> for ChatHistory {
    fn from(data: ChatHistoryData) -> Self {
        let mut history = ChatHistory::new(data.max_size);
        for msg in data.messages {
            history.push(msg);
        }
        history
    }
}

/// セッションストア（SQLite永続化）
#[derive(Debug)]
pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// 新しいセッションストアを作成
    pub fn new(db_path: &str) -> Result<Self, SessionStoreError> {
        // データベースディレクトリを作成
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(db_path)?;

        // テーブルを作成
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                user_id INTEGER NOT NULL,
                channel_id INTEGER NOT NULL,
                history TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (user_id, channel_id)
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// セッションを保存
    pub fn save_session(&self, session: &Session) -> Result<(), SessionStoreError> {
        let history_data = ChatHistoryData::from(&session.history);
        let history_json = serde_json::to_string(&history_data)?;
        let updated_at = session.last_active.to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO sessions (user_id, channel_id, history, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                session.key.user_id as i64,
                session.key.channel_id as i64,
                history_json,
                updated_at,
            ],
        )?;

        debug!("Saved session for user {} in channel {}", session.key.user_id, session.key.channel_id);
        Ok(())
    }

    /// セッションを読み込み
    pub fn load_session(&self, key: &SessionKey) -> Result<Option<Session>, SessionStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT history, updated_at FROM sessions WHERE user_id = ?1 AND channel_id = ?2"
        )?;

        let result = stmt.query_row(
            params![key.user_id as i64, key.channel_id as i64],
            |row| {
                let history_json: String = row.get(0)?;
                let updated_at_str: String = row.get(1)?;
                Ok((history_json, updated_at_str))
            },
        );

        match result {
            Ok((history_json, updated_at_str)) => {
                let history_data: ChatHistoryData = serde_json::from_str(&history_json)?;
                let history = ChatHistory::from(history_data);
                let last_active = DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(Session {
                    id: Uuid::new_v4(), // 復元時は新しいIDを生成
                    key: key.clone(),
                    history,
                    created_at: last_active, // 正確な作成時刻は不明なので最終活動時刻を使用
                    last_active,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SessionStoreError::Database(e)),
        }
    }

    /// 全セッションを一覧取得
    pub fn list_sessions(&self) -> Result<Vec<Session>, SessionStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT user_id, channel_id, history, updated_at FROM sessions"
        )?;

        let sessions = stmt.query_map([], |row| {
            let user_id: i64 = row.get(0)?;
            let channel_id: i64 = row.get(1)?;
            let history_json: String = row.get(2)?;
            let updated_at_str: String = row.get(3)?;

            Ok((user_id, channel_id, history_json, updated_at_str))
        })?;

        let mut result = Vec::new();
        for session_result in sessions {
            let (user_id, channel_id, history_json, updated_at_str) = session_result?;

            let history_data: ChatHistoryData = match serde_json::from_str(&history_json) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to deserialize history for user {}: {}", user_id, e);
                    continue;
                }
            };
            let history = ChatHistory::from(history_data);
            let last_active = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            result.push(Session {
                id: Uuid::new_v4(),
                key: SessionKey::new(user_id as u64, channel_id as u64),
                history,
                created_at: last_active,
                last_active,
            });
        }

        Ok(result)
    }

    /// セッションを削除
    pub fn delete_session(&self, key: &SessionKey) -> Result<bool, SessionStoreError> {
        let affected = self.conn.execute(
            "DELETE FROM sessions WHERE user_id = ?1 AND channel_id = ?2",
            params![key.user_id as i64, key.channel_id as i64],
        )?;

        Ok(affected > 0)
    }

    /// 全セッション数を取得
    pub fn count(&self) -> Result<usize, SessionStoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }
}

/// セッションマネージャー
#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<SessionKey, Session>,
    max_history: usize,
    timeout: Duration,
}

impl SessionManager {
    pub fn new(max_history: usize, timeout: Duration) -> Self {
        Self {
            sessions: HashMap::new(),
            max_history,
            timeout,
        }
    }

    /// セッションを取得または作成
    pub fn get_or_create(&mut self, key: SessionKey) -> &mut Session {
        self.sessions.entry(key.clone()).or_insert_with(|| {
            info!("Creating new session for user {} in channel {}", key.user_id, key.channel_id);
            Session::new(key, self.max_history)
        })
    }

    /// セッションを取得（存在する場合のみ）
    pub fn get(&self, key: &SessionKey) -> Option<&Session> {
        self.sessions.get(key)
    }

    /// セッションを可変参照で取得（存在する場合のみ）
    pub fn get_mut(&mut self, key: &SessionKey) -> Option<&mut Session> {
        self.sessions.get_mut(key)
    }

    /// セッションをクリア
    pub fn clear(&mut self, key: &SessionKey) -> bool {
        if let Some(session) = self.sessions.get_mut(key) {
            session.history.clear();
            session.touch();
            debug!("Cleared session for user {}", key.user_id);
            true
        } else {
            false
        }
    }

    /// 期限切れセッションを削除
    pub fn cleanup_expired(&mut self) -> usize {
        let timeout = self.timeout;
        let expired_keys: Vec<SessionKey> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.is_expired(timeout))
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            self.sessions.remove(&key);
            debug!("Removed expired session for user {}", key.user_id);
        }

        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }
        count
    }

    /// アクティブセッション数
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// セッションが空かどうか
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// ストアからセッションを復元
    pub fn load_from_store(&mut self, store: &SessionStore) -> Result<usize, SessionStoreError> {
        let sessions = store.list_sessions()?;
        let count = sessions.len();

        for session in sessions {
            // 期限切れでないセッションのみ復元
            if !session.is_expired(self.timeout) {
                self.sessions.insert(session.key.clone(), session);
            }
        }

        info!("Restored {} sessions from store", self.sessions.len());
        Ok(count)
    }

    /// 全セッションをストアに保存
    pub fn save_to_store(&self, store: &SessionStore) -> Result<usize, SessionStoreError> {
        let mut count = 0;
        for session in self.sessions.values() {
            store.save_session(session)?;
            count += 1;
        }

        if count > 0 {
            info!("Saved {} sessions to store", count);
        }
        Ok(count)
    }

    /// 全セッションのイテレータを取得
    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(50, Duration::from_secs(30 * 60)) // 50件, 30分タイムアウト
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_key() {
        let key1 = SessionKey::new(123, 456);
        let key2 = SessionKey::new(123, 456);
        let key3 = SessionKey::new(123, 789);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_session_creation() {
        let key = SessionKey::new(123, 456);
        let session = Session::new(key.clone(), 10);

        assert!(!session.id.is_nil());
        assert_eq!(session.key, key);
        assert!(session.history.is_empty());
    }

    #[test]
    fn test_session_manager() {
        let mut manager = SessionManager::new(10, Duration::from_secs(60));
        let key = SessionKey::new(123, 456);

        // 新規作成
        {
            let session = manager.get_or_create(key.clone());
            let id = session.id;
            assert!(!id.is_nil());

            // 同じキーで取得すると同じセッション
            let session2 = manager.get_or_create(key.clone());
            assert_eq!(id, session2.id);
        }

        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_session_clear() {
        let mut manager = SessionManager::new(10, Duration::from_secs(60));
        let key = SessionKey::new(123, 456);

        let session = manager.get_or_create(key.clone());
        session.history.push(crate::history::ChatMessage::user("test"));
        assert_eq!(session.history.len(), 1);

        manager.clear(&key);
        let session = manager.get(&key).unwrap();
        assert!(session.history.is_empty());
    }

    #[test]
    fn test_session_expiry() {
        let key = SessionKey::new(123, 456);
        let session = Session::new(key, 10);

        // 作成直後は期限切れではない
        assert!(!session.is_expired(Duration::from_secs(60)));

        // タイムアウト0なら即座に期限切れ
        assert!(session.is_expired(Duration::ZERO));
    }

    #[test]
    fn test_session_store_crud() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();

        let store = SessionStore::new(db_path_str).unwrap();

        // セッションを作成して保存
        let key = SessionKey::new(123, 456);
        let mut session = Session::new(key.clone(), 10);
        session.history.push(crate::history::ChatMessage::user("Hello"));
        session.history.push(crate::history::ChatMessage::assistant("Hi there"));

        store.save_session(&session).unwrap();

        // 読み込み
        let loaded = store.load_session(&key).unwrap().unwrap();
        assert_eq!(loaded.key, key);
        assert_eq!(loaded.history.len(), 2);

        // 一覧
        let all = store.list_sessions().unwrap();
        assert_eq!(all.len(), 1);

        // 削除
        let deleted = store.delete_session(&key).unwrap();
        assert!(deleted);

        // 削除後は読み込めない
        let loaded = store.load_session(&key).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_session_manager_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();

        // セッションを作成
        let mut manager = SessionManager::new(10, Duration::from_secs(3600));
        let key = SessionKey::new(123, 456);
        {
            let session = manager.get_or_create(key.clone());
            session.history.push(crate::history::ChatMessage::user("Test message"));
        }

        // 保存
        let store = SessionStore::new(db_path_str).unwrap();
        manager.save_to_store(&store).unwrap();

        // 新しいマネージャーに復元
        let mut manager2 = SessionManager::new(10, Duration::from_secs(3600));
        manager2.load_from_store(&store).unwrap();

        assert_eq!(manager2.len(), 1);

        // 復元したセッションを確認
        let session = manager2.get(&key).unwrap();
        assert_eq!(session.history.len(), 1);
    }
}
