use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use tracing::{debug, error, info, warn};

/// パーミッション定義
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    /// ファイル読み取り権限
    FileRead,
    /// ファイル書き込み権限
    FileWrite,
    /// スケジュール管理権限
    Schedule,
    /// 管理者権限
    Admin,
}

impl Permission {
    /// パーミッション名から変換
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fileread" | "file_read" => Some(Permission::FileRead),
            "filewrite" | "file_write" => Some(Permission::FileWrite),
            "schedule" => Some(Permission::Schedule),
            "admin" => Some(Permission::Admin),
            _ => None,
        }
    }

    /// パーミッション名を取得
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::FileRead => "FileRead",
            Permission::FileWrite => "FileWrite",
            Permission::Schedule => "Schedule",
            Permission::Admin => "Admin",
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// デフォルトパーミッション（全ユーザーに付与）
fn default_permissions() -> HashSet<Permission> {
    let mut perms = HashSet::new();
    perms.insert(Permission::FileRead);
    perms.insert(Permission::FileWrite);
    perms.insert(Permission::Schedule);
    perms
}

/// パーミッションマネージャーのエラー
#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid permission: {0}")]
    InvalidPermission(String),

    #[error("User not found: {0}")]
    UserNotFound(u64),
}

/// パーミッションストア（JSON永続化用）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermissionStore {
    /// ユーザーID -> カスタムパーミッション（デフォルトに追加）
    custom_permissions: HashMap<u64, HashSet<Permission>>,
    /// 管理者ユーザーID
    admins: HashSet<u64>,
    /// バージョン
    version: u32,
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self {
            custom_permissions: HashMap::new(),
            admins: HashSet::new(),
            version: 1,
        }
    }
}

/// パーミッションマネージャー
#[derive(Debug, Clone)]
pub struct PermissionManager {
    /// ストア
    store: PermissionStore,
}

impl PermissionManager {
    /// 新しいPermissionManagerを作成
    pub fn new() -> Self {
        Self {
            store: PermissionStore::default(),
        }
    }

