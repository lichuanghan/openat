//! Scheduler module - handles cron-like scheduled tasks.
//!
//! This module provides two levels of scheduling:
//! - `Scheduler`: Full-featured scheduler that publishes jobs to the MessageBus
//! - `JobManager` / `CronJob`: Simple CLI-friendly job management
//!
//! # Examples
//!
//! ```ignore
//! use crate::core::scheduler::{Scheduler, JobManager, ScheduledJob};
//!
//! // CLI usage
//! let manager = JobManager::new();
//! let jobs = manager.load_jobs();
//!
//! // Gateway usage
//! let scheduler = Scheduler::new(&bus);
//! tokio::spawn(scheduler.run());
//! ```

use crate::config;
use crate::core::bus::MessageBus;
use crate::types::InboundMessage;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

/// Scheduled job definition - the core job type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub message: String,
    pub enabled: bool,
    pub interval_seconds: Option<u64>,
    pub cron_expression: Option<String>,
    pub deliver_response: bool,
    pub deliver_to: Option<String>,
    pub deliver_channel: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
}

/// Alias for ScheduledJob (CLI compatibility)
pub type CronJob = ScheduledJob;

impl ScheduledJob {
    /// Create a new scheduled job
    pub fn new(name: String, message: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            message,
            enabled: true,
            interval_seconds: None,
            cron_expression: None,
            deliver_response: false,
            deliver_to: None,
            deliver_channel: None,
            created_at: now,
            last_run: None,
            next_run: None,
        }
    }

    /// Calculate next run time based on interval or cron expression
    pub fn calculate_next_run(&mut self) {
        let now = Utc::now();

        if let Some(interval) = self.interval_seconds {
            if let Some(next) = now.checked_add_signed(chrono::Duration::seconds(interval as i64)) {
                self.next_run = Some(next);
            }
        } else if let Some(cron) = &self.cron_expression {
            if let Ok(next) = parse_cron(cron, now) {
                self.next_run = Some(next);
            } else {
                warn!("Failed to parse cron expression for job: {}", self.name);
                self.next_run = None;
            }
        } else {
            self.next_run = None;
        }
    }

    /// Check if job is due to run
    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(next) = self.next_run {
            Utc::now() >= next
        } else {
            true
        }
    }

    /// Mark job as having been run
    pub fn mark_run(&mut self) {
        self.last_run = Some(Utc::now());
        self.calculate_next_run();
        debug!("Job '{}' marked as run, next run: {:?}", self.name, self.next_run);
    }
}

/// Simple cron expression parser
fn parse_cron(expr: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err("Invalid cron expression: expected 5 fields".to_string());
    }

    let min = parts[0].parse::<u32>().map_err(|_| "Invalid minute")?;
    let hour = parts[1].parse::<u32>().map_err(|_| "Invalid hour")?;
    let day = parts[2].parse::<u32>().map_err(|_| "Invalid day")?;
    let mon = parts[3].parse::<u32>().map_err(|_| "Invalid month")?;
    let wday = parts[4].parse::<u32>().map_err(|_| "Invalid weekday")?;

    // Very basic next occurrence calculation
    let mut next = now;
    for _ in 0..366 {
        next = next + chrono::Duration::minutes(1);

        let next_min = next.minute() as u32;
        let next_hour = next.hour() as u32;
        let next_day = next.day();
        let next_month = next.month();
        let next_wday = next.weekday().num_days_from_sunday() as u32;

        if min == next_min || min == 255 {
            if hour == next_hour || hour == 255 {
                if day == next_day as u32 || day == 255 {
                    if mon == next_month as u32 || mon == 255 {
                        if wday == next_wday || wday == 255 {
                            return Ok(next);
                        }
                    }
                }
            }
        }
    }

    Err("Could not calculate next run".to_string())
}

/// Job manager - loads and persists scheduled jobs
#[derive(Debug, Clone)]
pub struct JobManager {
    jobs_dir: PathBuf,
}

/// Alias for JobManager (CLI compatibility)
pub type CronManager = JobManager;

