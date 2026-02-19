use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// スケジュールID
pub type ScheduleId = Uuid;

/// スケジュールされたタスク
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: ScheduleId,
    pub cron_expression: String,
    pub prompt: String,
    pub channel_id: u64,
    pub created_at: DateTime<Utc>,
    pub enabled: bool,
}

impl ScheduledTask {
    pub fn new(cron_expression: String, prompt: String, channel_id: u64) -> Result<Self, SchedulerError> {
        // cron式をバリデーション
        let schedule = cron_expression.parse::<Schedule>()
            .map_err(|e| SchedulerError::InvalidCronExpression(format!("{}", e)))?;

        // 初回実行時間を確認（エラーにならないように）
        let _ = schedule.upcoming(Utc).next();

        Ok(Self {
            id: Uuid::new_v4(),
            cron_expression,
            prompt,
            channel_id,
            created_at: Utc::now(),
            enabled: true,
        })
    }

    /// 次回実行時刻を取得
    pub fn next_run(&self) -> Option<DateTime<Utc>> {
        let schedule = self.cron_expression.parse::<Schedule>().ok()?;
        schedule.upcoming(Utc).next()
    }
}

/// スケジューラーエラー
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Invalid cron expression: {0}")]
    InvalidCronExpression(String),

    #[error("Schedule not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

/// スケジュール実行イベント
#[derive(Debug, Clone)]
pub struct ScheduleEvent {
    pub task: ScheduledTask,
}

/// スケジューラー
pub struct Scheduler {
    tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    event_sender: broadcast::Sender<ScheduleEvent>,
}

impl Scheduler {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(16);
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            event_sender,
        }
    }

    /// イベントレシーバーを取得
    pub fn subscribe(&self) -> broadcast::Receiver<ScheduleEvent> {
        self.event_sender.subscribe()
    }

    /// スケジュールを追加
    pub async fn add_task(&self, task: ScheduledTask) -> ScheduleId {
        let id = task.id;
        let mut tasks = self.tasks.lock().await;
        info!("Adding scheduled task: {} (cron: {})", id, task.cron_expression);
        tasks.push(task);
        id
    }

    /// スケジュールを削除
    pub async fn remove_task(&self, id: ScheduleId) -> Result<ScheduledTask, SchedulerError> {
        let mut tasks = self.tasks.lock().await;
        let pos = tasks.iter().position(|t| t.id == id)
            .ok_or_else(|| SchedulerError::NotFound(id.to_string()))?;

        let removed = tasks.remove(pos);
        info!("Removed scheduled task: {}", id);
        Ok(removed)
    }

    /// スケジュールを一覧取得
    pub async fn list_tasks(&self) -> Vec<ScheduledTask> {
        self.tasks.lock().await.clone()
    }

    /// スケジュールをIDで取得
    pub async fn get_task(&self, id: ScheduleId) -> Option<ScheduledTask> {
        self.tasks.lock().await.iter().find(|t| t.id == id).cloned()
    }

    /// スケジュールの有効/無効を切り替え
    pub async fn toggle_task(&self, id: ScheduleId) -> Result<bool, SchedulerError> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks.iter_mut().find(|t| t.id == id)
            .ok_or_else(|| SchedulerError::NotFound(id.to_string()))?;

        task.enabled = !task.enabled;
        info!("Toggled task {} to enabled={}", id, task.enabled);
        Ok(task.enabled)
    }

    /// 全タスクを設定（ストアから復元用）
    pub async fn set_tasks(&self, new_tasks: Vec<ScheduledTask>) {
        let mut tasks = self.tasks.lock().await;
        *tasks = new_tasks;
        info!("Loaded {} scheduled tasks", tasks.len());
    }

    /// スケジューラーを開始
    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            info!("Scheduler started, checking every 60 seconds");

            loop {
                interval.tick().await;

                let tasks = self.tasks.lock().await;
                let now = Utc::now();

                for task in tasks.iter() {
                    if !task.enabled {
                        continue;
                    }

                    if let Some(next_run) = task.next_run() {
                        let diff = (next_run - now).num_seconds();

                        // 次回実行時刻が60秒以内なら実行
                        if diff <= 60 && diff > 0 {
                            info!("Triggering scheduled task: {}", task.id);

                            let event = ScheduleEvent {
                                task: task.clone(),
                            };

                            if let Err(e) = self.event_sender.send(event) {
                                error!("Failed to send schedule event: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduled_task_creation() {
        let task = ScheduledTask::new(
            "0 9 * * * *".to_string(),  // 毎時9分
            "Hello".to_string(),
            12345,
        );

        assert!(task.is_ok());
        let task = task.unwrap();
        assert!(task.enabled);
        assert!(task.next_run().is_some());
    }

    #[test]
    fn test_invalid_cron() {
        let task = ScheduledTask::new(
            "invalid".to_string(),
            "Hello".to_string(),
            12345,
        );

        assert!(task.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_add_remove() {
        let scheduler = Scheduler::new();

        let task = ScheduledTask::new(
            "0 9 * * * *".to_string(),
            "Hello".to_string(),
            12345,
        ).unwrap();

        let id = task.id;
        scheduler.add_task(task).await;

        assert_eq!(scheduler.list_tasks().await.len(), 1);

        let removed = scheduler.remove_task(id).await;
        assert!(removed.is_ok());
        assert_eq!(scheduler.list_tasks().await.len(), 0);
    }

    #[tokio::test]
    async fn test_scheduler_toggle() {
        let scheduler = Scheduler::new();

        let task = ScheduledTask::new(
            "0 9 * * * *".to_string(),
            "Hello".to_string(),
            12345,
        ).unwrap();

        let id = task.id;
        scheduler.add_task(task).await;

        let enabled = scheduler.toggle_task(id).await.unwrap();
        assert!(!enabled);

        let enabled = scheduler.toggle_task(id).await.unwrap();
        assert!(enabled);
    }
}
