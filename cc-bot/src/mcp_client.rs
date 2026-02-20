//! MCP (Model Context Protocol) クライアント実装
//!
//! MCPサーバーとの通信を管理し、動的ツールロードを提供します。
//! 接続プールによるプロセス再利用でパフォーマンスを最適化します。

use anyhow::Result;
use rmcp::model::{CallToolRequestParams, Tool};
use rmcp::service::{RoleClient, RunningService, ServiceExt};
use rmcp::transport::{TokioChildProcess, child_process::ConfigureCommandExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// MCPサーバー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// サーバー名
    pub name: String,
    /// 起動コマンド
    pub command: String,
    /// コマンド引数
    #[serde(default)]
    pub args: Vec<String>,
    /// 環境変数
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 有効/無効
    #[serde(default)]
    pub enabled: bool,
    /// 説明
    #[serde(default)]
    pub description: String,
}

/// MCP設定全体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPConfig {
    /// サーバーリスト
    pub servers: Vec<MCPServerConfig>,
    /// グローバル設定
    #[serde(default)]
    pub settings: MCPSettings,
}

/// MCPグローバル設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSettings {
    /// 接続タイムアウト（秒）
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_seconds: u64,
    /// ツール実行タイムアウト（秒）
    #[serde(default = "default_tool_timeout")]
    pub tool_execution_timeout_seconds: u64,
    /// 最大同時ツール数
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_tools: usize,
}

fn default_connection_timeout() -> u64 { 30 }
fn default_tool_timeout() -> u64 { 60 }
fn default_max_concurrent() -> usize { 5 }

impl Default for MCPSettings {
    fn default() -> Self {
        Self {
            connection_timeout_seconds: default_connection_timeout(),
            tool_execution_timeout_seconds: default_tool_timeout(),
            max_concurrent_tools: default_max_concurrent(),
        }
    }
}

/// MCPツール定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolDefinition {
    /// ツール名
    pub name: String,
    /// ツールの説明
    pub description: String,
    /// 入力スキーマ
    pub input_schema: serde_json::Value,
    /// 提供元サーバー名
    pub server_name: String,
}

/// サーバー接続情報
struct ServerConnection {
    /// サービス
    service: RunningService<RoleClient, ()>,
    /// 最終使用時刻
    last_used: Instant,
}

impl ServerConnection {
    /// 新しい接続を作成
    async fn new(server: &MCPServerConfig) -> Result<Self> {
        // 環境変数を展開
        let expanded_env: HashMap<String, String> = server.env.iter()
            .map(|(k, v)| {
                let expanded = if v.starts_with("${") && v.ends_with("}") {
                    let var_name = &v[2..v.len()-1];
                    std::env::var(var_name).unwrap_or_else(|_| v.clone())
                } else {
                    v.clone()
                };
                (k.clone(), expanded)
            })
            .collect();

        // 子プロセスとしてサーバーを起動
        let transport = TokioChildProcess::new(
            Command::new(&server.command).configure(|cmd| {
                for arg in &server.args {
                    cmd.arg(arg);
                }
                for (key, value) in &expanded_env {
                    cmd.env(key, value);
                }
            })
        )?;

        // サービスを開始
        let service = ().serve(transport).await?;
        debug!("Connected to MCP server: {}", server.name);

        Ok(Self {
            service,
            last_used: Instant::now(),
        })
    }
}

/// MCP接続プール
///
/// MCPサーバーへの永続的な接続を管理し、プロセス再利用による
/// パフォーマンス向上を実現します。
pub struct MCPConnectionPool {
    /// 接続マップ (サーバー名 -> 接続)
    connections: RwLock<HashMap<String, ServerConnection>>,
    /// アイドルタイムアウト（秒）
    idle_timeout_seconds: u64,
}