impl JobManager {
    /// Create a new job manager
    pub fn new() -> Self {
        let jobs_dir = config::workspace_path().join("cron");
        if let Err(e) = fs::create_dir_all(&jobs_dir) {
            warn!("Failed to create jobs directory: {}", e);
        }
        debug!("Job manager initialized with jobs directory: {:?}", jobs_dir);
        Self { jobs_dir }
    }

    /// Create a job manager with custom directory
    pub fn with_dir(jobs_dir: PathBuf) -> Self {
        if let Err(e) = fs::create_dir_all(&jobs_dir) {
            warn!("Failed to create jobs directory: {}", e);
        }
        Self { jobs_dir }
    }

    /// Load all jobs from disk
    pub fn load_jobs(&self) -> Vec<ScheduledJob> {
        let mut jobs = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.jobs_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        match serde_json::from_str(&content) {
                            Ok(job) => jobs.push(job),
                            Err(e) => warn!("Failed to parse job file: {}", e),
                        }
                    }
                }
            }
        }

        debug!("Loaded {} jobs from disk", jobs.len());
        jobs
    }

    /// Save a job to disk
    pub fn save_job(&self, job: &ScheduledJob) {
        let path = self.jobs_dir.join(format!("{}.json", job.id));
        if let Ok(content) = serde_json::to_string_pretty(job) {
            if let Err(e) = fs::write(path, &content) {
                warn!("Failed to save job '{}': {}", job.name, e);
            }
        }
    }

    /// Delete a job from disk
    pub fn delete_job(&self, id: &str) -> bool {
        let path = self.jobs_dir.join(format!("{}.json", id));
        if path.exists() {
            if let Ok(()) = fs::remove_file(path) {
                info!("Deleted job: {}", id);
                return true;
            }
        }
        false
    }

    /// Add a new job
    pub fn add_job(&mut self, job: &mut ScheduledJob) {
        job.calculate_next_run();
        self.save_job(job);
        info!("Added job '{}' with next run: {:?}", job.name, job.next_run);
    }

    /// Toggle job enabled state
    pub fn toggle_job(&mut self, id: &str, enabled: bool) -> bool {
        let jobs = self.load_jobs();
        for job in jobs {
            if job.id == id {
                let mut job = job;
                job.enabled = enabled;
                job.calculate_next_run();
                self.save_job(&job);
                info!("{} job: {}", if enabled { "Enabled" } else { "Disabled" }, job.name);
                return true;
            }
        }
        false
    }

    /// Get jobs directory
    pub fn jobs_dir(&self) -> &PathBuf {
        &self.jobs_dir
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduler - runs scheduled jobs and publishes messages to the bus
#[derive(Debug)]
pub struct Scheduler {
    manager: JobManager,
    bus: MessageBus,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(bus: &MessageBus) -> Self {
        Self {
            manager: JobManager::new(),
            bus: bus.clone(),
        }
    }

    /// Run the scheduler loop
    pub async fn run(&self) {
        info!("Scheduler started");

        let mut check_interval = interval(Duration::from_secs(30));

        loop {
            check_interval.tick().await;

            let mut jobs = self.manager.load_jobs();

            for job in jobs.iter_mut() {
                if job.is_due() {
                    info!("Executing scheduled job: {}", job.name);
                    debug!("Job details: id={}, message={}", job.id, job.message);

                    // Execute the job - publish message to bus
                    self.execute_job(job).await;

                    job.mark_run();
                    self.manager.save_job(job);
                }
            }
        }
    }

    /// Execute a job - publish message to the bus
    async fn execute_job(&self, job: &ScheduledJob) {
        let channel = job.deliver_channel.clone().unwrap_or_else(|| "scheduler".to_string());
        let chat_id = job.deliver_to.clone().unwrap_or_else(|| "default".to_string());

        let message = InboundMessage::new(&channel, "scheduler", &chat_id, &job.message);

        self.bus.publish_inbound(message).await;

        info!("Published scheduled message from job '{}' to {}:{}",
              job.name, channel, chat_id);
    }
}
