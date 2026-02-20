//! Discordユーザーロール取得モジュール
//!
//! SerenityのGuild APIを使用して、Discordサーバー上のユーザーロールを取得します。
//! キャッシュ機能によるパフォーマンス最適化も提供します。

use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// デフォルトのキャッシュTTL（秒）
const DEFAULT_CACHE_TTL_SECS: u64 = 300; // 5分

/// ロールキャッシュエントリ
#[derive(Debug, Clone)]
struct RoleCacheEntry {
    /// ユーザーが持つロールIDのセット
    role_ids: HashSet<u64>,
    /// キャッシュ作成時刻
    cached_at: Instant,
}

/// ユーザーロール取得のキャッシュ
#[derive(Debug)]
pub struct UserRoleCache {
    /// キャッシュエントリ（GuildId + UserId -> RoleCacheEntry）
    cache: Arc<RwLock<HashMap<(u64, u64), RoleCacheEntry>>>,
    /// キャッシュの有効期限（秒）
    ttl: Duration,
}

impl Default for UserRoleCache {
    fn default() -> Self {
        Self::new()
    }
}

impl UserRoleCache {
    /// 新しいUserRoleCacheを作成（TTLは環境変数から読み込み、デフォルト: 5分）
    pub fn new() -> Self {
        let ttl_secs = env::var("ROLE_CACHE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_CACHE_TTL_SECS);
        Self::with_ttl(Duration::from_secs(ttl_secs))
    }

    /// カスタムTTLでUserRoleCacheを作成
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// キャッシュからロールを取得
    pub async fn get_cached(&self, guild_id: u64, user_id: u64) -> Option<HashSet<u64>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(&(guild_id, user_id)) {
            if entry.cached_at.elapsed() < self.ttl {
                debug!("Cache hit for user {} in guild {}", user_id, guild_id);
                return Some(entry.role_ids.clone());
            }
        }
        None
    }

    /// キャッシュにロールを保存
    pub async fn set_cache(&self, guild_id: u64, user_id: u64, role_ids: HashSet<u64>) {
        let mut cache = self.cache.write().await;
        cache.insert(
            (guild_id, user_id),
            RoleCacheEntry {
                role_ids,
                cached_at: Instant::now(),
            },
        );
        debug!("Cached roles for user {} in guild {}", user_id, guild_id);
    }

    /// 期限切れエントリをクリーンアップ
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let before = cache.len();
        cache.retain(|_, entry| entry.cached_at.elapsed() < self.ttl);
        let removed = before - cache.len();
        if removed > 0 {
            debug!("Cleaned up {} expired cache entries", removed);
        }
    }

    /// 全キャッシュをクリア
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        debug!("Cleared all role cache entries");
    }
}

impl Clone for UserRoleCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            ttl: self.ttl,
        }
    }
}

/// Discord Guild APIからユーザーのロールを取得するヘルパー関数
///
/// # Arguments
/// * `ctx` - Serenity Context
/// * `guild_id` - サーバーID
/// * `user_id` - ユーザーID
/// * `cache` - オプションのキャッシュ
///
/// # Returns
/// * `Ok(HashSet<u64>)` - ユーザーが持つロールIDのセット
/// * `Err(String)` - エラーメッセージ
pub async fn get_user_roles(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    cache: Option<&UserRoleCache>,
) -> Result<HashSet<u64>, String> {
    let guild_id_u64 = guild_id.get();
    let user_id_u64 = user_id.get();

    // キャッシュをチェック
    if let Some(c) = cache {
        if let Some(cached_roles) = c.get_cached(guild_id_u64, user_id_u64).await {
            return Ok(cached_roles);
        }
    }

    // Guildからメンバー情報を取得
    let member = guild_id
        .member(&ctx.http, user_id)
        .await
        .map_err(|e| format!("ユーザー情報の取得に失敗しました: {}", e))?;

    // ロールIDを収集
    let role_ids: HashSet<u64> = member.roles.iter().map(|r| r.get()).collect();

    debug!(
        "User {} has {} roles in guild {}",
        user_id_u64,
        role_ids.len(),
        guild_id_u64
    );

    // キャッシュに保存
    if let Some(c) = cache {
        c.set_cache(guild_id_u64, user_id_u64, role_ids.clone()).await;
    }

    Ok(role_ids)
}