impl MCPConnectionPool {
    /// 新しい接続プールを作成
    pub fn new(idle_timeout_seconds: u64) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            idle_timeout_seconds,
        }
    }

    /// 接続を使用してツール一覧を取得
    pub async fn list_tools(&self, server: &MCPServerConfig) -> Result<Vec<Tool>> {
        let server_name = &server.name;

        // 既存の接続を確認
        {
            let mut connections = self.connections.write().await;
            if let Some(conn) = connections.get_mut(server_name) {
                let elapsed = conn.last_used.elapsed().as_secs();
                if elapsed < self.idle_timeout_seconds {
                    debug!("Reusing existing connection to {} (idle: {}s)", server_name, elapsed);
                    conn.last_used = Instant::now();
                    return self.do_list_tools(&conn.service).await;
                } else {
                    info!("Connection to {} timed out (idle: {}s), reconnecting", server_name, elapsed);
                    connections.remove(server_name);
                }
            }
        }

        // 新規接続を作成
        debug!("Creating new connection to {}", server_name);
        let conn = ServerConnection::new(server).await?;
        let tools = self.do_list_tools(&conn.service).await?;

        // 接続をプールに保存
        let mut connections = self.connections.write().await;
        connections.insert(server_name.clone(), conn);

        Ok(tools)
    }

    /// 接続を使用してツールを実行
    pub async fn call_tool(
        &self,
        server: &MCPServerConfig,
        tool_name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<rmcp::model::CallToolResult> {
        let server_name = &server.name;

        // 既存の接続を確認
        {
            let mut connections = self.connections.write().await;
            if let Some(conn) = connections.get_mut(server_name) {
                let elapsed = conn.last_used.elapsed().as_secs();
                if elapsed < self.idle_timeout_seconds {
                    debug!("Reusing existing connection to {} (idle: {}s)", server_name, elapsed);
                    conn.last_used = Instant::now();
                    return self.do_call_tool(&conn.service, tool_name, arguments).await;
                } else {
                    info!("Connection to {} timed out (idle: {}s), reconnecting", server_name, elapsed);
                    connections.remove(server_name);
                }
            }
        }

        // 新規接続を作成
        debug!("Creating new connection to {}", server_name);
        let conn = ServerConnection::new(server).await?;
        let result = self.do_call_tool(&conn.service, tool_name, arguments).await?;

        // 接続をプールに保存
        let mut connections = self.connections.write().await;
        connections.insert(server_name.clone(), conn);

        Ok(result)
    }

    /// ツール一覧取得の実装
    async fn do_list_tools(&self, service: &RunningService<RoleClient, ()>) -> Result<Vec<Tool>> {
        let result = service.list_tools(Default::default()).await?;
        Ok(result.tools)
    }

    /// ツール実行の実装
    async fn do_call_tool(
        &self,
        service: &RunningService<RoleClient, ()>,
        tool_name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<rmcp::model::CallToolResult> {
        let result = service.call_tool(CallToolRequestParams {
            name: tool_name.to_string().into(),
            arguments,
            meta: None,
            task: None,
        }).await?;
        Ok(result)
    }

    /// アイドル接続をクリーンアップ
    pub async fn cleanup_idle_connections(&self) {
        let mut connections = self.connections.write().await;
        let before = connections.len();

        connections.retain(|name, conn| {
            let elapsed = conn.last_used.elapsed().as_secs();
            if elapsed >= self.idle_timeout_seconds {
                info!("Closing idle connection to {} (idle: {}s)", name, elapsed);
                false
            } else {
                true
            }
        });

        let removed = before - connections.len();
        if removed > 0 {
            info!("Cleaned up {} idle MCP connections", removed);
        }
    }

    /// 全接続をクローズ
    pub async fn close_all(&self) {
        let mut connections = self.connections.write().await;
        let count = connections.len();
        connections.clear();
        info!("Closed {} MCP connections", count);
    }

    /// 現在の接続数を取得
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

/// MCPクライアント
pub struct MCPClient {
    config: MCPConfig,
    config_path: PathBuf,
    tools: Arc<RwLock<Vec<MCPToolDefinition>>>,
    pool: MCPConnectionPool,
}

impl MCPClient {
    /// 新しいMCPクライアントを作成
    pub fn new() -> Self {
        Self {
            config: MCPConfig::default(),
            config_path: PathBuf::from("mcp-servers.json"),
            tools: Arc::new(RwLock::new(Vec::new())),
            pool: MCPConnectionPool::new(300), // デフォルト5分アイドルタイムアウト
        }
    }

    /// 設定ファイルから読み込み
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading MCP config from {:?}", path);

        let content = std::fs::read_to_string(path)?;
        let config: MCPConfig = serde_json::from_str(&content)?;

        info!("Loaded {} MCP server configurations", config.servers.len());
        for server in &config.servers {
            debug!("  - {} (enabled: {})", server.name, server.enabled);
        }

        let idle_timeout = config.settings.connection_timeout_seconds * 10; // 接続タイムアウトの10倍

        Ok(Self {
            config,
            config_path: path.to_path_buf(),
            tools: Arc::new(RwLock::new(Vec::new())),
            pool: MCPConnectionPool::new(idle_timeout),
        })
    }

    /// 設定をファイルに保存
    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, content)?;
        info!("Saved MCP config to {:?}", self.config_path);
        Ok(())
    }

    /// サーバー設定を追加
    pub fn add_server(&mut self, server: MCPServerConfig) {
        info!("Adding MCP server: {}", server.name);
        self.config.servers.push(server);
    }

    /// サーバー設定を削除
    pub fn remove_server(&mut self, name: &str) -> bool {
        if let Some(pos) = self.config.servers.iter().position(|s| s.name == name) {
            info!("Removing MCP server: {}", name);
            self.config.servers.remove(pos);
            true
        } else {
            false
        }
    }

    /// サーバーを有効/無効化
    pub fn set_server_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(server) = self.config.servers.iter_mut().find(|s| s.name == name) {
            server.enabled = enabled;
            info!("Set MCP server {} enabled: {}", name, enabled);
            true
        } else {
            false
        }
    }

    /// サーバー一覧を取得
    pub fn list_servers(&self) -> &[MCPServerConfig] {
        &self.config.servers
    }

    /// 有効なサーバー一覧を取得
    pub fn list_enabled_servers(&self) -> Vec<&MCPServerConfig> {
        self.config.servers.iter().filter(|s| s.enabled).collect()
    }

    /// 全サーバーからツール一覧を取得
    pub async fn list_all_tools(&self) -> Vec<MCPToolDefinition> {
        self.tools.read().await.clone()
    }

    /// 指定サーバーに接続してツール一覧を取得（接続プールを使用）
    pub async fn refresh_tools_from_server(&self, server_name: &str) -> Result<Vec<MCPToolDefinition>> {
        let server = self.config.servers.iter()
            .find(|s| s.name == server_name)
            .ok_or_else(|| anyhow::anyhow!("Server not found: {}", server_name))?
            .clone();

        if !server.enabled {
            warn!("Server {} is disabled, skipping", server_name);
            return Ok(Vec::new());
        }

        debug!("Refreshing tools from MCP server: {}", server_name);

        // 接続プールを使用してツール一覧を取得
        let raw_tools = self.pool.list_tools(&server).await?;
        debug!("Received {} tools from {}", raw_tools.len(), server_name);

        let tools: Vec<MCPToolDefinition> = raw_tools.into_iter().map(|tool| {
            // input_schemaをValueに変換
            let schema_value = serde_json::to_value(&*tool.input_schema).unwrap_or(serde_json::json!({}));
            MCPToolDefinition {
                name: format!("mcp_{}_{}", server_name, tool.name),
                description: tool.description.unwrap_or_default().to_string(),
                input_schema: schema_value,
                server_name: server_name.to_string(),
            }
        }).collect();

        // キャッシュを更新
        let mut cached_tools = self.tools.write().await;
        // 同じサーバーの古いツールを削除
        cached_tools.retain(|t| t.server_name != server_name);
        // 新しいツールを追加
        cached_tools.extend(tools.clone());

        info!("Refreshed {} tools from server {} (pool connections: {})",
            tools.len(), server_name, self.pool.connection_count().await);
        Ok(tools)
    }

    /// 全有効サーバーからツールを更新
    pub async fn refresh_all_tools(&self) -> Result<usize> {
        let enabled_servers: Vec<_> = self.config.servers.iter()
            .filter(|s| s.enabled)
            .map(|s| s.name.clone())
            .collect();

        let mut total_tools = 0;
        for server_name in enabled_servers {
            match self.refresh_tools_from_server(&server_name).await {
                Ok(tools) => {
                    total_tools += tools.len();
                }
                Err(e) => {
                    error!("Failed to refresh tools from {}: {}", server_name, e);
                }
            }
        }

        info!("Total MCP tools available: {}", total_tools);
        Ok(total_tools)
    }

    /// ツールを実行（接続プールを使用）
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<serde_json::Value> {
        // ツール名からサーバー名を抽出 (mcp_<server>_<tool> 形式)
        let parts: Vec<&str> = tool_name.splitn(3, '_').collect();
        if parts.len() < 3 || parts[0] != "mcp" {
            return Err(anyhow::anyhow!("Invalid MCP tool name format: {}", tool_name));
        }

        let server_name = parts[1];
        let actual_tool_name = parts[2];

        let server = self.config.servers.iter()
            .find(|s| s.name == server_name)
            .ok_or_else(|| anyhow::anyhow!("Server not found: {}", server_name))?
            .clone();

        if !server.enabled {
            return Err(anyhow::anyhow!("Server {} is disabled", server_name));
        }

        debug!("Executing tool {} on server {} (pool)", actual_tool_name, server_name);

        // 接続プールを使用してツールを実行
        let result = self.pool.call_tool(&server, actual_tool_name, arguments).await?;

        // 結果をJSONに変換
        let value = serde_json::to_value(&result)?;
        debug!("Tool result: {:?}", value);

        Ok(value)
    }

    /// 設定を取得
    pub fn config(&self) -> &MCPConfig {
        &self.config
    }

    /// 設定を取得（mutable）
    pub fn config_mut(&mut self) -> &mut MCPConfig {
        &mut self.config
    }

    /// アイドル接続をクリーンアップ
    pub async fn cleanup_idle_connections(&self) {
        self.pool.cleanup_idle_connections().await;
    }

    /// 全接続をクローズ
    pub async fn close_all_connections(&self) {
        self.pool.close_all().await;
    }

    /// 現在の接続数を取得
    pub async fn connection_count(&self) -> usize {
        self.pool.connection_count().await
    }
}

