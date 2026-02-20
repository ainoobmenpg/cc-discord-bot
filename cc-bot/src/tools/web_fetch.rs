use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value as JsonValue};
use std::time::Duration;
use tracing::{debug, info, warn};

/// キャッシュされた正規表現パターン
struct RegexPatterns {
    script: Regex,
    style: Regex,
    h1: Regex,
    h2: Regex,
    h3: Regex,
    h4: Regex,
    h5: Regex,
    h6: Regex,
    p: Regex,
    a: Regex,
    strong: Regex,
    b: Regex,
    em: Regex,
    i: Regex,
    code: Regex,
    pre: Regex,
    li: Regex,
    list: Regex,
    br: Regex,
    html_tag: Regex,
    numeric_entity: Regex,
    multi_newline: Regex,
    multi_space: Regex,
}

impl RegexPatterns {
    fn new() -> Self {
        Self {
            script: Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap(),
            style: Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap(),
            h1: Regex::new(r"(?i)<h1[^>]*>(.*?)</h1>").unwrap(),
            h2: Regex::new(r"(?i)<h2[^>]*>(.*?)</h2>").unwrap(),
            h3: Regex::new(r"(?i)<h3[^>]*>(.*?)</h3>").unwrap(),
            h4: Regex::new(r"(?i)<h4[^>]*>(.*?)</h4>").unwrap(),
            h5: Regex::new(r"(?i)<h5[^>]*>(.*?)</h5>").unwrap(),
            h6: Regex::new(r"(?i)<h6[^>]*>(.*?)</h6>").unwrap(),
            p: Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap(),
            a: Regex::new(r#"(?i)<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap(),
            strong: Regex::new(r"(?i)<strong[^>]*>(.*?)</strong>").unwrap(),
            b: Regex::new(r"(?i)<b[^>]*>(.*?)</b>").unwrap(),
            em: Regex::new(r"(?i)<em[^>]*>(.*?)</em>").unwrap(),
            i: Regex::new(r"(?i)<i[^>]*>(.*?)</i>").unwrap(),
            code: Regex::new(r"(?i)<code[^>]*>(.*?)</code>").unwrap(),
            pre: Regex::new(r"(?is)<pre[^>]*>(.*?)</pre>").unwrap(),
            li: Regex::new(r"(?is)<li[^>]*>(.*?)</li>").unwrap(),
            list: Regex::new(r"(?i)</?[ou]l[^>]*>").unwrap(),
            br: Regex::new(r"(?i)<br\s*/?>").unwrap(),
            html_tag: Regex::new(r"<[^>]+>").unwrap(),
            numeric_entity: Regex::new(r"&#(\d+);").unwrap(),
            multi_newline: Regex::new(r"\n{3,}").unwrap(),
            multi_space: Regex::new(r" {2,}").unwrap(),
        }
    }
}

/// グローバルにキャッシュされた正規表現
static REGEX: Lazy<RegexPatterns> = Lazy::new(RegexPatterns::new);

/// Web取得ツール（HTTP取得 + Markdown変換）
pub struct WebFetchTool {
    client: Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("cc-discord-bot/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// URLからコンテンツを取得
    async fn fetch(&self, url: &str) -> Result<(String, String), String> {
        debug!("Fetching URL: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        Ok((body, content_type))
    }

    /// HTMLをMarkdownに変換（キャッシュされたRegexを使用）
    fn html_to_markdown(html: &str) -> String {
        let mut md = html.to_string();

        // スクリプトとスタイルを削除
        md = REGEX.script.replace_all(&md, "").to_string();
        md = REGEX.style.replace_all(&md, "").to_string();

        // ヘッダー
        md = REGEX.h1.replace_all(&md, "# $1\n").to_string();
        md = REGEX.h2.replace_all(&md, "## $1\n").to_string();
        md = REGEX.h3.replace_all(&md, "### $1\n").to_string();
        md = REGEX.h4.replace_all(&md, "#### $1\n").to_string();
        md = REGEX.h5.replace_all(&md, "##### $1\n").to_string();
        md = REGEX.h6.replace_all(&md, "###### $1\n").to_string();

        // 段落
        md = REGEX.p.replace_all(&md, "$1\n\n").to_string();

        // リンク
        md = REGEX.a.replace_all(&md, "[$2]($1)").to_string();

        // 太字・斜体
        md = REGEX.strong.replace_all(&md, "**$1**").to_string();
        md = REGEX.b.replace_all(&md, "**$1**").to_string();
        md = REGEX.em.replace_all(&md, "*$1*").to_string();
        md = REGEX.i.replace_all(&md, "*$1*").to_string();

        // コード
        md = REGEX.code.replace_all(&md, "`$1`").to_string();
        md = REGEX.pre.replace_all(&md, "```\n$1\n```").to_string();

        // リスト
        md = REGEX.li.replace_all(&md, "- $1\n").to_string();
        md = REGEX.list.replace_all(&md, "\n").to_string();

        // 改行
        md = REGEX.br.replace_all(&md, "\n").to_string();

        // 残りのHTMLタグを削除
        md = REGEX.html_tag.replace_all(&md, "").to_string();

        // HTMLエンティティをデコード
        md = md.replace("&nbsp;", " ");
        md = md.replace("&amp;", "&");
        md = md.replace("&lt;", "<");
        md = md.replace("&gt;", ">");
        md = md.replace("&quot;", "\"");
        md = md.replace("&#39;", "'");
        md = REGEX.numeric_entity
            .replace_all(&md, |caps: &regex::Captures| {
                caps[1]
                    .parse::<u32>()
                    .ok()
                    .and_then(|n| char::from_u32(n))
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            })
            .to_string();

        // 余分な空白を削除
        md = REGEX.multi_newline.replace_all(&md, "\n\n").to_string();
        md = REGEX.multi_space.replace_all(&md, " ").to_string();

        // 前後の空白を削除
        md.trim().to_string()
    }

    /// コンテンツを切り詰め（文字単位）
    fn truncate(content: &str, max_chars: usize) -> String {
        let char_count = content.chars().count();
        if char_count <= max_chars {
            content.to_string()
        } else {
            let truncated: String = content.chars().take(max_chars).collect();
            format!("{}...\n\n[Content truncated - {} chars total]", truncated, char_count)
        }
    }

    /// readabilityを使用して本文を抽出（フォールバック付き）
    fn extract_readable_content(html: &str, url: &str) -> Option<ExtractedContent> {
        // 読み取り可能かチェック
        if !legible::is_probably_readerable(html, None) {
            debug!("Content not readerable, falling back to regex");
            return None;
        }

        // 本文抽出
        match legible::parse(html, Some(url), None) {
            Ok(article) => {
                info!("Readability extraction successful: {:?}", article.title);

                // タイトルとコンテンツを取得
                let title = article.title;
                let content_html = article.content;

                // コンテンツHTMLをMarkdownに変換
                let content_md = Self::html_to_markdown(&content_html);

                Some(ExtractedContent {
                    title,
                    content: content_md,
                })
            }
            Err(e) => {
                warn!("Readability extraction failed: {:?}", e);
                None
            }
        }
    }
}

/// 抽出されたコンテンツ
struct ExtractedContent {
    title: String,
    content: String,
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL and convert it to Markdown format. Returns the content as clean, readable text."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "max_chars": {
                    "type": "integer",
                    "description": "Maximum characters to return (default: 10000, max: 50000)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: JsonValue, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = params["url"].as_str().ok_or_else(|| {
            ToolError::InvalidParams("Missing 'url' parameter".to_string())
        })?;

