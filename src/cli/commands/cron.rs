//! Cron commands - manage scheduled jobs.
//!
//! These commands use the core scheduler module for job management.

use crate::core::scheduler::{CronJob, CronManager, JobManager, ScheduledJob};
use anyhow::Result;
use dirs;
use std::path::PathBuf;

/// Get the default cron jobs directory
fn get_cron_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nanobot")
        .join("cron")
}

/// List scheduled jobs
pub fn list(all: bool) -> Result<()> {
    println!("=== Cron Jobs ===");
    println!("{}", "=".repeat(50));

    let jobs_dir = get_cron_dir();
    let manager = JobManager::with_dir(jobs_dir);
    let jobs = manager.load_jobs();

    if jobs.is_empty() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    for job in jobs {
        if !all && !job.enabled {
            continue;
        }

        println!("\n[{}] {}", if job.enabled { "X" } else { " " }, job.name);
        println!("  ID: {}", job.id);
        println!("  Message: {}", job.message);
        if let Some(interval) = job.interval_seconds {
            println!("  Every: {} seconds", interval);
        }
        if let Some(next) = job.next_run {
            println!("  Next run: {}", next);
        }
    }

    Ok(())
}

/// Add a new scheduled job
pub fn add(
    name: &str,
    message: &str,
    every: Option<u64>,
    cron: Option<String>,
    deliver: bool,
    to: Option<&str>,
    channel: Option<&str>,
) -> Result<()> {
    let jobs_dir = get_cron_dir();
    let mut manager = JobManager::with_dir(jobs_dir);

    let mut job = ScheduledJob::new(name.to_string(), message.to_string());
    job.interval_seconds = every;
    job.cron_expression = cron;
    job.deliver_response = deliver;
    job.deliver_to = to.map(|s| s.to_string());
    job.deliver_channel = channel.map(|s| s.to_string());

    manager.add_job(&mut job);

    println!("[+] Created cron job: {}", name);
    if let Some(next) = job.next_run {
        println!("  Next run: {}", next);
    }

    Ok(())
}

/// Remove a scheduled job
pub fn remove(job_id: &str) -> Result<()> {
    let jobs_dir = get_cron_dir();
    let manager = JobManager::with_dir(jobs_dir);

    if manager.delete_job(job_id) {
        println!("[+] Removed cron job: {}", job_id);
    } else {
        println!("[-] Cron job not found: {}", job_id);
    }

    Ok(())
}

/// Enable or disable a scheduled job
pub fn enable(job_id: &str, disable: bool) -> Result<()> {
    let jobs_dir = get_cron_dir();
    let mut manager = JobManager::with_dir(jobs_dir);

    if manager.toggle_job(job_id, !disable) {
        println!("[{}] Cron job: {}", if disable { "Disabled" } else { "Enabled" }, job_id);
    } else {
        println!("[-] Cron job not found: {}", job_id);
    }

    Ok(())
}