impl Default for MCPClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mcp_config_default() {
        let config = MCPConfig::default();
        assert!(config.servers.is_empty());
        assert_eq!(config.settings.connection_timeout_seconds, 30);
        assert_eq!(config.settings.tool_execution_timeout_seconds, 60);
        assert_eq!(config.settings.max_concurrent_tools, 5);
    }

    #[test]
    fn test_mcp_config_serialization() {
        let config = MCPConfig {
            servers: vec![MCPServerConfig {
                name: "test".to_string(),
                command: "test-cmd".to_string(),
                args: vec!["--arg1".to_string()],
                env: HashMap::new(),
                enabled: true,
                description: "Test server".to_string(),
            }],
            settings: MCPSettings::default(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: MCPConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.servers[0].name, "test");
        assert_eq!(parsed.servers[0].command, "test-cmd");
        assert!(parsed.servers[0].enabled);
    }

    #[test]
    fn test_mcp_client_new() {
        let client = MCPClient::new();
        assert!(client.list_servers().is_empty());
    }

    #[test]
    fn test_mcp_client_add_remove_server() {
        let mut client = MCPClient::new();

        client.add_server(MCPServerConfig {
            name: "server1".to_string(),
            command: "cmd1".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            description: String::new(),
        });

        assert_eq!(client.list_servers().len(), 1);

        client.add_server(MCPServerConfig {
            name: "server2".to_string(),
            command: "cmd2".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: false,
            description: String::new(),
        });

        assert_eq!(client.list_servers().len(), 2);
        assert_eq!(client.list_enabled_servers().len(), 1);

        assert!(client.remove_server("server1"));
        assert_eq!(client.list_servers().len(), 1);
        assert!(!client.remove_server("nonexistent"));
    }

    #[test]
    fn test_mcp_client_set_enabled() {
        let mut client = MCPClient::new();

        client.add_server(MCPServerConfig {
            name: "test".to_string(),
            command: "cmd".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            description: String::new(),
        });

        assert!(client.set_server_enabled("test", false));
        assert!(!client.list_servers()[0].enabled);

        assert!(!client.set_server_enabled("nonexistent", true));
    }

    #[test]
    fn test_mcp_client_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test-mcp.json");

        let mut client = MCPClient::new();
        client.config_path = config_path.clone();

        client.add_server(MCPServerConfig {
            name: "saved".to_string(),
            command: "saved-cmd".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            description: "Saved server".to_string(),
        });

        client.save().unwrap();
        assert!(config_path.exists());

        let loaded = MCPClient::load(&config_path).unwrap();
        assert_eq!(loaded.list_servers().len(), 1);
        assert_eq!(loaded.list_servers()[0].name, "saved");
    }

    #[test]
    fn test_mcp_tool_definition() {
        let tool = MCPToolDefinition {
            name: "mcp_git_status".to_string(),
            description: "Get git status".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            server_name: "git".to_string(),
        };

        assert!(tool.name.starts_with("mcp_"));
        assert_eq!(tool.server_name, "git");
    }

    #[tokio::test]
    async fn test_mcp_client_list_tools() {
        let client = MCPClient::new();
        let tools = client.list_all_tools().await;
        assert!(tools.is_empty());
    }
}