/// ユーザーが特定のロールを持っているかチェック
pub async fn has_role(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    role_id: u64,
    cache: Option<&UserRoleCache>,
) -> Result<bool, String> {
    let roles = get_user_roles(ctx, guild_id, user_id, cache).await?;
    Ok(roles.contains(&role_id))
}

/// ユーザーが指定したロールのいずれかを持っているかチェック
pub async fn has_any_role(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    role_ids: &[u64],
    cache: Option<&UserRoleCache>,
) -> Result<bool, String> {
    let roles = get_user_roles(ctx, guild_id, user_id, cache).await?;
    Ok(role_ids.iter().any(|r| roles.contains(r)))
}

/// ユーザーが指定したすべてのロールを持っているかチェック
pub async fn has_all_roles(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    role_ids: &[u64],
    cache: Option<&UserRoleCache>,
) -> Result<bool, String> {
    let roles = get_user_roles(ctx, guild_id, user_id, cache).await?;
    Ok(role_ids.iter().all(|r| roles.contains(r)))
}

/// Guildからロール名を取得してロールIDに変換
pub async fn get_role_id_by_name(
    ctx: &Context,
    guild_id: GuildId,
    role_name: &str,
) -> Result<Option<u64>, String> {
    let roles = guild_id
        .roles(&ctx.http)
        .await
        .map_err(|e| format!("ロール一覧の取得に失敗しました: {}", e))?;

    for (id, role) in roles {
        if role.name.eq_ignore_ascii_case(role_name) {
            return Ok(Some(id.get()));
        }
    }

    Ok(None)
}

