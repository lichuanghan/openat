//! Channel common utilities - shared code across all channels.
//!
//! Provides:
//! - User authorization helpers
//! - Common configuration traits
//! - Message parsing helpers

/// Trait for types that have allowed users list
pub trait HasAllowedUsers {
    /// Get reference to allowed users list
    fn allowed_users(&self) -> &[String];

    /// Check if a user is allowed
    fn is_allowed_user(&self, user_id: &str) -> bool {
        self.allowed_users().is_empty() || self.allowed_users().contains(&user_id.to_string())
    }
}

/// Trait for types that have enabled flag
pub trait IsEnabled {
    /// Check if this is enabled
    fn is_enabled(&self) -> bool;
}

/// Parse inbound message from channel components
#[derive(Debug, Clone)]
pub struct ParsedInbound<'a> {
    pub channel: &'static str,
    pub sender: &'a str,
    pub chat_id: &'a str,
    pub content: &'a str,
}

impl<'a> ParsedInbound<'a> {
    /// Create a new parsed inbound message
    pub fn new(channel: &'static str, sender: &'a str, chat_id: &'a str, content: &'a str) -> Self {
        Self { channel, sender, chat_id, content }
    }
}

/// Helper to validate API keys
pub fn validate_api_key(api_key: &str, name: &str, min_len: usize, errors: &mut Vec<String>) {
    if !api_key.is_empty() && api_key.len() < min_len {
        errors.push(format!("{} API key seems too short (minimum {} characters)", name, min_len));
    }
}

/// Helper to check if credentials are configured
pub fn has_credentials(api_key: &str) -> bool {
    !api_key.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestConfig {
        allowed_users: Vec<String>,
    }

    impl HasAllowedUsers for TestConfig {
        fn allowed_users(&self) -> &[String] {
            &self.allowed_users
        }
    }

    #[test]
    fn test_is_allowed_with_users() {
        let config = TestConfig {
            allowed_users: vec!["user1".to_string(), "user2".to_string()]
        };
        assert!(config.is_allowed_user("user1"));
        assert!(config.is_allowed_user("user2"));
        assert!(!config.is_allowed_user("user3"));
    }

    #[test]
    fn test_is_allowed_empty() {
        let config = TestConfig {
            allowed_users: vec![]
        };
        assert!(config.is_allowed_user("anyone"));
    }

    #[test]
    fn test_validate_api_key_short() {
        let mut errors = Vec::new();
        validate_api_key("short", "Test", 10, &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("too short"));
    }

    #[test]
    fn test_validate_api_key_empty() {
        let mut errors = Vec::new();
        validate_api_key("", "Test", 10, &mut errors);
        assert!(errors.is_empty());
    }
}
