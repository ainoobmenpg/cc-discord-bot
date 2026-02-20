//! チャンネル設定ストア（SQLite永続化）
//!
//! チャンネルごとの設定を管理します：
//! - ワーキングディレクトリ
//! - 権限設定

use crate::datetime_utils::parse_rfc3339_or_now;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use tracing::{debug, info};

/// チャンネル設定エラー
#[derive(Debug, Error)]
pub enum ChannelSettingsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid setting key: {0}")]
    InvalidKey(String),

    #[error("Invalid setting value: {0}")]
    InvalidValue(String),

    #[error("Channel not found: {0}")]
    NotFound(u64),
}

/// チャンネル設定の単一エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSetting {
    pub channel_id: u64,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// チャンネルの全設定をまとめた構造体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelSettings {
    /// チャンネルID
    pub channel_id: u64,
    /// 出力先ワーキングディレクトリ
    pub output_dir: Option<String>,
    /// 許可ロール（カンマ区切り）
    pub allowed_roles: Option<String>,
    /// 最大履歴数
    pub max_history: Option<String>,
}

impl ChannelSettings {
    /// 新しいChannelSettingsを作成
    pub fn new(channel_id: u64) -> Self {
        Self {
            channel_id,
            ..Default::default()
        }
    }

    /// ChannelSettingのリストからChannelSettingsを作成
    pub fn from_settings(channel_id: u64, settings: &[ChannelSetting]) -> Self {
        let mut result = Self::new(channel_id);

        for setting in settings {
            match setting.key.as_str() {
                setting_keys::OUTPUT_DIR => result.output_dir = Some(setting.value.clone()),
                setting_keys::ALLOWED_ROLES => result.allowed_roles = Some(setting.value.clone()),
                setting_keys::MAX_HISTORY => result.max_history = Some(setting.value.clone()),
                _ => {} // 不明なキーは無視
            }
        }

        result
    }

