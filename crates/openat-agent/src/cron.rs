//! Cron module for scheduled tasks

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use openat_runtime::MessageBus;
use openat_types::InboundMessage;

/// Cron job schedule type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleKind {
    /// One-time execution at specified time
    At,
    /// Interval execution every N seconds
    Every,
    /// Cron expression execution
    Cron,
}

/// Cron job schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    /// Schedule kind
    pub kind: ScheduleKind,
    /// For "at" - timestamp in milliseconds
    #[serde(rename = "atMs")]
    pub at_ms: Option<i64>,
    /// For "every" - interval in milliseconds
    #[serde(rename = "everyMs")]
    pub every_ms: Option<i64>,
    /// For "cron" - cron expression
    pub expr: Option<String>,
    /// Timezone for cron expression
    pub tz: Option<String>,
}

/// Cron job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronPayload {
    /// Message to send
    pub message: String,
    /// Target channel
    pub channel: Option<String>,
    /// Target chat_id
    pub to: Option<String>,
}

/// Cron job state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronJobState {
    /// Next run timestamp in ms
    #[serde(rename = "nextRunAtMs")]
    pub next_run_at_ms: Option<i64>,
    /// Last run timestamp in ms
    #[serde(rename = "lastRunAtMs")]
    pub last_run_at_ms: Option<i64>,
    /// Last execution status
    #[serde(rename = "lastStatus")]
    pub last_status: Option<String>,
    /// Last error message
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
}

/// Cron job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Job ID
    pub id: String,
    /// Job name/label
    pub name: String,
    /// Whether job is enabled
    pub enabled: bool,
    /// Schedule configuration
    pub schedule: CronSchedule,
    /// Job payload
    pub payload: CronPayload,
    /// Job state
    pub state: CronJobState,
    /// Created timestamp in ms
    #[serde(rename = "createdAtMs")]
    pub created_at_ms: i64,
    /// Updated timestamp in ms
    #[serde(rename = "updatedAtMs")]
    pub updated_at_ms: i64,
    /// Delete after run (for one-time jobs)
    #[serde(rename = "deleteAfterRun")]
    pub delete_after_run: bool,
}

/// Cron store
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronStore {
    pub jobs: Vec<CronJob>,
}

/// Compute next run time in milliseconds
fn compute_next_run(schedule: &CronSchedule, now_ms: i64) -> Option<i64> {
    match schedule.kind {
        ScheduleKind::At => {
            schedule.at_ms.filter(|&t| t > now_ms)
        }
        ScheduleKind::Every => {
            schedule.every_ms.filter(|&t| t > 0).map(|interval| now_ms + interval)
        }
        ScheduleKind::Cron => {
            if let Some(expr) = &schedule.expr {
                // Simple cron expression parsing
                // Format: "minute hour day month weekday"
                // Example: "0 9 * * *" = every day at 9:00
                compute_next_cron_run(expr, now_ms)
            } else {
                None
            }
        }
    }
}

/// Compute next run time for a cron expression
fn compute_next_cron_run(expr: &str, _now_ms: i64) -> Option<i64> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    let now = chrono::Utc::now();
    let mut next = now;

    // Simple implementation: add 1 minute and find next matching time
    // For a full implementation, a cron parsing library would be needed
    for _ in 0..1440 { // Max 1 day of iterations
        next = next + chrono::Duration::minutes(1);

        if cron_matches(&parts, &next) {
            return Some(next.timestamp_millis());
        }
    }

    None
}