    /// 環境変数から管理者を読み込み
    pub fn load_admins_from_env(&mut self) {
        if let Ok(admin_ids) = env::var("ADMIN_USER_IDS") {
            for id_str in admin_ids.split(',') {
                let id_str = id_str.trim();
                if id_str.is_empty() {
                    continue;
                }
                if let Ok(id) = id_str.parse::<u64>() {
                    self.store.admins.insert(id);
                    // 管理者にはAdmin権限も付与
                    self.store
                        .custom_permissions
                        .entry(id)
                        .or_insert_with(HashSet::new)
                        .insert(Permission::Admin);
                    info!("Loaded admin user: {}", id);
                } else {
                    warn!("Invalid admin user ID: {}", id_str);
                }
            }
        }
        debug!("Loaded {} admin users", self.store.admins.len());
    }

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join("permissions.json")
    }

    /// JSONファイルから読み込み
    pub async fn load(base_dir: &str) -> Result<Self, PermissionError> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading permissions from {:?}", path);

        if !path.exists() {
            info!("Permission file not found, creating new manager");
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| PermissionError::StorageError(format!("Failed to read file: {}", e)))?;

        let store: PermissionStore = serde_json::from_str(&content)
            .map_err(|e| {
                warn!("Failed to parse permission file, creating new manager: {}", e);
                PermissionError::StorageError(format!("Failed to parse JSON: {}", e))
            })?;

        info!(
            "Loaded permissions for {} users, {} admins",
            store.custom_permissions.len(),
            store.admins.len()
        );

        Ok(Self { store })
    }

    /// JSONファイルに保存
    pub async fn save(&self, base_dir: &str) -> Result<(), PermissionError> {
        let path = Self::get_file_path(base_dir);
        debug!("Saving permissions to {:?}", path);

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| {
                        PermissionError::StorageError(format!("Failed to create directory: {}", e))
                    })?;
            }
        }

        let content = serde_json::to_string_pretty(&self.store)
            .map_err(|e| PermissionError::StorageError(format!("Failed to serialize: {}", e)))?;

        fs::write(&path, content)
            .await
            .map_err(|e| PermissionError::StorageError(format!("Failed to write file: {}", e)))?;

        info!(
            "Saved permissions for {} users",
            self.store.custom_permissions.len()
        );
        Ok(())
    }

    /// ユーザーが管理者かどうか
    pub fn is_admin(&self, user_id: u64) -> bool {
        self.store.admins.contains(&user_id)
    }

    /// ユーザーの全パーミッションを取得
    pub fn get_permissions(&self, user_id: u64) -> HashSet<Permission> {
        let mut perms = default_permissions();

        // カスタムパーミッションを追加
        if let Some(custom) = self.store.custom_permissions.get(&user_id) {
            for perm in custom {
                perms.insert(perm.clone());
            }
        }

        perms
    }

    /// ユーザーが特定のパーミッションを持っているか
    pub fn has_permission(&self, user_id: u64, permission: &Permission) -> bool {
        self.get_permissions(user_id).contains(permission)
    }

    /// ユーザーにパーミッションを付与（管理者のみ）
    pub fn grant_permission(
        &mut self,
        admin_id: u64,
        target_user_id: u64,
        permission: Permission,
    ) -> Result<bool, PermissionError> {
        // 管理者権限チェック
        if !self.is_admin(admin_id) {
            return Err(PermissionError::PermissionDenied(
                "Only admins can grant permissions".to_string(),
            ));
        }

        // Admin権限の付与は禁止（環境変数でのみ設定可能）
        if permission == Permission::Admin {
            return Err(PermissionError::PermissionDenied(
                "Admin permission can only be set via ADMIN_USER_IDS environment variable"
                    .to_string(),
            ));
        }

        let perms = self
            .store
            .custom_permissions
            .entry(target_user_id)
            .or_insert_with(HashSet::new);

        let added = perms.insert(permission.clone());
        if added {
            info!(
                "Granted {} to user {} by admin {}",
                permission, target_user_id, admin_id
            );
        }

        Ok(added)
    }

    /// ユーザーからパーミッションを剥奪（管理者のみ）
    pub fn revoke_permission(
        &mut self,
        admin_id: u64,
        target_user_id: u64,
        permission: Permission,
    ) -> Result<bool, PermissionError> {
        // 管理者権限チェック
        if !self.is_admin(admin_id) {
            return Err(PermissionError::PermissionDenied(
                "Only admins can revoke permissions".to_string(),
            ));
        }

        // Admin権限の剥奪は禁止
        if permission == Permission::Admin {
            return Err(PermissionError::PermissionDenied(
                "Admin permission can only be modified via ADMIN_USER_IDS environment variable"
                    .to_string(),
            ));
        }

        if let Some(perms) = self.store.custom_permissions.get_mut(&target_user_id) {
            let removed = perms.remove(&permission);
            if removed {
                info!(
                    "Revoked {} from user {} by admin {}",
                    permission, target_user_id, admin_id
                );
            }

            // 空のエントリを削除
            if perms.is_empty() {
                self.store.custom_permissions.remove(&target_user_id);
            }

            Ok(removed)
        } else {
            Ok(false)
        }
    }

    /// 管理者一覧を取得
    pub fn get_admins(&self) -> &HashSet<u64> {
        &self.store.admins
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_permission_from_str() {
        assert_eq!(Permission::from_str("FileRead"), Some(Permission::FileRead));
        assert_eq!(Permission::from_str("file_read"), Some(Permission::FileRead));
        assert_eq!(Permission::from_str("invalid"), None);
    }

    #[test]
    fn test_default_permissions() {
        let perms = default_permissions();
        assert!(perms.contains(&Permission::FileRead));
        assert!(perms.contains(&Permission::FileWrite));
        assert!(perms.contains(&Permission::Schedule));
        assert!(!perms.contains(&Permission::Admin));
    }

    #[test]
    fn test_new_manager() {
        let manager = PermissionManager::new();
        assert!(!manager.is_admin(12345));
        assert!(manager.has_permission(12345, &Permission::FileRead));
        assert!(!manager.has_permission(12345, &Permission::Admin));
    }

    #[test]
    fn test_grant_revoke_permission() {
        let mut manager = PermissionManager::new();
        // 管理者を設定（直接storeに追加 - テスト用）
        manager.store.admins.insert(1);
        manager
            .store
            .custom_permissions
            .entry(1)
            .or_insert_with(HashSet::new)
            .insert(Permission::Admin);

        // 非管理者が権限付与を試みる -> エラー
        let result = manager.grant_permission(2, 3, Permission::Schedule);
        assert!(result.is_err());

        // 管理者が権限を付与
        let result = manager.grant_permission(1, 2, Permission::Schedule);
        assert!(result.is_ok());
        assert!(manager.has_permission(2, &Permission::Schedule));

        // 既に持っている権限を再度付与 -> false
        let result = manager.grant_permission(1, 2, Permission::Schedule);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // 既に持っているのでfalse

        // 管理者が権限を剥奪
        let result = manager.revoke_permission(1, 2, Permission::Schedule);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cannot_grant_admin() {
        let mut manager = PermissionManager::new();
        manager.store.admins.insert(1);
        manager
            .store
            .custom_permissions
            .entry(1)
            .or_insert_with(HashSet::new)
            .insert(Permission::Admin);

        // Admin権限の付与は禁止
        let result = manager.grant_permission(1, 2, Permission::Admin);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_save_load() {
        let dir = tempdir().unwrap();
        let base_dir = dir.path().to_str().unwrap();

        let mut manager = PermissionManager::new();
        manager.store.admins.insert(1);
        manager
            .store
            .custom_permissions
            .entry(1)
            .or_insert_with(HashSet::new)
            .insert(Permission::Admin);

        manager.grant_permission(1, 2, Permission::Schedule).unwrap();
        manager.save(base_dir).await.unwrap();

        let loaded = PermissionManager::load(base_dir).await.unwrap();
        assert!(loaded.is_admin(1));
        assert!(loaded.has_permission(2, &Permission::Schedule));
    }
}