    /// ChannelSettingsをChannelSettingのリストに変換
    pub fn to_settings(&self) -> Vec<ChannelSetting> {
        let now = Utc::now();
        let mut settings = Vec::new();

        if let Some(ref value) = self.output_dir {
            settings.push(ChannelSetting {
                channel_id: self.channel_id,
                key: setting_keys::OUTPUT_DIR.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        if let Some(ref value) = self.allowed_roles {
            settings.push(ChannelSetting {
                channel_id: self.channel_id,
                key: setting_keys::ALLOWED_ROLES.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        if let Some(ref value) = self.max_history {
            settings.push(ChannelSetting {
                channel_id: self.channel_id,
                key: setting_keys::MAX_HISTORY.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        settings
    }
}

/// 設定キー定数
pub mod setting_keys {
    /// 出力先ディレクトリ
    pub const OUTPUT_DIR: &str = "output_dir";
    /// 許可ロール
    pub const ALLOWED_ROLES: &str = "allowed_roles";
    /// 最大履歴数
    pub const MAX_HISTORY: &str = "max_history";
    /// チャンネル設定可能なすべてのキー
    pub const VALID_KEYS: &[&str] = &[
        OUTPUT_DIR,
        ALLOWED_ROLES,
        MAX_HISTORY,
    ];
}

/// チャンネル設定ストア（SQLite永続化）
pub struct ChannelSettingsStore {
    conn: Mutex<Connection>,
}

impl ChannelSettingsStore {
    /// Mutexロックを取得するヘルパー
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ChannelSettingsError> {
        self.conn.lock().map_err(|e| {
            ChannelSettingsError::DatabaseError(format!("Failed to lock connection: {}", e))
        })
    }

    /// 新しいChannelSettingsStoreを作成（インメモリ）
    pub fn new() -> Result<Self, ChannelSettingsError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to create in-memory DB: {}", e)))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.initialize()?;
        Ok(store)
    }

    /// ファイルパスから読み込み
    pub fn load(base_dir: &str) -> Result<Self, ChannelSettingsError> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading channel settings store from {:?}", path);

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to create directory: {}", e)))?;
        }

        let is_new = !path.exists();
        let conn = Connection::open(&path)
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to open database: {}", e)))?;

        // 新規作成時はパーミッションを設定（所有者のみ読み書き可能）
        if is_new {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to set file permissions: {}", e)))?;
                debug!("Set database file permissions to 0600");
            }
        }

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.initialize()?;
        info!("Channel settings store loaded successfully");
        Ok(store)
    }

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join("channel_settings.db")
    }

    /// データベースを初期化
    fn initialize(&self) -> Result<(), ChannelSettingsError> {
        let conn = self.lock_conn()?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS channel_settings (
                channel_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (channel_id, key)
            )",
            [],
        ).map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to create table: {}", e)))?;

        // チャンネルID用インデックス
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_channel_settings_channel_id ON channel_settings(channel_id)",
            [],
        ).map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to create index: {}", e)))?;

        debug!("Channel settings store initialized");
        Ok(())
    }

    /// 設定キーが有効か確認
    pub fn is_valid_key(key: &str) -> bool {
        setting_keys::VALID_KEYS.contains(&key)
    }

    /// 設定を保存（upsert）
    pub fn set_setting(&self, channel_id: u64, key: &str, value: &str) -> Result<ChannelSetting, ChannelSettingsError> {
        // キー検証
        if !Self::is_valid_key(key) {
            return Err(ChannelSettingsError::InvalidKey(format!(
                "Invalid setting key: {}. Valid keys: {:?}",
                key, setting_keys::VALID_KEYS
            )));
        }

        // 値検証
        let trimmed_value = value.trim();
        if trimmed_value.is_empty() {
            return Err(ChannelSettingsError::InvalidValue("Value cannot be empty".to_string()));
        }

        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let updated_at = now.to_rfc3339();

        let conn = self.lock_conn()?;

        // 既存設定の確認
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM channel_settings WHERE channel_id = ?1 AND key = ?2",
                params![channel_id as i64, key],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to check existing setting: {}", e)))?
            .unwrap_or(false);

        if exists {
            // 更新
            conn.execute(
                "UPDATE channel_settings SET value = ?1, updated_at = ?2 WHERE channel_id = ?3 AND key = ?4",
                params![trimmed_value, updated_at, channel_id as i64, key],
            ).map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to update setting: {}", e)))?;
        } else {
            // 挿入
            conn.execute(
                "INSERT INTO channel_settings (channel_id, key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![channel_id as i64, key, trimmed_value, created_at, updated_at],
            ).map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to insert setting: {}", e)))?;
        }

        Ok(ChannelSetting {
            channel_id,
            key: key.to_string(),
            value: trimmed_value.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// 設定を取得
    pub fn get_setting(&self, channel_id: u64, key: &str) -> Result<Option<ChannelSetting>, ChannelSettingsError> {
        let conn = self.lock_conn()?;

        let result = conn
            .query_row(
                "SELECT channel_id, key, value, created_at, updated_at FROM channel_settings WHERE channel_id = ?1 AND key = ?2",
                params![channel_id as i64, key],
                |row| {
                    Ok(ChannelSetting {
                        channel_id: row.get::<_, i64>(0)? as u64,
                        key: row.get(1)?,
                        value: row.get(2)?,
                        created_at: parse_rfc3339_or_now(&row.get::<_, String>(3)?),
                        updated_at: parse_rfc3339_or_now(&row.get::<_, String>(4)?),
                    })
                },
            )
            .optional()
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to query setting: {}", e)))?;

        Ok(result)
    }

    /// チャンネルの全設定を取得
    pub fn get_all_settings(&self, channel_id: u64) -> Result<Vec<ChannelSetting>, ChannelSettingsError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT channel_id, key, value, created_at, updated_at FROM channel_settings WHERE channel_id = ?1 ORDER BY key",
            )
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let settings = stmt
            .query_map(params![channel_id as i64], |row| {
                Ok(ChannelSetting {
                    channel_id: row.get::<_, i64>(0)? as u64,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    created_at: parse_rfc3339_or_now(&row.get::<_, String>(3)?),
                    updated_at: parse_rfc3339_or_now(&row.get::<_, String>(4)?),
                })
            })
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to query settings: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to collect settings: {}", e)))?;

        Ok(settings)
    }

    /// 設定を削除
    pub fn delete_setting(&self, channel_id: u64, key: &str) -> Result<bool, ChannelSettingsError> {
        let conn = self.lock_conn()?;

        let affected = conn
            .execute(
                "DELETE FROM channel_settings WHERE channel_id = ?1 AND key = ?2",
                params![channel_id as i64, key],
            )
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to delete setting: {}", e)))?;

        Ok(affected > 0)
    }

    /// チャンネルの全設定を削除
    pub fn clear_settings(&self, channel_id: u64) -> Result<usize, ChannelSettingsError> {
        let conn = self.lock_conn()?;

        let affected = conn
            .execute("DELETE FROM channel_settings WHERE channel_id = ?1", params![channel_id as i64])
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to clear settings: {}", e)))?;

        Ok(affected)
    }

    /// チャンネルの設定数を取得
    pub fn count_settings(&self, channel_id: u64) -> Result<usize, ChannelSettingsError> {
        let conn = self.lock_conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM channel_settings WHERE channel_id = ?1",
                params![channel_id as i64],
                |row| row.get(0),
            )
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to count settings: {}", e)))?;

        Ok(count as usize)
    }

    /// 設定値を取得（デフォルト値付き）
    pub fn get_setting_with_default(&self, channel_id: u64, key: &str, default: &str) -> Result<String, ChannelSettingsError> {
        match self.get_setting(channel_id, key)? {
            Some(setting) => Ok(setting.value),
            None => Ok(default.to_string()),
        }
    }

    /// 全チャンネルIDを取得（管理者用）
    pub fn list_all_channels(&self) -> Result<Vec<u64>, ChannelSettingsError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare("SELECT DISTINCT channel_id FROM channel_settings ORDER BY channel_id")
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let channel_ids = stmt
            .query_map([], |row| {
                Ok(row.get::<_, i64>(0)? as u64)
            })
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to query channels: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ChannelSettingsError::DatabaseError(format!("Failed to collect channels: {}", e)))?;

        Ok(channel_ids)
    }