/// Check if a datetime matches a cron expression
fn cron_matches(parts: &[&str], dt: &chrono::DateTime<chrono::Utc>) -> bool {
    if parts.len() != 5 {
        return false;
    }

    let minute = dt.format("%M").to_string().parse::<u32>().ok();
    let hour = dt.format("%H").to_string().parse::<u32>().ok();
    let day = dt.format("%d").to_string().parse::<u32>().ok();
    let month = dt.format("%m").to_string().parse::<u32>().ok();
    let weekday = dt.format("%w").to_string().parse::<u32>().ok();

    matches!(minute, Some(m) if cron_field_matches(parts[0], m))
        && matches!(hour, Some(h) if cron_field_matches(parts[1], h))
        && matches!(day, Some(d) if cron_field_matches(parts[2], d))
        && matches!(month, Some(m) if cron_field_matches(parts[3], m))
        && matches!(weekday, Some(w) if cron_field_matches(parts[4], w))
}

/// Check if a field value matches a cron field pattern
fn cron_field_matches(pattern: &str, value: u32) -> bool {
    if pattern == "*" {
        return true;
    }

    // Handle step values like "*/5"
    if let Some((step_str, _)) = pattern.split_once('/') {
        if step_str == "*" {
            if let Ok(step) = pattern[2..].parse::<u32>() {
                return value % step == 0;
            }
        }
    }

    // Handle list values like "1,2,3"
    if pattern.contains(',') {
        return pattern.split(',').any(|p| {
            p.parse::<u32>().map(|v| v == value).unwrap_or(false)
        });
    }

    // Handle range values like "1-5"
    if let Some((start_str, end_str)) = pattern.split_once('-') {
        if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>()) {
            return value >= start && value <= end;
        }
    }

    // Single value
    pattern.parse::<u32>().map(|v| v == value).unwrap_or(false)
}

