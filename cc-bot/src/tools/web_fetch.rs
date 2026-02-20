use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value as JsonValue};
use std::time::Duration;
use tracing::{debug, warn};

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

    /// HTMLをMarkdownに変換（シンプルな実装）
    fn html_to_markdown(html: &str) -> String {
        let mut md = html.to_string();

        // スクリプトとスタイルを削除
        let script_re = Regex::new(r"<script[^>]*>.*?</script>").unwrap();
        let style_re = Regex::new(r"<style[^>]*>.*?</style>").unwrap();
        md = script_re.replace_all(&md, "").to_string();
        md = style_re.replace_all(&md, "").to_string();

        // ヘッダー
        md = Regex::new(r"<h1[^>]*>(.*?)</h1>")
            .unwrap()
            .replace_all(&md, "# $1\n")
            .to_string();
        md = Regex::new(r"<h2[^>]*>(.*?)</h2>")
            .unwrap()
            .replace_all(&md, "## $1\n")
            .to_string();
        md = Regex::new(r"<h3[^>]*>(.*?)</h3>")
            .unwrap()
            .replace_all(&md, "### $1\n")
            .to_string();
        md = Regex::new(r"<h4[^>]*>(.*?)</h4>")
            .unwrap()
            .replace_all(&md, "#### $1\n")
            .to_string();
        md = Regex::new(r"<h5[^>]*>(.*?)</h5>")
            .unwrap()
            .replace_all(&md, "##### $1\n")
            .to_string();
        md = Regex::new(r"<h6[^>]*>(.*?)</h6>")
            .unwrap()
            .replace_all(&md, "###### $1\n")
            .to_string();

        // 段落
        md = Regex::new(r"<p[^>]*>(.*?)</p>")
            .unwrap()
            .replace_all(&md, "$1\n\n")
            .to_string();

        // リンク
        md = Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#)
            .unwrap()
            .replace_all(&md, "[$2]($1)")
            .to_string();

        // 太字・斜体
        md = Regex::new(r"<strong[^>]*>(.*?)</strong>")
            .unwrap()
            .replace_all(&md, "**$1**")
            .to_string();
        md = Regex::new(r"<b[^>]*>(.*?)</b>")
            .unwrap()
            .replace_all(&md, "**$1**")
            .to_string();
        md = Regex::new(r"<em[^>]*>(.*?)</em>")
            .unwrap()
            .replace_all(&md, "*$1*")
            .to_string();
        md = Regex::new(r"<i[^>]*>(.*?)</i>")
            .unwrap()
            .replace_all(&md, "*$1*")
            .to_string();

        // コード
        md = Regex::new(r"<code[^>]*>(.*?)</code>")
            .unwrap()
            .replace_all(&md, "`$1`")
            .to_string();
        md = Regex::new(r"<pre[^>]*>(.*?)</pre>")
            .unwrap()
            .replace_all(&md, "```\n$1\n```")
            .to_string();

        // リスト
        md = Regex::new(r"<li[^>]*>(.*?)</li>")
            .unwrap()
            .replace_all(&md, "- $1\n")
            .to_string();
        md = Regex::new(r"</?[ou]l[^>]*>")
            .unwrap()
            .replace_all(&md, "\n")
            .to_string();

        // 改行
        md = Regex::new(r"<br\s*/?>")
            .unwrap()
            .replace_all(&md, "\n")
            .to_string();

        // 残りのHTMLタグを削除
        md = Regex::new(r"<[^>]+>")
            .unwrap()
            .replace_all(&md, "")
            .to_string();

        // HTMLエンティティをデコード
        md = md.replace("&nbsp;", " ");
        md = md.replace("&amp;", "&");
        md = md.replace("&lt;", "<");
        md = md.replace("&gt;", ">");
        md = md.replace("&quot;", "\"");
        md = md.replace("&#39;", "'");
        md = Regex::new(r"&#(\d+);")
            .unwrap()
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
        md = Regex::new(r"\n{3,}")
            .unwrap()
            .replace_all(&md, "\n\n")
            .to_string();
        md = Regex::new(r" {2,}")
            .unwrap()
            .replace_all(&md, " ")
            .to_string();

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
                    Self::html_to_markdown(&body)
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
}
