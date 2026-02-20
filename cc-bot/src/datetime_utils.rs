//! DateTime操作のヘルパー関数

use chrono::{DateTime, Utc};
use tracing::warn;

/// RFC3339形式の文字列をDateTime<Utc>にパースする
///
/// パースに失敗した場合は警告ログを出力し、現在時刻を返す
///
/// # Arguments
/// * `s` - RFC3339形式の日時文字列
///
/// # Returns
/// * `DateTime<Utc>` - パース成功時は変換結果、失敗時は現在時刻
pub fn parse_rfc3339_or_now(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|e| {
            warn!("Failed to parse datetime '{}': {}, using current time", s, e);
            Utc::now()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_rfc3339_valid() {
        let result = parse_rfc3339_or_now("2024-01-15T10:30:00Z");
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_parse_rfc3339_with_timezone() {
        let result = parse_rfc3339_or_now("2024-01-15T10:30:00+09:00");
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_parse_rfc3339_invalid_returns_now() {
        let result = parse_rfc3339_or_now("invalid-datetime");
        // 現在時刻が返されることを確認（年が現在の年と一致する）
        let now = Utc::now();
        assert_eq!(result.year(), now.year());
    }

    #[test]
    fn test_parse_rfc3339_empty_returns_now() {
        let result = parse_rfc3339_or_now("");
        let now = Utc::now();
        assert_eq!(result.year(), now.year());
    }
}
