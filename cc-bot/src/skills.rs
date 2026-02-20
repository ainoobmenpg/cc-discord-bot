//! Skills実行エンジン
//!
//! 定義済みのスキル（コマンドシーケンス）を管理・実行します。
//! MCPツールとの連携もサポートします。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// スキルエラー
#[derive(Debug, Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Skill execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid skill definition: {0}")]
    InvalidDefinition(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// スキルのステップ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    /// ステップ名
    pub name: String,
    /// ツール名（MCPツールの場合は mcp_server_tool 形式）
    pub tool: String,
    /// ツール引数（テンプレート変数対応）
    #[serde(default)]
    pub args: HashMap<String, String>,
    /// 条件（省略時は常に実行）
    #[serde(default)]
    pub condition: Option<String>,
    /// 失敗時の処理
    #[serde(default)]
    pub on_failure: Option<FailureAction>,
}

/// 失敗時のアクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureAction {
    /// スキップして続行
    Skip,
    /// デフォルト値を使用
    UseDefault(String),
    /// エラーとして終了
    Fail,
}

/// スキル定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// スキルID
    pub id: String,
    /// スキル名
    pub name: String,
    /// 説明
    pub description: String,
    /// バージョン
    #[serde(default)]
    pub version: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 入力パラメータ定義
    #[serde(default)]
    pub parameters: Vec<SkillParameter>,
    /// 実行ステップ
    pub steps: Vec<SkillStep>,
    /// タグ
    #[serde(default)]
    pub tags: Vec<String>,
    /// 有効/無効
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

/// スキルパラメータ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    /// パラメータ名
    pub name: String,
    /// 型
    #[serde(default)]
    pub param_type: SkillParamType,
    /// 説明
    #[serde(default)]
    pub description: String,
    /// 必須フラグ
    #[serde(default)]
    pub required: bool,
    /// デフォルト値
    #[serde(default)]
    pub default: Option<String>,
}

/// パラメータ型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkillParamType {
    #[default]
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
}

/// スキル実行結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    /// スキルID
    pub skill_id: String,
    /// 成功フラグ
    pub success: bool,
    /// 各ステップの結果
    pub step_results: Vec<StepResult>,
    /// 最終出力
    pub output: Option<String>,
    /// エラーメッセージ
    pub error: Option<String>,
    /// 実行時間（ミリ秒）
    pub duration_ms: u64,
}

/// ステップ実行結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// ステップ名
    pub step_name: String,
    /// 成功フラグ
    pub success: bool,
    /// 出力
    pub output: Option<String>,
    /// エラー
    pub error: Option<String>,
}

/// スキル実行コンテキスト
#[derive(Debug, Clone)]
pub struct SkillContext {
    /// パラメータ値
    pub parameters: HashMap<String, String>,
    /// 前のステップの結果
    pub previous_results: Vec<StepResult>,
    /// 環境変数
    pub env: HashMap<String, String>,
}

impl SkillContext {
    /// 新しいコンテキストを作成
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
            previous_results: Vec::new(),
            env: HashMap::new(),
        }
    }

    /// パラメータを設定
    pub fn with_param(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }

    /// テンプレート変数を解決
    pub fn resolve_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        // パラメータ置換: ${param_name}
        for (key, value) in &self.parameters {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        // 前のステップの結果置換: ${step_name.output}
        for step_result in &self.previous_results {
            if let Some(ref output) = step_result.output {
                let placeholder = format!("${{{}.output}}", step_result.step_name);
                result = result.replace(&placeholder, output);
            }
        }

        // 環境変数置換: ${env.VAR_NAME}
        for (key, value) in &self.env {
            let placeholder = format!("${{env.{}}}", key);
            result = result.replace(&placeholder, value);
        }

        result
    }
}

impl Default for SkillContext {
    fn default() -> Self {
        Self::new()
    }
}

/// スキルストア
pub struct SkillStore {
    skills: RwLock<HashMap<String, Skill>>,
    skills_path: PathBuf,
}

