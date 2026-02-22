//! Common channel utilities

/// Check if user is allowed
pub fn is_allowed_user(allowed_users: &[String], user_id: &str) -> bool {
    allowed_users.is_empty() || allowed_users.iter().any(|u| u == user_id)
}

/// Validate API key
pub fn validate_api_key(api_key: &str, name: &str, min_len: usize, errors: &mut Vec<String>) {
    if !api_key.is_empty() && api_key.len() < min_len {
        errors.push(format!("{} API key seems too short (minimum {} characters)", name, min_len));
    }
}
