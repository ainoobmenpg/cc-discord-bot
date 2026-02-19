#![allow(dead_code)]

use crate::scheduler::{ScheduledTask, ScheduleId, SchedulerError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// スケジュールストア（JSON永続化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleStore {
    pub tasks: Vec<ScheduledTask>,
    pub version: u32,
}

impl ScheduleStore {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            version: 1,
        }
    }

    /// ファイルパスを生成
    fn get_file_path(base_dir: &str) -> PathBuf {
        Path::new(base_dir).join("schedules.json")
    }

    /// JSONファイルから読み込み
    pub async fn load(base_dir: &str) -> Result<Self, SchedulerError> {
        let path = Self::get_file_path(base_dir);
        debug!("Loading schedules from {:?}", path);

        if !path.exists() {
            info!("Schedule file not found, creating new store");
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| SchedulerError::StorageError(format!("Failed to read file: {}", e)))?;

        let store: Self = serde_json::from_str(&content)
            .map_err(|e| {
                warn!("Failed to parse schedule file, creating new store: {}", e);
                SchedulerError::StorageError(format!("Failed to parse JSON: {}", e))
            })?;

        info!("Loaded {} scheduled tasks", store.tasks.len());
        Ok(store)
    }

    /// JSONファイルに保存
    pub async fn save(&self, base_dir: &str) -> Result<(), SchedulerError> {
        let path = Self::get_file_path(base_dir);
        debug!("Saving schedules to {:?}", path);

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| SchedulerError::StorageError(format!("Failed to create directory: {}", e)))?;
            }
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| SchedulerError::StorageError(format!("Failed to serialize: {}", e)))?;

        fs::write(&path, content)
            .await
            .map_err(|e| SchedulerError::StorageError(format!("Failed to write file: {}", e)))?;

        info!("Saved {} scheduled tasks", self.tasks.len());
        Ok(())
    }

    /// タスクを追加
    pub fn add_task(&mut self, task: ScheduledTask) -> ScheduleId {
        let id = task.id;
        self.tasks.push(task);
        id
    }

    /// タスクを削除
    pub fn remove_task(&mut self, id: ScheduleId) -> Option<ScheduledTask> {
        let pos = self.tasks.iter().position(|t| t.id == id)?;
        Some(self.tasks.remove(pos))
    }

    /// タスクを更新
    pub fn update_task(&mut self, task: ScheduledTask) -> bool {
        if let Some(existing) = self.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
            true
        } else {
            false
        }
    }

    /// 全タスクを取得
    pub fn get_tasks(&self) -> &[ScheduledTask] {
        &self.tasks
    }

    /// タスク数を取得
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// 空かどうか
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for ScheduleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_new() {
        let store = ScheduleStore::new();
        assert!(store.is_empty());
        assert_eq!(store.version, 1);
    }

    #[test]
    fn test_store_add_remove() {
        let mut store = ScheduleStore::new();

        let task = ScheduledTask::new(
            "0 9 * * * *".to_string(),
            "Hello".to_string(),
            12345,
        ).unwrap();

        let id = task.id;
        store.add_task(task);

        assert_eq!(store.len(), 1);

        let removed = store.remove_task(id);
        assert!(removed.is_some());
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn test_store_save_load() {
        let dir = tempdir().unwrap();
        let base_dir = dir.path().to_str().unwrap();

        let mut store = ScheduleStore::new();

        let task = ScheduledTask::new(
            "0 9 * * * *".to_string(),
            "Hello".to_string(),
            12345,
        ).unwrap();

        store.add_task(task);
        store.save(base_dir).await.unwrap();

        let loaded = ScheduleStore::load(base_dir).await.unwrap();
        assert_eq!(loaded.len(), 1);
    }
}
