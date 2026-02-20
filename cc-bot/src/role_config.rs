//! Discordロールと権限のマッピング設定
//!
//! このモジュールは`data/roles.json`からロール-権限マッピングを読み込み、
//! DiscordロールIDに基づいてユーザーに付与する権限を管理します。

use crate::permission::Permission;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use tracing::{debug, info, warn};

/// ロール設定のエラー
#[derive(Debug, Error)]
pub enum RoleConfigError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("JSON parse error: {0}")]
    ParseError(String),
}

/// 単一ロールの権限設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleEntry {
    /// ロール名（表示用）
    pub name: String,
    /// このロールに付与する権限リスト
    pub permissions: Vec<String>,
}

/// ロール設定ストア（JSON永続化用）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoleConfigStore {
    /// ロールID -> ロール設定のマッピング
    roles: HashMap<u64, RoleEntry>,
    /// デフォルト権限（ロールがないユーザーに付与）
    #[serde(default)]
    default_permissions: Vec<String>,
    /// バージョン
    version: u32,
}

impl Default for RoleConfigStore {
    fn default() -> Self {
        let mut roles = HashMap::new();

        // デフォルトのロール設定
        roles.insert(
            100000000000000000, // サンプルロールID
            RoleEntry {
                name: "Moderator".to_string(),
                permissions: vec!["FileRead".to_string(), "FileWrite".to_string(), "Schedule".to_string()],
            },
        );

        Self {
            roles,
            default_permissions: vec!["FileRead".to_string()],
            version: 1,
        }
    }
}

/// ロール設定マネージャー
#[derive(Debug, Clone)]
pub struct RoleConfig {
    /// ストア
    store: RoleConfigStore,
}

