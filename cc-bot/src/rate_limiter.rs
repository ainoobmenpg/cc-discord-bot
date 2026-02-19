use std::collections::HashMap;
use std::time::Instant;

/// ユーザーごとのリクエスト制限を管理するレートリミッター
pub struct RateLimiter {
    /// ユーザーID -> タイムスタンプリスト
    requests: HashMap<u64, Vec<Instant>>,
    /// 時間窓内の最大リクエスト数
    max_requests: usize,
    /// 時間窓の秒数
    window_secs: u64,
}

impl RateLimiter {
    /// デフォルト設定（1分間に10リクエスト）で作成
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
            max_requests: 10,
            window_secs: 60,
        }
    }

    /// カスタム設定で作成
    pub fn with_config(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: HashMap::new(),
            max_requests,
            window_secs,
        }
    }

    /// 指定ユーザーがリクエスト可能かチェック
    ///
    /// # Arguments
    /// * `user_id` - DiscordユーザーID
    ///
    /// # Returns
    /// * `true` - リクエスト可能
    /// * `false` - 制限超過
    pub fn check(&self, user_id: u64) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        if let Some(timestamps) = self.requests.get(&user_id) {
            // 時間窓内のリクエスト数をカウント
            let recent_count = timestamps
                .iter()
                .filter(|&&t| now.duration_since(t) < window)
                .count();

            recent_count < self.max_requests
        } else {
            true
        }
    }

    /// リクエストを記録
    ///
    /// # Arguments
    /// * `user_id` - DiscordユーザーID
    pub fn record(&mut self, user_id: u64) {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        let timestamps = self.requests.entry(user_id).or_insert_with(Vec::new);

        // 古いエントリを削除（パフォーマンス最適化）
        timestamps.retain(|&t| now.duration_since(t) < window);

        // 新しいリクエストを追加
        timestamps.push(now);
    }

    /// 残りリクエスト可能数を返す
    ///
    /// # Arguments
    /// * `user_id` - DiscordユーザーID
    ///
    /// # Returns
    /// * 残りリクエスト数（0以上）
    pub fn remaining(&self, user_id: u64) -> usize {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        if let Some(timestamps) = self.requests.get(&user_id) {
            let recent_count = timestamps
                .iter()
                .filter(|&&t| now.duration_since(t) < window)
                .count();

            self.max_requests.saturating_sub(recent_count)
        } else {
            self.max_requests
        }
    }

    /// 制限解除までの秒数を返す
    ///
    /// # Arguments
    /// * `user_id` - DiscordユーザーID
    ///
    /// # Returns
    /// * `Some(秒数)` - 制限中の場合
    /// * `None` - 制限されていない場合
    pub fn retry_after(&self, user_id: u64) -> Option<u64> {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        if let Some(timestamps) = self.requests.get(&user_id) {
            // 時間窓内の最も古いリクエストを見つける
            let oldest_in_window = timestamps
                .iter()
                .filter(|&&t| now.duration_since(t) < window)
                .min()?;

            let recent_count = timestamps
                .iter()
                .filter(|&&t| now.duration_since(t) < window)
                .count();

            if recent_count >= self.max_requests {
                // 最も古いリクエストが期限切れになるまでの秒数
                let elapsed = now.duration_since(*oldest_in_window);
                let remaining = window.saturating_sub(elapsed);
                Some(remaining.as_secs() + 1) // 切り上げ
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 期限切れエントリをクリーンアップ
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        self.requests.retain(|_, timestamps| {
            timestamps.retain(|&t| now.duration_since(t) < window);
            !timestamps.is_empty()
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_creates_default_settings() {
        let limiter = RateLimiter::new();
        assert_eq!(limiter.remaining(12345), 10);
    }

    #[test]
    fn test_with_config_custom_settings() {
        let limiter = RateLimiter::with_config(5, 30);
        assert_eq!(limiter.remaining(12345), 5);
    }

    #[test]
    fn test_check_returns_true_for_new_user() {
        let limiter = RateLimiter::new();
        assert!(limiter.check(12345));
    }

    #[test]
    fn test_remaining_returns_max_for_new_user() {
        let limiter = RateLimiter::new();
        assert_eq!(limiter.remaining(12345), 10);
    }

    #[test]
    fn test_record_decreases_remaining() {
        let mut limiter = RateLimiter::new();

        limiter.record(12345);
        assert_eq!(limiter.remaining(12345), 9);

        limiter.record(12345);
        assert_eq!(limiter.remaining(12345), 8);
    }

    #[test]
    fn test_check_returns_false_when_limit_exceeded() {
        let mut limiter = RateLimiter::with_config(3, 60);

        // 3回記録
        for _ in 0..3 {
            limiter.record(12345);
        }

        // 制限超過
        assert!(!limiter.check(12345));
        assert_eq!(limiter.remaining(12345), 0);
    }

    #[test]
    fn test_different_users_have_separate_limits() {
        let mut limiter = RateLimiter::with_config(2, 60);

        // ユーザー1: 2回記録
        limiter.record(111);
        limiter.record(111);
        assert!(!limiter.check(111));

        // ユーザー2: 制限されていない
        assert!(limiter.check(222));
        assert_eq!(limiter.remaining(222), 2);
    }

    #[test]
    fn test_remaining_returns_zero_when_exhausted() {
        let mut limiter = RateLimiter::with_config(2, 60);

        limiter.record(12345);
        limiter.record(12345);

        assert_eq!(limiter.remaining(12345), 0);
    }

    #[test]
    fn test_retry_after_returns_none_when_not_limited() {
        let limiter = RateLimiter::new();
        assert!(limiter.retry_after(12345).is_none());

        let mut limiter = RateLimiter::with_config(5, 60);
        limiter.record(12345);
        assert!(limiter.retry_after(12345).is_none());
    }

    #[test]
    fn test_retry_after_returns_seconds_when_limited() {
        let mut limiter = RateLimiter::with_config(1, 60);

        limiter.record(12345);

        // 制限超過
        let retry_secs = limiter.retry_after(12345);
        assert!(retry_secs.is_some());
        assert!(retry_secs.unwrap() <= 60);
    }

    #[test]
    fn test_record_removes_old_entries() {
        let mut limiter = RateLimiter::with_config(3, 1); // 1秒ウィンドウ

        // 最初のリクエスト
        limiter.record(12345);

        // ウィンドウ内で待機
        thread::sleep(Duration::from_millis(500));

        limiter.record(12345);
        assert_eq!(limiter.remaining(12345), 1);

        // ウィンドウが過ぎるまで待機
        thread::sleep(Duration::from_millis(600));

        // 古いエントリが削除される
        limiter.record(12345);
        assert!(limiter.check(12345));
    }

    #[test]
    fn test_cleanup_removes_expired_entries() {
        let mut limiter = RateLimiter::with_config(1, 1);

        limiter.record(12345);

        // ウィンドウが過ぎるまで待機
        thread::sleep(Duration::from_millis(1100));

        limiter.cleanup();

        // 制限が解除されている
        assert!(limiter.check(12345));
        assert_eq!(limiter.remaining(12345), 1);
    }

    #[test]
    fn test_saturating_remaining() {
        let mut limiter = RateLimiter::with_config(1, 60);

        limiter.record(12345);
        limiter.record(12345); // 制限超過で追加

        // remaining は0未満にならない
        assert_eq!(limiter.remaining(12345), 0);
    }
}
