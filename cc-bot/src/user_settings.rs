use crate::datetime_utils::parse_rfc3339_or_now;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use tracing::{debug, info};

/// ユーザー設定エラー
#[derive(Debug, Error)]
pub enum UserSettingsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid setting key: {0}")]
    InvalidKey(String),

    #[error("Invalid setting value: {0}")]
    InvalidValue(String),

    #[error("Setting not found: {0}")]
    NotFound(String),
}

/// ユーザー設定の単一エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSetting {
    pub user_id: u64,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ユーザーの全設定をまとめた構造体
/// コマンドやAPIで簡単に扱うためのヘルパー構造体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSettings {
    /// ユーザーID
    pub user_id: u64,
    /// 出力先サブディレクトリ
    pub output_subdir: Option<String>,
    /// 言語設定
    pub language: Option<String>,
    /// タイムゾーン
    pub timezone: Option<String>,
    /// 通知設定
    pub notifications: Option<String>,
    /// 最大履歴数
    pub max_history: Option<String>,
}

impl UserSettings {
    /// 新しいUserSettingsを作成
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            ..Default::default()
        }
    }

    /// UserSettingのリストからUserSettingsを作成
    pub fn from_settings(user_id: u64, settings: &[UserSetting]) -> Self {
        let mut result = Self::new(user_id);

        for setting in settings {
            match setting.key.as_str() {
                setting_keys::OUTPUT_DIR => result.output_subdir = Some(setting.value.clone()),
                setting_keys::LANGUAGE => result.language = Some(setting.value.clone()),
                setting_keys::TIMEZONE => result.timezone = Some(setting.value.clone()),
                setting_keys::NOTIFICATIONS => result.notifications = Some(setting.value.clone()),
                setting_keys::MAX_HISTORY => result.max_history = Some(setting.value.clone()),
                _ => {} // 不明なキーは無視
            }
        }

        result
    }

    /// UserSettingsをUserSettingのリストに変換
    pub fn to_settings(&self) -> Vec<UserSetting> {
        let now = Utc::now();
        let mut settings = Vec::new();

        if let Some(ref value) = self.output_subdir {
            settings.push(UserSetting {
                user_id: self.user_id,
                key: setting_keys::OUTPUT_DIR.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        if let Some(ref value) = self.language {
            settings.push(UserSetting {
                user_id: self.user_id,
                key: setting_keys::LANGUAGE.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        if let Some(ref value) = self.timezone {
            settings.push(UserSetting {
                user_id: self.user_id,
                key: setting_keys::TIMEZONE.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        if let Some(ref value) = self.notifications {
            settings.push(UserSetting {
                user_id: self.user_id,
                key: setting_keys::NOTIFICATIONS.to_string(),
                value: value.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        if let Some(ref value) = self.max_history {
            settings.push(UserSetting {
                user_id: self.user_id,
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
    /// 言語設定
    pub const LANGUAGE: &str = "language";
    /// タイムゾーン
    pub const TIMEZONE: &str = "timezone";
    /// 通知設定
    pub const NOTIFICATIONS: &str = "notifications";
    /// 最大履歴数
    pub const MAX_HISTORY: &str = "max_history";
    /// ユーザー設定可能なすべてのキー
    pub const VALID_KEYS: &[&str] = &[
        OUTPUT_DIR,
        LANGUAGE,
        TIMEZONE,
        NOTIFICATIONS,
        MAX_HISTORY,
    ];
}

/// ユーザー設定ストア（SQLite永続化）
pub struct UserSettingsStore {
    conn: Mutex<Connection>,
}

impl UserSettingsStore {
    /// Mutexロックを取得するヘルパー
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, UserSettingsError> {
        self.conn.lock().map_err(|e| {
            UserSettingsError::DatabaseError(format!("Failed to lock connection: {}", e))
        })
    }

    /// 新しいUserSettingsStoreを作成（インメモリ）
    pub fn new() -> Result<Self, UserSettingsError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to create in-memory DB: {}", e)))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.initialize()?;
        Ok(store)
    }

    /// ファイルパスから読み込み
    pub fn load(base_dir: &str) -> Result<Self, UserSettingsError> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading user settings store from {:?}", path);

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to create directory: {}", e)))?;
        }

        let is_new = !path.exists();
        let conn = Connection::open(&path)
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to open database: {}", e)))?;

        // 新規作成時はパーミッションを設定（所有者のみ読み書き可能）
        if is_new {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to set file permissions: {}", e)))?;
                debug!("Set database file permissions to 0600");
            }
        }

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.initialize()?;
        info!("User settings store loaded successfully");
        Ok(store)
    }

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join("user_settings.db")
    }

    /// データベースを初期化
    fn initialize(&self) -> Result<(), UserSettingsError> {
        let conn = self.lock_conn()?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_settings (
                user_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (user_id, key)
            )",
            [],
        ).map_err(|e| UserSettingsError::DatabaseError(format!("Failed to create table: {}", e)))?;

        // ユーザーID用インデックス
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_settings_user_id ON user_settings(user_id)",
            [],
        ).map_err(|e| UserSettingsError::DatabaseError(format!("Failed to create index: {}", e)))?;

        debug!("User settings store initialized");
        Ok(())
    }

    /// 設定キーが有効か確認
    pub fn is_valid_key(key: &str) -> bool {
        setting_keys::VALID_KEYS.contains(&key)
    }

    /// 設定を保存（upsert）
    pub fn set_setting(&self, user_id: u64, key: &str, value: &str) -> Result<UserSetting, UserSettingsError> {
        // キー検証
        if !Self::is_valid_key(key) {
            return Err(UserSettingsError::InvalidKey(format!(
                "Invalid setting key: {}. Valid keys: {:?}",
                key, setting_keys::VALID_KEYS
            )));
        }

        // 値検証
        let trimmed_value = value.trim();
        if trimmed_value.is_empty() {
            return Err(UserSettingsError::InvalidValue("Value cannot be empty".to_string()));
        }

        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let updated_at = now.to_rfc3339();

        let conn = self.lock_conn()?;

        // 既存設定の確認
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM user_settings WHERE user_id = ?1 AND key = ?2",
                params![user_id as i64, key],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to check existing setting: {}", e)))?
            .unwrap_or(false);

        if exists {
            // 更新
            conn.execute(
                "UPDATE user_settings SET value = ?1, updated_at = ?2 WHERE user_id = ?3 AND key = ?4",
                params![trimmed_value, updated_at, user_id as i64, key],
            ).map_err(|e| UserSettingsError::DatabaseError(format!("Failed to update setting: {}", e)))?;
        } else {
            // 挿入
            conn.execute(
                "INSERT INTO user_settings (user_id, key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![user_id as i64, key, trimmed_value, created_at, updated_at],
            ).map_err(|e| UserSettingsError::DatabaseError(format!("Failed to insert setting: {}", e)))?;
        }

        Ok(UserSetting {
            user_id,
            key: key.to_string(),
            value: trimmed_value.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// 設定を取得
    pub fn get_setting(&self, user_id: u64, key: &str) -> Result<Option<UserSetting>, UserSettingsError> {
        let conn = self.lock_conn()?;

        let result = conn
            .query_row(
                "SELECT user_id, key, value, created_at, updated_at FROM user_settings WHERE user_id = ?1 AND key = ?2",
                params![user_id as i64, key],
                |row| {
                    Ok(UserSetting {
                        user_id: row.get::<_, i64>(0)? as u64,
                        key: row.get(1)?,
                        value: row.get(2)?,
                        created_at: parse_rfc3339_or_now(&row.get::<_, String>(3)?),
                        updated_at: parse_rfc3339_or_now(&row.get::<_, String>(4)?),
                    })
                },
            )
            .optional()
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to query setting: {}", e)))?;

        Ok(result)
    }

    /// ユーザーの全設定を取得
    pub fn get_all_settings(&self, user_id: u64) -> Result<Vec<UserSetting>, UserSettingsError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT user_id, key, value, created_at, updated_at FROM user_settings WHERE user_id = ?1 ORDER BY key",
            )
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let settings = stmt
            .query_map(params![user_id as i64], |row| {
                Ok(UserSetting {
                    user_id: row.get::<_, i64>(0)? as u64,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    created_at: parse_rfc3339_or_now(&row.get::<_, String>(3)?),
                    updated_at: parse_rfc3339_or_now(&row.get::<_, String>(4)?),
                })
            })
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to query settings: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to collect settings: {}", e)))?;

        Ok(settings)
    }

    /// 設定を削除
    pub fn delete_setting(&self, user_id: u64, key: &str) -> Result<bool, UserSettingsError> {
        let conn = self.lock_conn()?;

        let affected = conn
            .execute(
                "DELETE FROM user_settings WHERE user_id = ?1 AND key = ?2",
                params![user_id as i64, key],
            )
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to delete setting: {}", e)))?;

        Ok(affected > 0)
    }

    /// ユーザーの全設定を削除
    pub fn clear_settings(&self, user_id: u64) -> Result<usize, UserSettingsError> {
        let conn = self.lock_conn()?;

        let affected = conn
            .execute("DELETE FROM user_settings WHERE user_id = ?1", params![user_id as i64])
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to clear settings: {}", e)))?;

        Ok(affected)
    }

    /// ユーザーの設定数を取得
    pub fn count_settings(&self, user_id: u64) -> Result<usize, UserSettingsError> {
        let conn = self.lock_conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM user_settings WHERE user_id = ?1",
                params![user_id as i64],
                |row| row.get(0),
            )
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to count settings: {}", e)))?;

        Ok(count as usize)
    }

    /// 設定値を取得（デフォルト値付き）
    pub fn get_setting_with_default(&self, user_id: u64, key: &str, default: &str) -> Result<String, UserSettingsError> {
        match self.get_setting(user_id, key)? {
            Some(setting) => Ok(setting.value),
            None => Ok(default.to_string()),
        }
    }

    /// 全ユーザーIDを取得（管理者用）
    pub fn list_all_users(&self) -> Result<Vec<u64>, UserSettingsError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn
            .prepare("SELECT DISTINCT user_id FROM user_settings ORDER BY user_id")
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;

        let user_ids = stmt
            .query_map([], |row| {
                Ok(row.get::<_, i64>(0)? as u64)
            })
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to query users: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| UserSettingsError::DatabaseError(format!("Failed to collect users: {}", e)))?;

        Ok(user_ids)
    }

    /// ユーザーの全設定をUserSettings構造体として取得
    pub fn get_user_settings(&self, user_id: u64) -> Result<UserSettings, UserSettingsError> {
        let settings = self.get_all_settings(user_id)?;
        Ok(UserSettings::from_settings(user_id, &settings))
    }

    /// UserSettings構造体を保存（トランザクションで一括処理）
    pub fn save_user_settings(&self, user_settings: &UserSettings) -> Result<usize, UserSettingsError> {
        let settings = user_settings.to_settings();
        if settings.is_empty() {
            return Ok(0);
        }

        let mut conn = self.lock_conn()?;

        // トランザクション開始
        let tx = conn.transaction().map_err(|e| {
            UserSettingsError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let updated_at = now.to_rfc3339();
        let mut count = 0;

        for setting in &settings {
            // キー検証
            if !Self::is_valid_key(&setting.key) {
                return Err(UserSettingsError::InvalidKey(format!(
                    "Invalid setting key: {}. Valid keys: {:?}",
                    setting.key, setting_keys::VALID_KEYS
                )));
            }

            // 値検証
            let trimmed_value = setting.value.trim();
            if trimmed_value.is_empty() {
                return Err(UserSettingsError::InvalidValue("Value cannot be empty".to_string()));
            }

            // UPSERT（INSERT OR REPLACE）
            tx.execute(
                "INSERT OR REPLACE INTO user_settings (user_id, key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![setting.user_id as i64, &setting.key, trimmed_value, &created_at, &updated_at],
            ).map_err(|e| UserSettingsError::DatabaseError(format!("Failed to save setting: {}", e)))?;
            count += 1;
        }

        // コミット
        tx.commit().map_err(|e| {
            UserSettingsError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(count)
    }
}

impl Default for UserSettingsStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default UserSettingsStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_keys() {
        assert!(UserSettingsStore::is_valid_key(setting_keys::OUTPUT_DIR));
        assert!(UserSettingsStore::is_valid_key(setting_keys::LANGUAGE));
        assert!(UserSettingsStore::is_valid_key(setting_keys::TIMEZONE));
        assert!(UserSettingsStore::is_valid_key(setting_keys::NOTIFICATIONS));
        assert!(UserSettingsStore::is_valid_key(setting_keys::MAX_HISTORY));
        assert!(!UserSettingsStore::is_valid_key("invalid_key"));
    }

    #[test]
    fn test_set_and_get_setting() {
        let store = UserSettingsStore::new().unwrap();

        let setting = store
            .set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output")
            .unwrap();

        assert_eq!(setting.user_id, 12345);
        assert_eq!(setting.key, setting_keys::OUTPUT_DIR);
        assert_eq!(setting.value, "/tmp/output");

        let loaded = store.get_setting(12345, setting_keys::OUTPUT_DIR).unwrap().unwrap();
        assert_eq!(loaded.value, "/tmp/output");
    }

    #[test]
    fn test_update_setting() {
        let store = UserSettingsStore::new().unwrap();

        // 初期設定
        store.set_setting(12345, setting_keys::LANGUAGE, "ja").unwrap();

        // 更新
        let updated = store.set_setting(12345, setting_keys::LANGUAGE, "en").unwrap();
        assert_eq!(updated.value, "en");

        // 確認
        let loaded = store.get_setting(12345, setting_keys::LANGUAGE).unwrap().unwrap();
        assert_eq!(loaded.value, "en");
    }

    #[test]
    fn test_get_all_settings() {
        let store = UserSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output").unwrap();
        store.set_setting(12345, setting_keys::LANGUAGE, "ja").unwrap();
        store.set_setting(12345, setting_keys::TIMEZONE, "Asia/Tokyo").unwrap();

        let settings = store.get_all_settings(12345).unwrap();
        assert_eq!(settings.len(), 3);
    }

    #[test]
    fn test_delete_setting() {
        let store = UserSettingsStore::new().unwrap();

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
        let store = UserSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp/output").unwrap();
        store.set_setting(12345, setting_keys::LANGUAGE, "ja").unwrap();

        let count = store.clear_settings(12345).unwrap();
        assert_eq!(count, 2);

        let settings = store.get_all_settings(12345).unwrap();
        assert!(settings.is_empty());
    }

    #[test]
    fn test_get_setting_with_default() {
        let store = UserSettingsStore::new().unwrap();

        // 存在しない場合はデフォルト値
        let value = store.get_setting_with_default(12345, setting_keys::LANGUAGE, "en").unwrap();
        assert_eq!(value, "en");

        // 存在する場合は設定値
        store.set_setting(12345, setting_keys::LANGUAGE, "ja").unwrap();
        let value = store.get_setting_with_default(12345, setting_keys::LANGUAGE, "en").unwrap();
        assert_eq!(value, "ja");
    }

    #[test]
    fn test_invalid_key_rejected() {
        let store = UserSettingsStore::new().unwrap();

        let result = store.set_setting(12345, "invalid_key", "value");
        assert!(matches!(result, Err(UserSettingsError::InvalidKey(_))));
    }

    #[test]
    fn test_empty_value_rejected() {
        let store = UserSettingsStore::new().unwrap();

        let result = store.set_setting(12345, setting_keys::OUTPUT_DIR, "");
        assert!(matches!(result, Err(UserSettingsError::InvalidValue(_))));

        let result = store.set_setting(12345, setting_keys::OUTPUT_DIR, "   ");
        assert!(matches!(result, Err(UserSettingsError::InvalidValue(_))));
    }

    #[test]
    fn test_list_all_users() {
        let store = UserSettingsStore::new().unwrap();

        store.set_setting(111, setting_keys::LANGUAGE, "ja").unwrap();
        store.set_setting(222, setting_keys::LANGUAGE, "en").unwrap();
        store.set_setting(333, setting_keys::LANGUAGE, "fr").unwrap();

        let users = store.list_all_users().unwrap();
        assert_eq!(users.len(), 3);
        assert!(users.contains(&111));
        assert!(users.contains(&222));
        assert!(users.contains(&333));
    }

    #[test]
    fn test_count_settings() {
        let store = UserSettingsStore::new().unwrap();

        assert_eq!(store.count_settings(12345).unwrap(), 0);

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "/tmp").unwrap();
        store.set_setting(12345, setting_keys::LANGUAGE, "ja").unwrap();

        assert_eq!(store.count_settings(12345).unwrap(), 2);
    }

    #[test]
    fn test_user_settings_struct_new() {
        let settings = UserSettings::new(12345);
        assert_eq!(settings.user_id, 12345);
        assert!(settings.output_subdir.is_none());
        assert!(settings.language.is_none());
    }

    #[test]
    fn test_user_settings_from_settings() {
        let now = Utc::now();
        let settings = vec![
            UserSetting {
                user_id: 12345,
                key: setting_keys::OUTPUT_DIR.to_string(),
                value: "my_output".to_string(),
                created_at: now,
                updated_at: now,
            },
            UserSetting {
                user_id: 12345,
                key: setting_keys::LANGUAGE.to_string(),
                value: "ja".to_string(),
                created_at: now,
                updated_at: now,
            },
        ];

        let user_settings = UserSettings::from_settings(12345, &settings);
        assert_eq!(user_settings.user_id, 12345);
        assert_eq!(user_settings.output_subdir, Some("my_output".to_string()));
        assert_eq!(user_settings.language, Some("ja".to_string()));
        assert!(user_settings.timezone.is_none());
    }

    #[test]
    fn test_user_settings_to_settings() {
        let mut user_settings = UserSettings::new(12345);
        user_settings.output_subdir = Some("output_dir".to_string());
        user_settings.language = Some("en".to_string());

        let settings = user_settings.to_settings();
        assert_eq!(settings.len(), 2);

        let output_setting = settings.iter().find(|s| s.key == setting_keys::OUTPUT_DIR);
        assert!(output_setting.is_some());
        assert_eq!(output_setting.unwrap().value, "output_dir");

        let lang_setting = settings.iter().find(|s| s.key == setting_keys::LANGUAGE);
        assert!(lang_setting.is_some());
        assert_eq!(lang_setting.unwrap().value, "en");
    }

    #[test]
    fn test_get_user_settings() {
        let store = UserSettingsStore::new().unwrap();

        store.set_setting(12345, setting_keys::OUTPUT_DIR, "my_output").unwrap();
        store.set_setting(12345, setting_keys::TIMEZONE, "Asia/Tokyo").unwrap();

        let user_settings = store.get_user_settings(12345).unwrap();
        assert_eq!(user_settings.user_id, 12345);
        assert_eq!(user_settings.output_subdir, Some("my_output".to_string()));
        assert_eq!(user_settings.timezone, Some("Asia/Tokyo".to_string()));
    }

    #[test]
    fn test_save_user_settings() {
        let store = UserSettingsStore::new().unwrap();

        let mut user_settings = UserSettings::new(12345);
        user_settings.output_subdir = Some("custom_dir".to_string());
        user_settings.notifications = Some("enabled".to_string());

        let count = store.save_user_settings(&user_settings).unwrap();
        assert_eq!(count, 2);

        // 確認
        let loaded = store.get_user_settings(12345).unwrap();
        assert_eq!(loaded.output_subdir, Some("custom_dir".to_string()));
        assert_eq!(loaded.notifications, Some("enabled".to_string()));
    }

    #[test]
    fn test_user_settings_serialization() {
        let mut user_settings = UserSettings::new(12345);
        user_settings.output_subdir = Some("test_dir".to_string());
        user_settings.language = Some("ja".to_string());

        // Serialize
        let json = serde_json::to_string(&user_settings).unwrap();
        assert!(json.contains("test_dir"));
        assert!(json.contains("ja"));

        // Deserialize
        let deserialized: UserSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, 12345);
        assert_eq!(deserialized.output_subdir, Some("test_dir".to_string()));
        assert_eq!(deserialized.language, Some("ja".to_string()));
    }
}