impl RoleConfig {
    /// 新しいRoleConfigを作成（デフォルト値）
    pub fn new() -> Self {
        Self {
            store: RoleConfigStore::default(),
        }
    }

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join("roles.json")
    }

    /// JSONファイルから読み込み
    ///
    /// ファイルが存在しない場合はデフォルト値を使用
    pub async fn load(base_dir: &str) -> Result<Self, RoleConfigError> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading role config from {:?}", path);

        if !path.exists() {
            info!("Role config file not found at {:?}, using defaults", path);
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| RoleConfigError::IoError(format!("Failed to read file: {}", e)))?;

        let store: RoleConfigStore = serde_json::from_str(&content).map_err(|e| {
            warn!("Failed to parse role config file, using defaults: {}", e);
            RoleConfigError::ParseError(format!("Failed to parse JSON: {}", e))
        })?;

        info!(
            "Loaded role config with {} roles",
            store.roles.len()
        );

        Ok(Self { store })
    }

    /// JSONファイルに保存
    pub async fn save(&self, base_dir: &str) -> Result<(), RoleConfigError> {
        let path = Self::get_file_path(base_dir);
        debug!("Saving role config to {:?}", path);

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| {
                        RoleConfigError::IoError(format!("Failed to create directory: {}", e))
                    })?;
            }
        }

        let content = serde_json::to_string_pretty(&self.store)
            .map_err(|e| RoleConfigError::ParseError(format!("Failed to serialize: {}", e)))?;

        fs::write(&path, content)
            .await
            .map_err(|e| RoleConfigError::IoError(format!("Failed to write file: {}", e)))?;

        info!("Saved role config with {} roles", self.store.roles.len());
        Ok(())
    }

    /// ロールIDから権限セットを取得
    ///
    /// ロールが存在しない場合は空のセットを返す
    pub fn get_permissions_for_role(&self, role_id: u64) -> HashSet<Permission> {
        let mut perms = HashSet::new();

        if let Some(role_entry) = self.store.roles.get(&role_id) {
            for perm_str in &role_entry.permissions {
                if let Some(perm) = Permission::from_str(perm_str) {
                    perms.insert(perm);
                } else {
                    warn!("Unknown permission '{}' for role {}", perm_str, role_id);
                }
            }
        }

        perms
    }

    /// 複数のロールIDから統合された権限セットを取得
    ///
    /// すべてのロールの権限をマージします
    pub fn get_permissions_for_roles(&self, role_ids: &[u64]) -> HashSet<Permission> {
        let mut perms = HashSet::new();

        for role_id in role_ids {
            let role_perms = self.get_permissions_for_role(*role_id);
            perms.extend(role_perms);
        }

        perms
    }

    /// デフォルト権限を取得
    ///
    /// ロールがないユーザーに付与される基本権限
    pub fn get_default_permissions(&self) -> HashSet<Permission> {
        let mut perms = HashSet::new();

        for perm_str in &self.store.default_permissions {
            if let Some(perm) = Permission::from_str(perm_str) {
                perms.insert(perm);
            } else {
                warn!("Unknown permission in default_permissions: {}", perm_str);
            }
        }

        perms
    }

    /// ユーザーの権限を取得（ロール＋デフォルト）
    ///
    /// ロール権限とデフォルト権限をマージして返します
    pub fn get_user_permissions(&self, role_ids: &[u64]) -> HashSet<Permission> {
        let mut perms = self.get_default_permissions();
        perms.extend(self.get_permissions_for_roles(role_ids));
        perms
    }

    /// ロール設定を追加・更新
    pub fn set_role(&mut self, role_id: u64, entry: RoleEntry) {
        self.store.roles.insert(role_id, entry);
    }

    /// ロール設定を削除
    pub fn remove_role(&mut self, role_id: u64) -> Option<RoleEntry> {
        self.store.roles.remove(&role_id)
    }

    /// すべてのロール設定を取得
    pub fn get_all_roles(&self) -> &HashMap<u64, RoleEntry> {
        &self.store.roles
    }

    /// ロールの名前を取得
    pub fn get_role_name(&self, role_id: u64) -> Option<&str> {
        self.store.roles.get(&role_id).map(|e| e.name.as_str())
    }

    /// 設定されているロール数を取得
    pub fn len(&self) -> usize {
        self.store.roles.len()
    }

    /// 設定が空かどうか
    pub fn is_empty(&self) -> bool {
        self.store.roles.is_empty()
    }
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_config() {
        let config = RoleConfig::new();
        assert!(!config.is_empty()); // デフォルト値がある
    }

    #[test]
    fn test_get_permissions_for_unknown_role() {
        let config = RoleConfig::new();
        let perms = config.get_permissions_for_role(999999);
        assert!(perms.is_empty());
    }

    #[test]
    fn test_set_and_get_role() {
        let mut config = RoleConfig::new();

        let entry = RoleEntry {
            name: "TestRole".to_string(),
            permissions: vec!["FileRead".to_string(), "Admin".to_string()],
        };

        config.set_role(12345, entry);

        let perms = config.get_permissions_for_role(12345);
        assert!(perms.contains(&Permission::FileRead));
        assert!(perms.contains(&Permission::Admin));

        assert_eq!(config.get_role_name(12345), Some("TestRole"));
    }

    #[test]
    fn test_get_permissions_for_multiple_roles() {
        let mut config = RoleConfig::new();

        config.set_role(
            100,
            RoleEntry {
                name: "Role1".to_string(),
                permissions: vec!["FileRead".to_string()],
            },
        );

        config.set_role(
            200,
            RoleEntry {
                name: "Role2".to_string(),
                permissions: vec!["FileWrite".to_string()],
            },
        );

        let perms = config.get_permissions_for_roles(&[100, 200]);
        assert!(perms.contains(&Permission::FileRead));
        assert!(perms.contains(&Permission::FileWrite));
    }

    #[test]
    fn test_remove_role() {
        let mut config = RoleConfig::new();

        config.set_role(
            12345,
            RoleEntry {
                name: "TestRole".to_string(),
                permissions: vec!["FileRead".to_string()],
            },
        );

        assert!(config.remove_role(12345).is_some());
        assert!(config.remove_role(12345).is_none());
        assert!(config.get_permissions_for_role(12345).is_empty());
    }

    #[test]
    fn test_invalid_permission_name() {
        let mut config = RoleConfig::new();

        config.set_role(
            12345,
            RoleEntry {
                name: "TestRole".to_string(),
                permissions: vec!["InvalidPermission".to_string(), "FileRead".to_string()],
            },
        );

        // 無効な権限名は無視され、有効なもののみ取得される
        let perms = config.get_permissions_for_role(12345);
        assert!(perms.contains(&Permission::FileRead));
        assert_eq!(perms.len(), 1);
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let base_dir = dir.path().to_str().unwrap();

        let mut config = RoleConfig::new();
        config.set_role(
            111,
            RoleEntry {
                name: "SavedRole".to_string(),
                permissions: vec!["Schedule".to_string()],
            },
        );

        config.save(base_dir).await.unwrap();

        let loaded = RoleConfig::load(base_dir).await.unwrap();
        let perms = loaded.get_permissions_for_role(111);
        assert!(perms.contains(&Permission::Schedule));
        assert_eq!(loaded.get_role_name(111), Some("SavedRole"));
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let base_dir = dir.path().to_str().unwrap();

        // ファイルが存在しない場合はデフォルト値
        let config = RoleConfig::load(base_dir).await.unwrap();
        assert!(!config.is_empty()); // デフォルト値がある
    }

    #[test]
    fn test_get_default_permissions() {
        let config = RoleConfig::new();
        let perms = config.get_default_permissions();
        assert!(perms.contains(&Permission::FileRead));
    }

    #[test]
    fn test_get_user_permissions() {
        let mut config = RoleConfig::new();

        config.set_role(
            100,
            RoleEntry {
                name: "Admin".to_string(),
                permissions: vec!["Admin".to_string()],
            },
        );

        // ロールがあるユーザー
        let perms_with_role = config.get_user_permissions(&[100]);
        assert!(perms_with_role.contains(&Permission::FileRead)); // デフォルト
        assert!(perms_with_role.contains(&Permission::Admin)); // ロールから

        // ロールがないユーザー
        let perms_no_role = config.get_user_permissions(&[]);
        assert!(perms_no_role.contains(&Permission::FileRead)); // デフォルトのみ
        assert!(!perms_no_role.contains(&Permission::Admin));
    }
}