impl SkillStore {
    /// 新しいスキルストアを作成
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            skills_path: PathBuf::from("skills"),
        }
    }

    /// スキルディレクトリから読み込み
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self, SkillError> {
        let path = path.as_ref();
        debug!("Loading skills from {:?}", path);

        let mut skills = HashMap::new();

        if path.exists() {
            let mut entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let file_path = entry.path();
                if file_path.extension().map_or(false, |ext| ext == "json") {
                    match Self::load_skill_file(&file_path).await {
                        Ok(skill) => {
                            info!("Loaded skill: {} ({})", skill.name, skill.id);
                            skills.insert(skill.id.clone(), skill);
                        }
                        Err(e) => {
                            error!("Failed to load skill from {:?}: {}", file_path, e);
                        }
                    }
                }
            }
        }

        info!("Loaded {} skills", skills.len());
        Ok(Self {
            skills: RwLock::new(skills),
            skills_path: path.to_path_buf(),
        })
    }

    /// 個別のスキルファイルを読み込み
    async fn load_skill_file(path: &Path) -> Result<Skill, SkillError> {
        let content = tokio::fs::read_to_string(path).await?;
        let skill: Skill = serde_json::from_str(&content)?;
        skill.validate()?;
        Ok(skill)
    }

    /// スキルを追加
    pub async fn add_skill(&self, skill: Skill) -> Result<(), SkillError> {
        skill.validate()?;
        let mut skills = self.skills.write().await;
        info!("Adding skill: {} ({})", skill.name, skill.id);
        skills.insert(skill.id.clone(), skill);
        Ok(())
    }

    /// スキルを削除
    pub async fn remove_skill(&self, id: &str) -> bool {
        let mut skills = self.skills.write().await;
        if skills.remove(id).is_some() {
            info!("Removed skill: {}", id);
            true
        } else {
            false
        }
    }

    /// スキルを取得
    pub async fn get_skill(&self, id: &str) -> Option<Skill> {
        self.skills.read().await.get(id).cloned()
    }

    /// 全スキル一覧を取得
    pub async fn list_skills(&self) -> Vec<Skill> {
        self.skills.read().await.values().cloned().collect()
    }

    /// 有効なスキル一覧を取得
    pub async fn list_enabled_skills(&self) -> Vec<Skill> {
        self.skills
            .read()
            .await
            .values()
            .filter(|s| s.enabled)
            .cloned()
            .collect()
    }

    /// スキルを有効/無効化
    pub async fn set_skill_enabled(&self, id: &str, enabled: bool) -> bool {
        let mut skills = self.skills.write().await;
        if let Some(skill) = skills.get_mut(id) {
            skill.enabled = enabled;
            info!("Set skill {} enabled: {}", id, enabled);
            true
        } else {
            false
        }
    }

    /// スキルをファイルに保存
    pub async fn save_skill(&self, skill: &Skill) -> Result<(), SkillError> {
        skill.validate()?;

        // ディレクトリを作成
        if !self.skills_path.exists() {
            tokio::fs::create_dir_all(&self.skills_path).await?;
        }

        let file_path = self.skills_path.join(format!("{}.json", skill.id));
        let content = serde_json::to_string_pretty(skill)?;
        tokio::fs::write(&file_path, content).await?;

        info!("Saved skill to {:?}", file_path);
        Ok(())
    }
}

impl Default for SkillStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Skill {
    /// スキル定義を検証
    pub fn validate(&self) -> Result<(), SkillError> {
        if self.id.is_empty() {
            return Err(SkillError::InvalidDefinition("Skill ID is required".to_string()));
        }
        if self.name.is_empty() {
            return Err(SkillError::InvalidDefinition("Skill name is required".to_string()));
        }
        if self.steps.is_empty() {
            return Err(SkillError::InvalidDefinition("At least one step is required".to_string()));
        }

        for (i, step) in self.steps.iter().enumerate() {
            if step.name.is_empty() {
                return Err(SkillError::InvalidDefinition(format!(
                    "Step {} has no name",
                    i
                )));
            }
            if step.tool.is_empty() {
                return Err(SkillError::InvalidDefinition(format!(
                    "Step {} '{}' has no tool",
                    i, step.name
                )));
            }
        }

        Ok(())
    }