        let max_chars = params["max_chars"].as_u64().unwrap_or(10000).min(50000) as usize;

        // URLの基本的なバリデーション
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidParams(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        debug!("Web fetch: {} (max_chars: {})", url, max_chars);

        match self.fetch(url).await {
            Ok((body, content_type)) => {
                let markdown = if content_type.contains("text/html") {
                    // readabilityで本文抽出を試みる
                    if let Some(extracted) = Self::extract_readable_content(&body, url) {
                        // タイトル + 本文の形式で出力
                        let mut result = String::new();
                        if !extracted.title.is_empty() {
                            result.push_str(&format!("# {}\n\n", extracted.title));
                        }
                        result.push_str(&format!("> URL: {}\n\n", url));
                        result.push_str(&extracted.content);
                        result
                    } else {
                        // フォールバック: 従来の正規表現処理
                        Self::html_to_markdown(&body)
                    }
                } else if content_type.contains("application/json") {
                    // JSONはそのままコードブロックで表示
                    format!("```json\n{}\n```", body)
                } else if content_type.contains("text/plain") || content_type.contains("text/markdown") {
                    body
                } else {
                    // その他はHTMLとして処理を試みる
                    Self::html_to_markdown(&body)
                };

                let truncated = Self::truncate(&markdown, max_chars);
                Ok(ToolResult::success(truncated))
            }
            Err(e) => {
                warn!("Web fetch failed: {}", e);
                Err(ToolError::ExecutionFailed(e))
            }
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> ToolContext {
        ToolContext::new(123, "test_user".to_string(), 456, "output".to_string())
    }

    #[test]
    fn test_tool_definition() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "web_fetch");
    }

