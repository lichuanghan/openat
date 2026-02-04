use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::time::{interval, Duration};
use tracing::info;

/// Cron job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
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

impl CronJob {
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

    /// Calculate next run time
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
                self.next_run = None;
            }
        } else {
            self.next_run = None;
        }
    }

    /// Check if job is due
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

    /// Mark job as run
    pub fn mark_run(&mut self) {
        self.last_run = Some(Utc::now());
        self.calculate_next_run();
    }
}

/// Simple cron parser
fn parse_cron(expr: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err("Invalid cron expression".to_string());
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

/// Cron job manager
#[derive(Debug)]
pub struct CronManager {
    jobs_dir: PathBuf,
}

impl CronManager {
    pub fn new(jobs_dir: PathBuf) -> Self {
        if let Err(e) = fs::create_dir_all(&jobs_dir) {
            tracing::warn!("Failed to create jobs directory: {}", e);
        }
        Self { jobs_dir }
    }

    /// Load all jobs
    pub fn load_jobs(&self) -> Vec<CronJob> {
        let mut jobs = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.jobs_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(job) = serde_json::from_str(&content) {
                            jobs.push(job);
                        }
                    }
                }
            }
        }

        jobs
    }

    /// Save a job
    pub fn save_job(&self, job: &CronJob) {
        let path = self.jobs_dir.join(format!("{}.json", job.id));
        if let Ok(content) = serde_json::to_string_pretty(job) {
            let _ = fs::write(path, content);
        }
    }

    /// Delete a job
    pub fn delete_job(&self, id: &str) -> bool {
        let path = self.jobs_dir.join(format!("{}.json", id));
        if path.exists() {
            return fs::remove_file(path).is_ok();
        }
        false
    }

    /// Add a job
    pub fn add_job(&mut self, job: &mut CronJob) {
        job.calculate_next_run();
        self.save_job(job);
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
                return true;
            }
        }
        false
    }
}

/// Cron executor that runs scheduled jobs
#[derive(Debug)]
pub struct CronExecutor {
    manager: CronManager,
}

impl CronExecutor {
    pub fn new(manager: CronManager) -> Self {
        Self { manager }
    }

    /// Run the cron executor
    pub async fn run(&mut self) {
        info!("Cron executor started");

        let mut check_interval = interval(Duration::from_secs(30));

        loop {
            check_interval.tick().await;

            let mut jobs = self.manager.load_jobs();

            for job in jobs.iter_mut() {
                if job.is_due() {
                    info!("Executing cron job: {}", job.name);

                    // Execute the job
                    let result = self.execute_job(job).await;

                    info!("Job '{}' result: {}", job.name, result);

                    job.mark_run();
                    self.manager.save_job(job);
                }
            }
        }
    }

    /// Execute a cron job
    async fn execute_job(&self, job: &CronJob) -> String {
        // TODO: Integrate with agent to process the message
        format!(
            "Cron job '{}' executed with message: {}",
            job.name,
            job.message
        )
    }
}
