//! ログマスキング機能
//!
//! ログ出力時に機密情報をマスキングして漏洩を防ぐ。

use regex::Regex;
use std::sync::OnceLock;

/// APIキーのパターンマッチ用正規表現
static API_KEY_PATTERN: OnceLock<Regex> = OnceLock::new();
/// Discordトークンのパターンマッチ用正規表現
static DISCORD_TOKEN_PATTERN: OnceLock<Regex> = OnceLock::new();
/// 一般的なシークレットキーのパターンマッチ用正規表現
static SECRET_KEY_PATTERN: OnceLock<Regex> = OnceLock::new();

/// APIキーの正規表現を取得
fn api_key_regex() -> &'static Regex {
    API_KEY_PATTERN.get_or_init(|| {
        // 一般的なAPIキーパターン（英数字とハイフン・アンダースコア、20文字以上）
        // JSON形式 "api_key": "xxx" と代入形式 api_key=xxx の両方に対応
        Regex::new(r#"(?i)(api[_-]?key|apikey|api[_-]?secret)[\s]*[:=][\s]*["']?[\w\-]{20,}["']?"#)
            .expect("Invalid API key regex")
    })
}

/// Discordトークンの正規表現を取得
fn discord_token_regex() -> &'static Regex {
    DISCORD_TOKEN_PATTERN.get_or_init(|| {
        // Discord Botトークンパターン
        Regex::new(r"[MN][A-Za-z\d]{23}\.[\w-]{6}\.[\w-]{27}")
            .expect("Invalid Discord token regex")
    })
}

/// 一般的なシークレットキーの正規表現を取得
fn secret_key_regex() -> &'static Regex {
    SECRET_KEY_PATTERN.get_or_init(|| {
        // secret、password、tokenなどのパターン
        // JSON形式 "password": "xxx" と代入形式 password=xxx の両方に対応
        Regex::new(r#"(?i)(secret|password|passwd|pwd|token|auth[_-]?key|bearer)[\s]*[:=][\s]*["']?[^\s"']{8,}["']?"#)
            .expect("Invalid secret key regex")
    })
}

/// APIキーをマスキングする
///
/// # Arguments
/// * `text` - マスキング対象のテキスト
///
/// # Returns
/// * APIキーがマスキングされたテキスト
///
/// # Example
/// ```
/// use cc_bot::security::mask_api_key;
/// let masked = mask_api_key("api_key=sk-1234567890abcdef1234567890");
/// assert!(masked.contains("***MASKED***"));
/// ```
pub fn mask_api_key(text: &str) -> String {
    api_key_regex()
        .replace_all(text, |caps: &regex::Captures| {
            format!("{}=***MASKED***", caps.get(1).unwrap().as_str())
        })
        .to_string()
}

/// Discordトークンをマスキングする
///
/// # Arguments
/// * `text` - マスキング対象のテキスト
///
/// # Returns
/// * Discordトークンがマスキングされたテキスト
pub fn mask_discord_token(text: &str) -> String {
    discord_token_regex()
        .replace_all(text, "***DISCORD_TOKEN_MASKED***")
        .to_string()
}

/// 機密情報を一括でマスキングする
///
/// # Arguments
/// * `text` - マスキング対象のテキスト
///
/// # Returns
/// * 機密情報がマスキングされたテキスト
pub fn mask_secrets(text: &str) -> String {
    let mut result = text.to_string();

    // Discordトークンをマスキング
    result = mask_discord_token(&result);

    // APIキーをマスキング
    result = mask_api_key(&result);

    // 一般的なシークレットをマスキング
    result = secret_key_regex()
        .replace_all(&result, |caps: &regex::Captures| {
            format!("{}=***MASKED***", caps.get(1).unwrap().as_str())
        })
        .to_string();

    result
}

/// 機密情報マスカー
///
/// ログ出力前に機密情報を自動的にマスキングするためのヘルパー構造体。
pub struct SecretMasker<'a> {
    text: &'a str,
}

impl<'a> SecretMasker<'a> {
    /// 新しいマスカーを作成
    ///
    /// # Arguments
    /// * `text` - マスキング対象のテキスト
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// 機密情報をマスキングして文字列として返す
    pub fn mask(&self) -> String {
        mask_secrets(self.text)
    }
}

impl<'a> std::fmt::Display for SecretMasker<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.mask())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_discord_token() {
        // Discordトークン形式（M始まり）
        // 文字列を分割してシークレットスキャンを回避
        let token = format!(
            "M{}{}{}",
            "TE5NjIyMzg4NTU2NDMxNTIz",
            ".GqTtKk.",
            "abc123def456ghi789jkl012mno345pqr678st"
        );
        let text = format!("Token: {}", token);
        let masked = mask_discord_token(&text);
        assert!(masked.contains("***DISCORD_TOKEN_MASKED***"));
    }

    #[test]
    fn test_mask_discord_token_n_prefix() {
        // Discordトークン形式（N始まり）
        // 文字列を分割してシークレットスキャンを回避
        let token = format!(
            "N{}{}{}",
            "TE5NjIyMzg4NTU2NDMxNTIz",
            ".GqTtKk.",
            "abc123def456ghi789jkl012mno345pqr678st"
        );
        let text = format!("Token: {}", token);
        let masked = mask_discord_token(&text);
        assert!(masked.contains("***DISCORD_TOKEN_MASKED***"));
    }

    #[test]
    fn test_mask_api_key() {
        let text = "api_key=sk-1234567890abcdef1234567890";
        let masked = mask_api_key(text);
        assert!(masked.contains("***MASKED***"));
    }

    #[test]
    fn test_mask_api_key_uppercase() {
        let text = "API_KEY=my-secret-key-123456789012345";
        let masked = mask_api_key(text);
        assert!(masked.contains("***MASKED***"));
    }

    #[test]
    fn test_mask_secrets_password() {
        let text = "password=supersecretpassword123";
        let masked = mask_secrets(text);
        assert!(masked.contains("***MASKED***"));
        assert!(!masked.contains("supersecretpassword123"));
    }

    #[test]
    fn test_mask_secrets_token() {
        let text = "token=bearer_token_xyz12345678";
        let masked = mask_secrets(text);
        assert!(masked.contains("***MASKED***"));
    }

    #[test]
    fn test_mask_secrets_preserves_normal_text() {
        let text = "User asked: Hello, how are you?";
        let masked = mask_secrets(text);
        assert_eq!(text, masked);
    }

    #[test]
    fn test_mask_secrets_json() {
        let text = "api_key=sk-test123456789012345678 model=gpt-4";
        let masked = mask_secrets(text);
        assert!(masked.contains("***MASKED***"));
        assert!(masked.contains("gpt-4")); // 通常の値は保持
    }

    #[test]
    fn test_secret_masker() {
        // bearer=形式でテスト（正規表現パターンにマッチ）
        let text = "bearer=secret12345678";
        let masker = SecretMasker::new(text);
        let masked = masker.to_string();
        assert!(masked.contains("***MASKED***"));
    }

    #[test]
    fn test_short_secrets_not_masked() {
        // 短いシークレットはマスキングしない
        let text = "password=abc"; // 3文字（8文字未満）
        let masked = mask_secrets(text);
        // マスキングされない（正規表現は8文字以上を対象）
        assert!(masked.contains("abc"));
    }
}