/// ユーザーのロール一覧をロール名で取得
pub async fn get_user_role_names(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    cache: Option<&UserRoleCache>,
) -> Result<Vec<String>, String> {
    let user_role_ids = get_user_roles(ctx, guild_id, user_id, cache).await?;

    let guild_roles = guild_id
        .roles(&ctx.http)
        .await
        .map_err(|e| format!("ロール一覧の取得に失敗しました: {}", e))?;

    let mut role_names: Vec<String> = guild_roles
        .iter()
        .filter(|(id, _)| user_role_ids.contains(&id.get()))
        .map(|(_, role)| role.name.clone())
        .collect();

    role_names.sort();
    Ok(role_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_new() {
        let cache = UserRoleCache::new();
        assert!(cache.get_cached(123, 456).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = UserRoleCache::new();
        let mut roles = HashSet::new();
        roles.insert(100);
        roles.insert(200);

        cache.set_cache(123, 456, roles.clone()).await;
        let cached = cache.get_cached(123, 456).await;

        assert!(cached.is_some());
        let cached_roles = cached.unwrap();
        assert_eq!(cached_roles.len(), 2);
        assert!(cached_roles.contains(&100));
        assert!(cached_roles.contains(&200));
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let cache = UserRoleCache::with_ttl(Duration::from_millis(50));
        let mut roles = HashSet::new();
        roles.insert(100);

        cache.set_cache(123, 456, roles).await;
        assert!(cache.get_cached(123, 456).await.is_some());

        // TTL経過待機
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(cache.get_cached(123, 456).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = UserRoleCache::with_ttl(Duration::from_millis(50));
        let mut roles = HashSet::new();
        roles.insert(100);

        cache.set_cache(123, 456, roles).await;

        // TTL経過待機
        tokio::time::sleep(Duration::from_millis(60)).await;

        cache.cleanup_expired().await;
        assert!(cache.get_cached(123, 456).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = UserRoleCache::new();
        let mut roles = HashSet::new();
        roles.insert(100);

        cache.set_cache(123, 456, roles).await;
        assert!(cache.get_cached(123, 456).await.is_some());

        cache.clear().await;
        assert!(cache.get_cached(123, 456).await.is_none());
    }

    #[test]
    fn test_cache_clone() {
        let cache1 = UserRoleCache::new();
        let cache2 = cache1.clone();

        // クローンは同じ内部Arcを共有
        assert!(Arc::ptr_eq(&cache1.cache, &cache2.cache));
    }

    #[tokio::test]
    async fn test_cache_multiple_users_same_guild() {
        // 同じギルド内の複数ユーザーのキャッシュをテスト
        let cache = UserRoleCache::new();

        let mut roles1 = HashSet::new();
        roles1.insert(100);
        roles1.insert(200);

        let mut roles2 = HashSet::new();
        roles2.insert(100);
        roles2.insert(300);

        cache.set_cache(123, 456, roles1).await;
        cache.set_cache(123, 789, roles2).await;

        // それぞれのユーザーのキャッシュが正しく取得できる
        let cached1 = cache.get_cached(123, 456).await;
        let cached2 = cache.get_cached(123, 789).await;

        assert!(cached1.is_some());
        assert!(cached2.is_some());
        assert_ne!(cached1.unwrap(), cached2.unwrap());
    }

    #[tokio::test]
    async fn test_cache_same_user_different_guilds() {
        // 同じユーザーが異なるギルドに所属する場合のキャッシュをテスト
        let cache = UserRoleCache::new();

        let mut roles_guild1 = HashSet::new();
        roles_guild1.insert(100);

        let mut roles_guild2 = HashSet::new();
        roles_guild2.insert(200);

        cache.set_cache(111, 12345, roles_guild1).await;
        cache.set_cache(222, 12345, roles_guild2).await;

        // それぞれのギルドでのロールが正しく取得できる
        let cached_guild1 = cache.get_cached(111, 12345).await;
        let cached_guild2 = cache.get_cached(222, 12345).await;

        assert!(cached_guild1.is_some());
        assert!(cached_guild2.is_some());
        assert!(cached_guild1.unwrap().contains(&100));
        assert!(cached_guild2.unwrap().contains(&200));
    }

    #[tokio::test]
    async fn test_cache_update() {
        // キャッシュの更新をテスト
        let cache = UserRoleCache::new();

        let mut roles_v1 = HashSet::new();
        roles_v1.insert(100);

        cache.set_cache(123, 456, roles_v1).await;
        let cached = cache.get_cached(123, 456).await.unwrap();
        assert_eq!(cached.len(), 1);
        assert!(cached.contains(&100));

        // ロールを更新
        let mut roles_v2 = HashSet::new();
        roles_v2.insert(100);
        roles_v2.insert(200);
        roles_v2.insert(300);

        cache.set_cache(123, 456, roles_v2).await;
        let cached = cache.get_cached(123, 456).await.unwrap();
        assert_eq!(cached.len(), 3);
        assert!(cached.contains(&100));
        assert!(cached.contains(&200));
        assert!(cached.contains(&300));
    }

    #[tokio::test]
    async fn test_cache_concurrent_access() {
        // 並行アクセスのテスト
        let cache = Arc::new(UserRoleCache::new());
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = tokio::spawn(async move {
                let guild_id = 123u64;
                let user_id = i as u64;

                let mut roles = HashSet::new();
                roles.insert(i * 10);

                cache_clone.set_cache(guild_id, user_id, roles).await;
                let cached = cache_clone.get_cached(guild_id, user_id).await;

                assert!(cached.is_some());
                assert!(cached.unwrap().contains(&(i * 10)));
            });
            handles.push(handle);
        }

        // すべてのタスクが完了するのを待つ
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[test]
    fn test_cache_ttl_default() {
        // デフォルトTTLが5分（300秒）であることを確認
        let cache = UserRoleCache::new();
        assert_eq!(cache.ttl, Duration::from_secs(300));
    }

    #[tokio::test]
    async fn test_cache_partial_cleanup() {
        // 一部のエントリだけが期限切れになるケースをテスト
        let cache = UserRoleCache::with_ttl(Duration::from_millis(100));

        let mut roles = HashSet::new();
        roles.insert(100);

        // 最初のエントリを追加
        cache.set_cache(123, 456, roles.clone()).await;

        // 少し待機してから2つ目のエントリを追加
        tokio::time::sleep(Duration::from_millis(60)).await;
        cache.set_cache(123, 789, roles.clone()).await;

        // さらに待機して1つ目のエントリが期限切れになるようにする
        tokio::time::sleep(Duration::from_millis(60)).await;

        // クリーンアップ実行
        cache.cleanup_expired().await;

        // 1つ目は期限切れ、2つ目はまだ有効
        assert!(cache.get_cached(123, 456).await.is_none());
        assert!(cache.get_cached(123, 789).await.is_some());
    }
}
