//! Common utilities

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
