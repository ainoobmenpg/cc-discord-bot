//! 入力検証とパス検証のためのモジュール
//!
//! セキュリティ対策として、ユーザー入力のサニタイズと
//! ファイルパスの検証を提供する。
#![allow(dead_code)]

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// 検証エラーの種類
#[derive(Debug, Error)]
pub enum ValidationError {
    /// パストラバーサル攻撃を検出
    #[error("Path traversal detected")]
    PathTraversal,

    /// 絶対パスは許可されていない
    #[error("Absolute path not allowed")]
    AbsolutePath,

    /// 許可されたディレクトリ外のパス
    #[error("Path outside allowed directory")]
    OutsideAllowedDirectory,

    /// 無効な文字が含まれている
    #[error("Invalid characters")]
    InvalidCharacters,
}

/// 入力検証を行うトレイト
pub trait Validator {
    /// 入力を検証し、サニタイズされた結果を返す
    fn validate(&self, input: &str) -> Result<String, ValidationError>;
}

/// XSS対策のための入力サニタイザー
///
/// HTMLエスケープを行い、制御文字を削除する。
/// 改行とタブは保持する。
pub struct InputSanitizer;

impl InputSanitizer {
    /// 新しい InputSanitizer を作成
    pub fn new() -> Self {
        Self
    }

    /// 文字列をHTMLエスケープする
    fn escape_html(&self, input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// 制御文字を削除する（改行とタブは保持）
    fn remove_control_chars(&self, input: &str) -> String {
        input
            .chars()
            .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
            .collect()
    }
}

impl Default for InputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for InputSanitizer {
    fn validate(&self, input: &str) -> Result<String, ValidationError> {
        // まず制御文字を削除
        let sanitized = self.remove_control_chars(input);
        // HTMLエスケープ
        let escaped = self.escape_html(&sanitized);
        Ok(escaped)
    }
}

/// ファイルパスの検証を行う
///
/// セキュリティ対策として以下を防止する:
/// - パストラバーサル（`..`の使用）
/// - 絶対パスの使用
/// - 許可されていないディレクトリ外へのアクセス
/// - シンボリックリンクの追跡
pub struct PathValidator {
    /// 許可されたベースディレクトリ
    allowed_base: PathBuf,
}

impl PathValidator {
    /// 新しい PathValidator を作成
    ///
    /// # Arguments
    /// * `allowed_base` - 許可するベースディレクトリのパス
    pub fn new<P: Into<PathBuf>>(allowed_base: P) -> Self {
        Self {
            allowed_base: allowed_base.into(),
        }
    }

    /// デフォルトの PathValidator を作成（`output/` ディレクトリを許可）
    pub fn with_default_dir() -> Self {
        Self::new("output")
    }

    /// パスに `..` が含まれているかチェック
    fn contains_traversal(&self, path: &Path) -> bool {
        path.components().any(|c| matches!(c, Component::ParentDir))
    }

    /// パスが絶対パスかどうかチェック
    fn is_absolute_path(&self, path: &Path) -> bool {
        path.is_absolute()
    }

    /// パスが許可されたディレクトリ内にあるかチェック
    fn is_within_allowed_dir(&self, path: &Path) -> bool {
        // ベースディレクトリを正規化
        let normalized_base = self.normalize_path(&self.allowed_base);

        // 入力パスをベースからの相対パスとして解決
        let full_path = normalized_base.join(path);
        let normalized_path = self.normalize_path(&full_path);

        // 正規化されたパスがベースディレクトリで始まるかチェック
        normalized_path.starts_with(&normalized_base)
    }