    /// 必須パラメータのリストを取得
    pub fn required_parameters(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.as_str())
            .collect()
    }

    /// パラメータを検証
    pub fn validate_parameters(&self, provided: &HashMap<String, String>) -> Result<(), SkillError> {
        for param in &self.parameters {
            if param.required && !provided.contains_key(&param.name) {
                if param.default.is_none() {
                    return Err(SkillError::InvalidDefinition(format!(
                        "Required parameter '{}' is missing",
                        param.name
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_skill() -> Skill {
        Skill {
            id: "test-skill".to_string(),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            parameters: vec![SkillParameter {
                name: "input".to_string(),
                param_type: SkillParamType::String,
                description: "Input text".to_string(),
                required: true,
                default: None,
            }],
            steps: vec![SkillStep {
                name: "step1".to_string(),
                tool: "read_file".to_string(),
                args: [("path".to_string(), "${input}".to_string())]
                    .into_iter()
                    .collect(),
                condition: None,
                on_failure: None,
            }],
            tags: vec!["test".to_string()],
            enabled: true,
        }
    }

    #[test]
    fn test_skill_validation() {
        let skill = create_test_skill();
        assert!(skill.validate().is_ok());

        let invalid_skill = Skill {
            id: "".to_string(),
            name: "Invalid".to_string(),
            description: String::new(),
            version: String::new(),
            author: String::new(),
            parameters: vec![],
            steps: vec![],
            tags: vec![],
            enabled: true,
        };
        assert!(invalid_skill.validate().is_err());
    }

    #[test]
    fn test_skill_required_parameters() {
        let skill = create_test_skill();
        let required = skill.required_parameters();
        assert_eq!(required.len(), 1);
        assert!(required.contains(&"input"));
    }

    #[test]
    fn test_skill_validate_parameters() {
        let skill = create_test_skill();

        let mut params = HashMap::new();
        params.insert("input".to_string(), "test.txt".to_string());
        assert!(skill.validate_parameters(&params).is_ok());

        let empty_params = HashMap::new();
        assert!(skill.validate_parameters(&empty_params).is_err());
    }

    #[test]
    fn test_skill_context_resolve_template() {
        let ctx = SkillContext::new()
            .with_param("name".to_string(), "world".to_string());

        // パラメータ置換
        let result = ctx.resolve_template("Hello, ${name}!");
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_skill_context_resolve_step_output() {
        let mut ctx = SkillContext::new();
        ctx.previous_results.push(StepResult {
            step_name: "read".to_string(),
            success: true,
            output: Some("file content".to_string()),
            error: None,
        });

        let result = ctx.resolve_template("Content: ${read.output}");
        assert_eq!(result, "Content: file content");
    }

    #[tokio::test]
    async fn test_skill_store_new() {
        let store = SkillStore::new();
        let skills = store.list_skills().await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_skill_store_add_remove() {
        let store = SkillStore::new();
        let skill = create_test_skill();

        store.add_skill(skill.clone()).await.unwrap();
        let skills = store.list_skills().await;
        assert_eq!(skills.len(), 1);

        assert!(store.remove_skill("test-skill").await);
        let skills = store.list_skills().await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_skill_store_get() {
        let store = SkillStore::new();
        let skill = create_test_skill();

        store.add_skill(skill.clone()).await.unwrap();

        let retrieved = store.get_skill("test-skill").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Skill");

        let not_found = store.get_skill("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_skill_store_set_enabled() {
        let store = SkillStore::new();
        let skill = create_test_skill();

        store.add_skill(skill).await.unwrap();

        assert!(store.set_skill_enabled("test-skill", false).await);
        let enabled = store.list_enabled_skills().await;
        assert!(enabled.is_empty());

        assert!(!store.set_skill_enabled("nonexistent", true).await);
    }

    #[test]
    fn test_skill_step_serialization() {
        let step = SkillStep {
            name: "test".to_string(),
            tool: "read_file".to_string(),
            args: HashMap::new(),
            condition: Some("${input} != ''".to_string()),
            on_failure: Some(FailureAction::Skip),
        };

        let json = serde_json::to_string(&step).unwrap();
        let parsed: SkillStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert!(matches!(parsed.on_failure, Some(FailureAction::Skip)));
    }

    #[test]
    fn test_failure_action_serialization() {
        let actions = vec![
            FailureAction::Skip,
            FailureAction::UseDefault("default".to_string()),
            FailureAction::Fail,
        ];

        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let parsed: FailureAction = serde_json::from_str(&json).unwrap();
            match (&action, &parsed) {
                (FailureAction::Skip, FailureAction::Skip) => {}
                (FailureAction::Fail, FailureAction::Fail) => {}
                (FailureAction::UseDefault(a), FailureAction::UseDefault(b)) => {
                    assert_eq!(a, b);
                }
                _ => panic!("Mismatch"),
            }
        }
    }
}
