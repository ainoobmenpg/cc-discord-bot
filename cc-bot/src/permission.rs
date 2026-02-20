#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use tracing::{debug, info, warn};

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
    /// スーパーユーザー権限（全権限を持ち、全チェックで最優先）
    SuperUser,
}

impl Permission {
    /// パーミッション名から変換
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fileread" | "file_read" => Some(Permission::FileRead),
            "filewrite" | "file_write" => Some(Permission::FileWrite),
            "schedule" => Some(Permission::Schedule),
            "admin" => Some(Permission::Admin),
            "superuser" | "super_user" | "super-user" => Some(Permission::SuperUser),
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
            Permission::SuperUser => "SuperUser",
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
    /// スーパーユーザーID
    super_users: HashSet<u64>,
    /// バージョン
    version: u32,
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self {
            custom_permissions: HashMap::new(),
            admins: HashSet::new(),
            super_users: HashSet::new(),
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

    /// 環境変数からスーパーユーザーを読み込み
    pub fn load_super_users_from_env(&mut self) {
        if let Ok(super_user_ids) = env::var("SUPER_USER_IDS") {
            for id_str in super_user_ids.split(',') {
                let id_str = id_str.trim();
                if id_str.is_empty() {
                    continue;
                }
                if let Ok(id) = id_str.parse::<u64>() {
                    self.store.super_users.insert(id);
                    // スーパーユーザーにはSuperUser権限も付与
                    self.store
                        .custom_permissions
                        .entry(id)
                        .or_insert_with(HashSet::new)
                        .insert(Permission::SuperUser);
                    info!("Loaded super user: {}", id);
                } else {
                    warn!("Invalid super user ID: {}", id_str);
                }
            }
        }
        debug!("Loaded {} super users", self.store.super_users.len());
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
            "Loaded permissions for {} users, {} admins, {} super users",
            store.custom_permissions.len(),
            store.admins.len(),
            store.super_users.len()
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

    /// ユーザーがスーパーユーザーかどうか
    pub fn is_super_user(&self, user_id: u64) -> bool {
        self.store.super_users.contains(&user_id)
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

    /// ユーザーの全パーミッションを取得（ロールベース）
    ///
    /// 権限チェックフロー:
    /// 1. SuperUser? → 全チェックバイパス
    /// 2. 個別ユーザー権限? → 適用
    /// 3. ロール権限? → 適用
    /// 4. デフォルト権限 → 適用
    pub fn get_permissions_with_roles(
        &self,
        user_id: u64,
        role_ids: &[u64],
        role_config: &crate::role_config::RoleConfig,
    ) -> HashSet<Permission> {
        // スーパーユーザーは全権限を持つ
        if self.is_super_user(user_id) {
            // 全ての権限を返す
            let mut all_perms = HashSet::new();
            all_perms.insert(Permission::FileRead);
            all_perms.insert(Permission::FileWrite);
            all_perms.insert(Permission::Schedule);
            all_perms.insert(Permission::Admin);
            all_perms.insert(Permission::SuperUser);
            return all_perms;
        }

        let mut perms = HashSet::new();

        // 1. デフォルト権限（ベース）
        perms.extend(role_config.get_default_permissions());

        // 2. ロール権限をマージ
        perms.extend(role_config.get_permissions_for_roles(role_ids));

        // 3. 個別ユーザー権限をマージ（最優先）
        if let Some(custom) = self.store.custom_permissions.get(&user_id) {
            for perm in custom {
                perms.insert(perm.clone());
            }
        }

        perms
    }

    /// ユーザーが特定のパーミッションを持っているか（ロールベース）
    ///
    /// SuperUser権限を持つユーザーは全ての権限を持っているとみなす
    pub fn has_permission_with_roles(
        &self,
        user_id: u64,
        permission: &Permission,
        role_ids: &[u64],
        role_config: &crate::role_config::RoleConfig,
    ) -> bool {
        let perms = self.get_permissions_with_roles(user_id, role_ids, role_config);
        perms.contains(permission)
    }

    /// ユーザーが特定のパーミッションを持っているか
    /// SuperUser権限を持つユーザーは全ての権限を持っているとみなす
    pub fn has_permission(&self, user_id: u64, permission: &Permission) -> bool {
        let perms = self.get_permissions(user_id);
        // SuperUserは全ての権限を持つ
        if perms.contains(&Permission::SuperUser) {
            return true;
        }
        perms.contains(permission)
    }

    /// ユーザーにパーミッションを付与（管理者またはスーパーユーザーのみ）
    pub fn grant_permission(
        &mut self,
        admin_id: u64,
        target_user_id: u64,
        permission: Permission,
    ) -> Result<bool, PermissionError> {
        // 管理者またはスーパーユーザー権限チェック
        if !self.is_admin(admin_id) && !self.is_super_user(admin_id) {
            return Err(PermissionError::PermissionDenied(
                "Only admins or super users can grant permissions".to_string(),
            ));
        }

        // Admin権限の付与はスーパーユーザーのみ可能
        if permission == Permission::Admin && !self.is_super_user(admin_id) {
            return Err(PermissionError::PermissionDenied(
                "Admin permission can only be granted by super users".to_string(),
            ));
        }

        // SuperUser権限の付与は禁止
        if permission == Permission::SuperUser {
            return Err(PermissionError::PermissionDenied(
                "SuperUser permission can only be set via SUPER_USER_IDS environment variable"
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
                "Granted {} to user {} by admin/superuser {}",
                permission, target_user_id, admin_id
            );
        }

        Ok(added)
    }

    /// ユーザーからパーミッションを剥奪（管理者またはスーパーユーザーのみ）
    pub fn revoke_permission(
        &mut self,
        admin_id: u64,
        target_user_id: u64,
        permission: Permission,
    ) -> Result<bool, PermissionError> {
        // 管理者またはスーパーユーザー権限チェック
        if !self.is_admin(admin_id) && !self.is_super_user(admin_id) {
            return Err(PermissionError::PermissionDenied(
                "Only admins or super users can revoke permissions".to_string(),
            ));
        }

        // Admin権限の剥奪はスーパーユーザーのみ可能
        if permission == Permission::Admin && !self.is_super_user(admin_id) {
            return Err(PermissionError::PermissionDenied(
                "Admin permission can only be revoked by super users".to_string(),
            ));
        }

        // SuperUser権限の剥奪は禁止
        if permission == Permission::SuperUser {
            return Err(PermissionError::PermissionDenied(
                "SuperUser permission can only be modified via SUPER_USER_IDS environment variable"
                    .to_string(),
            ));
        }

        if let Some(perms) = self.store.custom_permissions.get_mut(&target_user_id) {
            let removed = perms.remove(&permission);
            if removed {
                info!(
                    "Revoked {} from user {} by admin/superuser {}",
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
