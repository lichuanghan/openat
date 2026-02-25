//! Common utilities

use std::future::Future;
use std::time::Duration;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
        }
    }
}

/// Retry an async operation with exponential backoff
pub async fn retry_async<T, E, F, Fut>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut delay = config.initial_delay;
    let mut last_error: Option<E> = None;

    for attempt in 0..config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries - 1 {
                    tracing::warn!(
                        "Retry attempt {}/{} failed, waiting {:?}",
                        attempt + 1,
                        config.max_retries,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * config.backoff_factor).min(config.max_delay.as_secs_f64()),
                    );
                }
            }
        }
    }

    Err(last_error.unwrap())
}

/// Expand ~ to home directory
pub fn expand_home(path: &str) -> std::path::PathBuf {
    if path.starts_with("~") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(path.replacen("~", &home, 1));
        }
    }
    std::path::PathBuf::from(path)
}

/// Check if a string is empty or whitespace only
pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Trim and return empty string if still blank
pub fn trim_blank(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Generate a random string of given length
pub fn random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Measure execution time
pub async fn measure_time<F, T>(f: F) -> (T, std::time::Duration)
where
    F: std::future::Future<Output = T>,
{
    let start = std::time::Instant::now();
    let result = f.await;
    (result, start.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blank() {
        assert!(is_blank(""));
        assert!(is_blank("   "));
        assert!(!is_blank("hello"));
        assert!(!is_blank(" hello "));
    }

    #[test]
    fn test_trim_blank() {
        assert_eq!(trim_blank("hello"), Some("hello".to_string()));
        assert_eq!(trim_blank("  hello  "), Some("hello".to_string()));
        assert_eq!(trim_blank(""), None);
        assert_eq!(trim_blank("   "), None);
    }
}