/// Cron service for managing scheduled tasks
#[derive(Clone)]
pub struct CronService {
    store_path: PathBuf,
    jobs: Arc<RwLock<HashMap<String, CronJob>>>,
    bus: MessageBus,
    /// Timer task handle
    timer_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl CronService {
    /// Create a new CronService
    pub fn new(store_path: PathBuf, bus: MessageBus) -> Self {
        Self {
            store_path,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            bus,
            timer_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Load jobs from disk
    pub async fn load(&self) -> anyhow::Result<()> {
        match tokio::fs::read_to_string(&self.store_path).await {
            Ok(content) => {
                let store: CronStore = serde_json::from_str(&content)?;
                let mut jobs = self.jobs.write().await;
                for job in store.jobs {
                    // Compute next run time
                    let now_ms = Utc::now().timestamp_millis();
                    let next_run = compute_next_run(&job.schedule, now_ms);
                    let mut job = job;
                    job.state.next_run_at_ms = next_run;
                    jobs.insert(job.id.clone(), job);
                }
                tracing::info!("Loaded {} cron jobs", jobs.len());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No jobs file yet, that's ok
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    /// Save jobs to disk
    pub async fn save(&self) -> anyhow::Result<()> {
        let jobs = self.jobs.read().await;
        let store = CronStore {
            jobs: jobs.values().cloned().collect(),
        };
        if let Some(parent) = self.store_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(&store)?;
        tokio::fs::write(&self.store_path, content).await?;
        Ok(())
    }

    /// Add a new cron job
    pub async fn add_job(&self, name: String, schedule: CronSchedule, payload: CronPayload) -> String {
        let now_ms = Utc::now().timestamp_millis();
        let delete_after_run = matches!(schedule.kind, ScheduleKind::At);

        let job = CronJob {
            id: Uuid::new_v4().to_string(),
            name,
            enabled: true,
            schedule: schedule.clone(),
            payload,
            state: CronJobState {
                next_run_at_ms: compute_next_run(&schedule, now_ms),
                ..Default::default()
            },
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            delete_after_run,
        };

        let job_id = job.id.clone();
        self.jobs.write().await.insert(job_id.clone(), job);
        let _ = self.save().await;

        tracing::info!("Added cron job: {}", job_id);
        job_id
    }

    /// List all cron jobs
    pub async fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Remove a cron job
    pub async fn remove_job(&self, job_id: &str) -> bool {
        let removed = self.jobs.write().await.remove(job_id).is_some();
        if removed {
            let _ = self.save().await;
            tracing::info!("Removed cron job: {}", job_id);
        }
        removed
    }

    /// Enable or disable a cron job
    pub async fn set_job_enabled(&self, job_id: &str, enabled: bool) -> bool {
        let should_save;
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.enabled = enabled;
                job.updated_at_ms = Utc::now().timestamp_millis();
                should_save = true;
                tracing::info!("Set cron job {} enabled: {}", job_id, enabled);
            } else {
                should_save = false;
            }
        }
        // Save after dropping the write lock to avoid deadlock
        if should_save {
            let _ = self.save().await;
        }
        should_save
    }

    /// Start the cron timer
    pub async fn start(&self) {
        let jobs = self.jobs.clone();
        let bus = self.bus.clone();
        let store_path = self.store_path.clone();

        let handle = tokio::spawn(async move {
            loop {
                // Check every second
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let now_ms = Utc::now().timestamp_millis();
                let mut jobs_guard = jobs.write().await;
                let mut to_remove: Vec<String> = Vec::new();

                for (job_id, job) in jobs_guard.iter_mut() {
                    if !job.enabled {
                        continue;
                    }

                    if let Some(next_run) = job.state.next_run_at_ms {
                        if now_ms >= next_run {
                            tracing::info!("Executing cron job: {} ({})", job.name, job_id);

                            // Execute the job
                            let channel = job.payload.channel.as_deref().unwrap_or("cli");
                            let to = job.payload.to.as_deref().unwrap_or("cron");
                            let message = job.payload.message.clone();

                            let inbound = InboundMessage::new(channel, "cron", to, &message);
                            bus.publish_inbound(inbound).await;

                            // Update job state
                            job.state.last_run_at_ms = Some(now_ms);
                            job.state.next_run_at_ms = compute_next_run(&job.schedule, now_ms);

                            // Mark for deletion if one-time job
                            if job.delete_after_run {
                                to_remove.push(job_id.clone());
                            }

                            tracing::debug!("Cron job {} executed, next run: {:?}", job_id, job.state.next_run_at_ms);
                        }
                    }
                }

                // Remove completed one-time jobs
                for job_id in to_remove.clone() {
                    jobs_guard.remove(&job_id);
                }

                let has_changes = !to_remove.is_empty();

                drop(jobs_guard);

                // Save if there were changes
                if has_changes {
                    if let Some(parent) = store_path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let store = CronStore {
                        jobs: jobs.read().await.values().cloned().collect(),
                    };
                    if let Ok(content) = serde_json::to_string_pretty(&store) {
                        let _ = tokio::fs::write(&store_path, content).await;
                    }
                }
            }
        });

        *self.timer_task.write().await = Some(handle);
        tracing::info!("Cron service started");
    }

    /// Stop the cron timer
    pub async fn stop(&self) {
        if let Some(handle) = self.timer_task.write().await.take() {
            handle.abort();
            tracing::info!("Cron service stopped");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openat_runtime::MessageBus;

    #[tokio::test]
    async fn test_cron_add_list_remove() {
        let bus = MessageBus::new();
        let service = CronService::new(PathBuf::from("/tmp/test_cron_jobs.json"), bus);

        // Add a job
        let schedule = CronSchedule {
            kind: ScheduleKind::Every,
            at_ms: None,
            every_ms: Some(60000), // Every 60 seconds
            expr: None,
            tz: None,
        };
        let payload = CronPayload {
            message: "Test message".to_string(),
            channel: Some("test".to_string()),
            to: Some("test".to_string()),
        };

        let job_id = service.add_job("TestJob".to_string(), schedule, payload).await;
        println!("Added job with ID: {}", job_id);

        // List jobs
        let jobs = service.list_jobs().await;
        println!("Total jobs: {}", jobs.len());
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "TestJob");

        // Remove job
        let removed = service.remove_job(&job_id).await;
        assert!(removed);

        // Verify removed
        let jobs = service.list_jobs().await;
        assert_eq!(jobs.len(), 0);
    }
}