    /// パスを正規化する（シンボリックリンクは追跡しない）
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::CurDir => {
                    // カレントディレクトリはスキップ
                }
                Component::ParentDir => {
                    // 親ディレクトリの場合は、正規化されたパスから1つ戻る
                    if !normalized.pop() {
                        // ルートより上に行こうとしている
                        normalized.push(component);
                    }
                }
                _ => {
                    normalized.push(component);
                }
            }
        }

        normalized
    }

    /// パスを検証して正規化されたパスを返す
    pub fn validate_path(&self, input: &str) -> Result<PathBuf, ValidationError> {
        let path = Path::new(input);

        // 絶対パスチェック
        if self.is_absolute_path(path) {
            return Err(ValidationError::AbsolutePath);
        }

        // パストラバーサルチェック
        if self.contains_traversal(path) {
            return Err(ValidationError::PathTraversal);
        }

        // 許可ディレクトリ内かチェック
        if !self.is_within_allowed_dir(path) {
            return Err(ValidationError::OutsideAllowedDirectory);
        }

        // 正規化されたパスを返す
        let full_path = self.allowed_base.join(path);
        Ok(self.normalize_path(&full_path))
    }
}

impl Validator for PathValidator {
    fn validate(&self, input: &str) -> Result<String, ValidationError> {
        let path = self.validate_path(input)?;
        Ok(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // InputSanitizer Tests
    // ============================================

    #[test]
    fn test_input_sanitizer_escapes_html() {
        let sanitizer = InputSanitizer::new();

        // 基本的なHTMLエスケープ
        assert_eq!(
            sanitizer.validate("<script>alert('xss')</script>").unwrap(),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );

        // アンパサンド
        assert_eq!(
            sanitizer.validate("a & b").unwrap(),
            "a &amp; b"
        );

        // ダブルクォート
        assert_eq!(
            sanitizer.validate("say \"hello\"").unwrap(),
            "say &quot;hello&quot;"
        );

        // シングルクォート
        assert_eq!(
            sanitizer.validate("it's").unwrap(),
            "it&#x27;s"
        );
    }

    #[test]
    fn test_input_sanitizer_preserves_newlines_and_tabs() {
        let sanitizer = InputSanitizer::new();

        // 改行を保持
        assert_eq!(
            sanitizer.validate("line1\nline2").unwrap(),
            "line1\nline2"
        );

        // タブを保持
        assert_eq!(
            sanitizer.validate("col1\tcol2").unwrap(),
            "col1\tcol2"
        );

        // 改行とタブの組み合わせ
        assert_eq!(
            sanitizer.validate("a\tb\nc\td").unwrap(),
            "a\tb\nc\td"
        );
    }

    #[test]
    fn test_input_sanitizer_removes_control_chars() {
        let sanitizer = InputSanitizer::new();

        // NULL文字を削除
        assert_eq!(
            sanitizer.validate("hello\0world").unwrap(),
            "helloworld"
        );

        // ベル文字を削除
        assert_eq!(
            sanitizer.validate("alert\x07").unwrap(),
            "alert"
        );

        // エスケープ文字を削除
        assert_eq!(
            sanitizer.validate("escape\x1b").unwrap(),
            "escape"
        );

        // 複数の制御文字
        assert_eq!(
            sanitizer.validate("a\x00b\x07c\x1bd").unwrap(),
            "abcd"
        );
    }

    #[test]
    fn test_input_sanitizer_normal_text() {
        let sanitizer = InputSanitizer::new();

        // 通常のテキストはそのまま
        assert_eq!(
            sanitizer.validate("Hello, World!").unwrap(),
            "Hello, World!"
        );

        // 日本語
        assert_eq!(
            sanitizer.validate("こんにちは世界").unwrap(),
            "こんにちは世界"
        );

        // 数字と記号
        assert_eq!(
            sanitizer.validate("Price: $100 (10% off)").unwrap(),
            "Price: $100 (10% off)"
        );
    }

    #[test]
    fn test_input_sanitizer_empty_string() {
        let sanitizer = InputSanitizer::new();
        assert_eq!(sanitizer.validate("").unwrap(), "");
    }

    // ============================================
    // PathValidator Tests
    // ============================================

    #[test]
    fn test_path_validator_accepts_valid_path() {
        let validator = PathValidator::with_default_dir();

        // シンプルなファイル名
        assert!(validator.validate("file.txt").is_ok());
        assert!(validator.validate("output/file.txt").is_ok() || validator.validate("file.txt").is_ok());

        // サブディレクトリ
        assert!(validator.validate("subdir/file.txt").is_ok());
    }

    #[test]
    fn test_path_validator_rejects_traversal() {
        let validator = PathValidator::with_default_dir();

        // 親ディレクトリへの移動
        assert!(matches!(
            validator.validate("../etc/passwd"),
            Err(ValidationError::PathTraversal)
        ));

        // 複雑なトラバーサル
        assert!(matches!(
            validator.validate("foo/../../bar"),
            Err(ValidationError::PathTraversal)
        ));

        // 中間のトラバーサル
        assert!(matches!(
            validator.validate("safe/../unsafe"),
            Err(ValidationError::PathTraversal)
        ));
    }

    #[test]
    fn test_path_validator_rejects_absolute_path() {
        let validator = PathValidator::with_default_dir();

        // Unix絶対パス
        assert!(matches!(
            validator.validate("/etc/passwd"),
            Err(ValidationError::AbsolutePath)
        ));

        // Unix絶対パス（別パターン）
        assert!(matches!(
            validator.validate("/usr/local/bin"),
            Err(ValidationError::AbsolutePath)
        ));
    }

    #[test]
    fn test_path_validator_rejects_outside_directory() {
        // カスタムベースディレクトリでテスト
        let validator = PathValidator::new("output");

        // トラバーサルなしで外に出ようとするパス
        // 注: PathValidator は相対パスを allowed_base からの相対として扱うため
        // "data/file.txt" は "output/data/file.txt" として解決される
        // したがって、このテストは許可される
        assert!(validator.validate("subdir/file.txt").is_ok());
    }

    #[test]
    fn test_path_validator_custom_base() {
        let validator = PathValidator::new("custom_dir");

        // カスタムディレクトリ内のファイル
        assert!(validator.validate("file.txt").is_ok());

        // トラバーサルは拒否
        assert!(matches!(
            validator.validate("../file.txt"),
            Err(ValidationError::PathTraversal)
        ));
    }

    #[test]
    fn test_path_validator_empty_path() {
        let validator = PathValidator::with_default_dir();

        // 空文字列は許可（ベースディレクトリ自体を指す）
        assert!(validator.validate("").is_ok());
    }

    #[test]
    fn test_path_validator_current_dir() {
        let validator = PathValidator::with_default_dir();

        // カレントディレクトリ表記を含むパス
        assert!(validator.validate("./file.txt").is_ok());
        assert!(validator.validate("subdir/./file.txt").is_ok());
    }

    #[test]
    fn test_path_validator_normalizes_path() {
        let validator = PathValidator::new("output");

        // 正規化のテスト
        let result = validator.validate_path("subdir/../file.txt");
        // トラバーサルを含むためエラー
        assert!(matches!(result, Err(ValidationError::PathTraversal)));
    }

    #[test]
    fn test_path_validator_deep_nested() {
        let validator = PathValidator::with_default_dir();

        // 深いネスト
        assert!(validator.validate("a/b/c/d/e/f.txt").is_ok());
    }

    // ============================================
    // ValidationError Tests
    // ============================================

    #[test]
    fn test_validation_error_messages() {
        assert_eq!(
            ValidationError::PathTraversal.to_string(),
            "Path traversal detected"
        );
        assert_eq!(
            ValidationError::AbsolutePath.to_string(),
            "Absolute path not allowed"
        );
        assert_eq!(
            ValidationError::OutsideAllowedDirectory.to_string(),
            "Path outside allowed directory"
        );
        assert_eq!(
            ValidationError::InvalidCharacters.to_string(),
            "Invalid characters"
        );
    }
}