    #[test]
    fn test_html_to_markdown_headers() {
        let html = "<h1>Title</h1><h2>Subtitle</h2>";
        let md = WebFetchTool::html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
    }

    #[test]
    fn test_html_to_markdown_links() {
        let html = r#"<a href="https://example.com">Example</a>"#;
        let md = WebFetchTool::html_to_markdown(html);
        assert!(md.contains("[Example](https://example.com)"));
    }

    #[test]
    fn test_html_to_markdown_formatting() {
        let html = "<strong>bold</strong> and <em>italic</em>";
        let md = WebFetchTool::html_to_markdown(html);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn test_html_to_markdown_code() {
        let html = "<code>inline</code> and <pre>block</pre>";
        let md = WebFetchTool::html_to_markdown(html);
        assert!(md.contains("`inline`"));
        assert!(md.contains("block"));
    }

    #[test]
    fn test_html_to_markdown_entities() {
        let html = "&lt;tag&gt; &amp; &quot;quote&quot;";
        let md = WebFetchTool::html_to_markdown(html);
        assert!(md.contains("<tag>"));
        assert!(md.contains("&"));
        assert!(md.contains("\"quote\""));
    }

    #[test]
    fn test_truncate_short() {
        let content = "Short content";
        let result = WebFetchTool::truncate(content, 100);
        assert_eq!(result, content);
    }

    #[test]
    fn test_truncate_long() {
        let content = "a".repeat(200);
        let result = WebFetchTool::truncate(&content, 100);
        assert!(result.contains("..."));
        assert!(result.contains("200 chars total"));
        // 最初の100文字が含まれていることを確認
        assert!(result.starts_with(&"a".repeat(100)));
    }

    #[tokio::test]
    async fn test_web_fetch_missing_url() {
        let tool = WebFetchTool::new();
        let ctx = create_test_context();

        let result = tool.execute(json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool::new();
        let ctx = create_test_context();

        let result = tool
            .execute(json!({"url": "invalid-url"}), &ctx)
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ToolError::InvalidParams(_))));
    }

    #[test]
    fn test_regex_patterns_case_insensitive() {
        // 大文字小文字を区別しないことを確認
        let html = "<H1>Title</H1><STRONG>bold</STRONG>";
        let md = WebFetchTool::html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("**bold**"));
    }

    #[test]
    fn test_extract_readable_content_fallback() {
        // シンプルなHTML（readabilityが適用されない可能性が高い）
        let html = "<html><body><p>Simple text</p></body></html>";

        // フォールバックが動作することを確認（panicしない）
        let result = WebFetchTool::extract_readable_content(html, "https://example.com");
        // 結果はSomeでもNoneでもOK（フォールバック先のregex処理があるため）
        if let Some(extracted) = result {
            assert!(!extracted.content.is_empty() || !extracted.title.is_empty());
        }
    }

    #[test]
    fn test_extract_readable_content_complex() {
        // より複雑な記事構造（readabilityが適用されやすい）
        let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <title>Test Blog Post - My Blog</title>
    <meta property="og:title" content="Test Blog Post">
</head>
<body>
    <header>
        <nav><a href="/">Home</a></nav>
    </header>
    <main>
        <article>
            <h1>Test Blog Post</h1>
            <p>This is the first paragraph of a blog post with substantial content that should make the page readerable.</p>
            <p>Here is another paragraph with more information and details about the topic at hand.</p>
            <p>And yet another paragraph to ensure there is enough content for the readability algorithm to work properly.</p>
            <h2>Section Title</h2>
            <p>More content in this section with additional paragraphs and information.</p>
        </article>
    </main>
    <footer>
        <p>Copyright 2024</p>
    </footer>
</body>
</html>
"#;

        let result = WebFetchTool::extract_readable_content(html, "https://example.com/blog/test-post");
        // 結果はSomeでもNoneでもOK（readabilityの判定による）
        // 重要なのは関数が正常に実行されること
        if let Some(extracted) = result {
            // タイトルまたはコンテンツが抽出されていることを確認
            let has_content = !extracted.title.is_empty() || !extracted.content.is_empty();
            assert!(has_content, "Extracted content should have title or content");
        }
    }
}