    /// チャンネルの全設定をChannelSettings構造体として取得
    pub fn get_channel_settings(&self, channel_id: u64) -> Result<ChannelSettings, ChannelSettingsError> {
        let settings = self.get_all_settings(channel_id)?;
        Ok(ChannelSettings::from_settings(channel_id, &settings))
    }
}

impl Default for ChannelSettingsStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default ChannelSettingsStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_keys() {
        assert!(ChannelSettingsStore::is_valid_key(setting_keys::OUTPUT_DIR));
        assert!(ChannelSettingsStore::is_valid_key(setting_keys::ALLOWED_ROLES));
        assert!(ChannelSettingsStore::is_valid_key(setting_keys::MAX_HISTORY));
        assert!(!ChannelSettingsStore::is_valid_key("invalid_key"));
    }

    #[test]
    fn test_set_and_get_setting() {
        let store = ChannelSettingsStore::new().unwrap();

        let setting = store
            .set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output")
            .unwrap();

        assert_eq!(setting.channel_id, 12345);
        assert_eq!(setting.key, setting_keys::OUTPUT_DIR);
        assert_eq!(setting.value, "/tmp/output");

        let loaded = store.get_setting(12345, setting_keys::OUTPUT_DIR).unwrap().unwrap();
        assert_eq!(loaded.value, "/tmp/output");
    }

    #[test]
    fn test_update_setting() {
        let store = ChannelSettingsStore::new().unwrap();

        // 初期設定
        store.set_setting(12345, setting_keys::OUTPUT_DIR, "old_dir").unwrap();

        // 更新
        let updated = store.set_setting(12345, setting_keys::OUTPUT_DIR, "new_dir").unwrap();
        assert_eq!(updated.value, "new_dir");

        // 確認
        let loaded = store.get_setting(12345, setting_keys::OUTPUT_DIR).unwrap().unwrap();
        assert_eq!(loaded.value, "new_dir");
    }

    #[test]
    fn test_get_all_settings() {
        let store = ChannelSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output").unwrap();
        store.set_setting(12345, setting_keys::ALLOWED_ROLES, "admin,mod").unwrap();
        store.set_setting(12345, setting_keys::MAX_HISTORY, "100").unwrap();

        let settings = store.get_all_settings(12345).unwrap();
        assert_eq!(settings.len(), 3);
    }

    #[test]
    fn test_delete_setting() {
        let store = ChannelSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output").unwrap();

        let deleted = store.delete_setting(12345, setting_keys::OUTPUT_DIR).unwrap();
        assert!(deleted);

        let loaded = store.get_setting(12345, setting_keys::OUTPUT_DIR).unwrap();
        assert!(loaded.is_none());

        // 再度削除はfalse
        let deleted_again = store.delete_setting(12345, setting_keys::OUTPUT_DIR).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_clear_settings() {
        let store = ChannelSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output").unwrap();
        store.set_setting(12345, setting_keys::ALLOWED_ROLES, "admin").unwrap();

        let count = store.clear_settings(12345).unwrap();
        assert_eq!(count, 2);

        let settings = store.get_all_settings(12345).unwrap();
        assert!(settings.is_empty());
    }

    #[test]
    fn test_get_setting_with_default() {
        let store = ChannelSettingsStore::new().unwrap();

        // 存在しない場合はデフォルト値
        let value = store.get_setting_with_default(12345, setting_keys::OUTPUT_DIR, "default_dir").unwrap();
        assert_eq!(value, "default_dir");

        // 存在する場合は設定値
        store.set_setting(12345, setting_keys::OUTPUT_DIR, "custom_dir").unwrap();
        let value = store.get_setting_with_default(12345, setting_keys::OUTPUT_DIR, "default_dir").unwrap();
        assert_eq!(value, "custom_dir");
    }

    #[test]
    fn test_invalid_key_rejected() {
        let store = ChannelSettingsStore::new().unwrap();

        let result = store.set_setting(12345, "invalid_key", "value");
        assert!(matches!(result, Err(ChannelSettingsError::InvalidKey(_))));
    }

    #[test]
    fn test_empty_value_rejected() {
        let store = ChannelSettingsStore::new().unwrap();

        let result = store.set_setting(12345, setting_keys::OUTPUT_DIR, "");
        assert!(matches!(result, Err(ChannelSettingsError::InvalidValue(_))));

        let result = store.set_setting(12345, setting_keys::OUTPUT_DIR, "   ");
        assert!(matches!(result, Err(ChannelSettingsError::InvalidValue(_))));
    }

    #[test]
    fn test_list_all_channels() {
        let store = ChannelSettingsStore::new().unwrap();

        store.set_setting(111, setting_keys::OUTPUT_DIR, "dir1").unwrap();
        store.set_setting(222, setting_keys::OUTPUT_DIR, "dir2").unwrap();
        store.set_setting(333, setting_keys::OUTPUT_DIR, "dir3").unwrap();

        let channels = store.list_all_channels().unwrap();
        assert_eq!(channels.len(), 3);
        assert!(channels.contains(&111));
        assert!(channels.contains(&222));
        assert!(channels.contains(&333));
    }

    #[test]
    fn test_count_settings() {
        let store = ChannelSettingsStore::new().unwrap();

        assert_eq!(store.count_settings(12345).unwrap(), 0);

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp").unwrap();
        store.set_setting(12345, setting_keys::ALLOWED_ROLES, "admin").unwrap();

        assert_eq!(store.count_settings(12345).unwrap(), 2);
    }

    #[test]
    fn test_channel_settings_struct_new() {
        let settings = ChannelSettings::new(12345);
        assert_eq!(settings.channel_id, 12345);
        assert!(settings.output_dir.is_none());
        assert!(settings.allowed_roles.is_none());
    }

    #[test]
    fn test_channel_settings_from_settings() {
        let now = Utc::now();
        let settings = vec![
            ChannelSetting {
                channel_id: 12345,
                key: setting_keys::OUTPUT_DIR.to_string(),
                value: "my_output".to_string(),
                created_at: now,
                updated_at: now,
            },
            ChannelSetting {
                channel_id: 12345,
                key: setting_keys::ALLOWED_ROLES.to_string(),
                value: "admin,mod".to_string(),
                created_at: now,
                updated_at: now,
            },
        ];

        let channel_settings = ChannelSettings::from_settings(12345, &settings);
        assert_eq!(channel_settings.channel_id, 12345);
        assert_eq!(channel_settings.output_dir, Some("my_output".to_string()));
        assert_eq!(channel_settings.allowed_roles, Some("admin,mod".to_string()));
        assert!(channel_settings.max_history.is_none());
    }

    #[test]
    fn test_channel_settings_to_settings() {
        let mut channel_settings = ChannelSettings::new(12345);
        channel_settings.output_dir = Some("output_dir".to_string());
        channel_settings.allowed_roles = Some("admin".to_string());

        let settings = channel_settings.to_settings();
        assert_eq!(settings.len(), 2);

        let output_setting = settings.iter().find(|s| s.key == setting_keys::OUTPUT_DIR);
        assert!(output_setting.is_some());
        assert_eq!(output_setting.unwrap().value, "output_dir");

        let roles_setting = settings.iter().find(|s| s.key == setting_keys::ALLOWED_ROLES);
        assert!(roles_setting.is_some());
        assert_eq!(roles_setting.unwrap().value, "admin");
    }

    #[test]
    fn test_get_channel_settings() {
        let store = ChannelSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "my_output").unwrap();
        store.set_setting(12345, setting_keys::ALLOWED_ROLES, "admin,mod").unwrap();

        let channel_settings = store.get_channel_settings(12345).unwrap();
        assert_eq!(channel_settings.channel_id, 12345);
        assert_eq!(channel_settings.output_dir, Some("my_output".to_string()));
        assert_eq!(channel_settings.allowed_roles, Some("admin,mod".to_string()));
    }

    #[test]
    fn test_channel_settings_serialization() {
        let mut channel_settings = ChannelSettings::new(12345);
        channel_settings.output_dir = Some("test_dir".to_string());
        channel_settings.allowed_roles = Some("admin".to_string());

        // Serialize
        let json = serde_json::to_string(&channel_settings).unwrap();
        assert!(json.contains("test_dir"));
        assert!(json.contains("admin"));

        // Deserialize
        let deserialized: ChannelSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.channel_id, 12345);
        assert_eq!(deserialized.output_dir, Some("test_dir".to_string()));
        assert_eq!(deserialized.allowed_roles, Some("admin".to_string()));
    }

    #[test]
    fn test_different_channels_independent() {
        let store = ChannelSettingsStore::new().unwrap();

        // 異なるチャンネルに異なる設定
        store.set_setting(111, setting_keys::OUTPUT_DIR, "channel_111_dir").unwrap();
        store.set_setting(222, setting_keys::OUTPUT_DIR, "channel_222_dir").unwrap();

        // それぞれのチャンネルで独立した設定を取得
        let settings_111 = store.get_setting(111, setting_keys::OUTPUT_DIR).unwrap().unwrap();
        let settings_222 = store.get_setting(222, setting_keys::OUTPUT_DIR).unwrap().unwrap();

        assert_eq!(settings_111.value, "channel_111_dir");
        assert_eq!(settings_222.value, "channel_222_dir");

        // チャンネル111の設定を削除しても222には影響しない
        store.delete_setting(111, setting_keys::OUTPUT_DIR).unwrap();
        let settings_222_after = store.get_setting(222, setting_keys::OUTPUT_DIR).unwrap().unwrap();
        assert_eq!(settings_222_after.value, "channel_222_dir");
    }
}
