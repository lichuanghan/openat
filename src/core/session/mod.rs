//! Session module - manages conversation sessions with JSONL persistence.
//!
//! # Features
//!
//! - Session creation and management
//! - Message history with automatic trimming
//! - JSONL file format for persistence
//! - Thread-safe operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session key (usually channel:chat_id)
    pub key: String,
    /// Message history
    pub messages: Vec<SessionMessage>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

/// A single message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// When the message was sent
    pub timestamp: DateTime<Utc>,
}

impl Session {
    /// Create a new session
    pub fn new(key: String) -> Self {
        let now = Utc::now();
        Self {
            key,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    /// Get message history (optionally limited)
    pub fn get_history(&self, max_messages: usize) -> Vec<HashMap<String, String>> {
        let recent = if self.messages.len() > max_messages {
            &self.messages[self.messages.len() - max_messages..]
        } else {
            &self.messages
        };

        recent
            .iter()
            .map(|m| {
                let mut map = HashMap::new();
                map.insert("role".to_string(), m.role.clone());
                map.insert("content".to_string(), m.content.clone());
                map
            })
            .collect()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    /// Get number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

/// Session manager - handles persistence
#[derive(Debug)]
pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(sessions_dir: PathBuf) -> Self {
        if let Err(e) = fs::create_dir_all(&sessions_dir) {
            tracing::warn!("Failed to create sessions directory: {}", e);
        }

        Self { sessions_dir }
    }

    /// Get sessions directory
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// Load a session from disk
    pub fn load(&self, key: &str) -> Option<Session> {
        let path = self.get_session_path(key);
        if !path.exists() {
            return None;
        }

        let mut messages = Vec::new();
        let mut created_at = Utc::now();
        let mut metadata = HashMap::new();

        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                    if data.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                        if let Some(ts) = data.get("created_at").and_then(|v| v.as_str()) {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                                created_at = dt.with_timezone(&Utc);
                            }
                        }
                        if let Some(meta) = data.get("metadata").and_then(|v| v.as_object()) {
                            metadata = meta
                                .iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                .collect();
                        }
                    } else if let Some(role) = data.get("role").and_then(|v| v.as_str()) {
                        if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                            let timestamp = data
                                .get("timestamp")
                                .and_then(|v| v.as_str())
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                .unwrap_or_else(|| Utc::now().fixed_offset());

                            messages.push(SessionMessage {
                                role: role.to_string(),
                                content: content.to_string(),
                                timestamp: timestamp.with_timezone(&Utc),
                            });
                        }
                    }
                }
            }
        }

        Some(Session {
            key: key.to_string(),
            messages,
            created_at,
            updated_at: Utc::now(),
            metadata,
        })
    }

    /// Save a session to disk
    pub fn save(&self, session: &Session) {
        let path = self.get_session_path(&session.key);

        let mut file = match File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to create session file: {}", e);
                return;
            }
        };

        // Write metadata
        let metadata_line = serde_json::json!({
            "_type": "metadata",
            "created_at": session.created_at.to_rfc3339(),
            "updated_at": session.updated_at.to_rfc3339(),
            "metadata": session.metadata
        });

        if let Err(e) = writeln!(file, "{}", metadata_line) {
            tracing::error!("Failed to write metadata: {}", e);
            return;
        }

        // Write messages
        for msg in &session.messages {
            if let Ok(line) = serde_json::to_string(&msg) {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    /// Delete a session
    pub fn delete(&self, key: &str) -> bool {
        let path = self.get_session_path(key);
        if path.exists() {
            return fs::remove_file(path).is_ok();
        }
        false
    }

    fn get_session_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace(":", "_");
        self.sessions_dir.join(format!("{}.jsonl", safe_key))
    }
}

/// Safe filename conversion
pub fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new("test_key".to_string());
        assert_eq!(session.key, "test_key");
        assert!(session.messages.is_empty());
        assert!(session.metadata.is_empty());
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new("test_key".to_string());
        session.add_message("user", "Hello");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[0].content, "Hello");
    }

    #[test]
    fn test_session_add_multiple_messages() {
        let mut session = Session::new("test_key".to_string());
        session.add_message("user", "Hello");
        session.add_message("assistant", "Hi there!");
        session.add_message("user", "How are you?");

        assert_eq!(session.messages.len(), 3);
        assert_eq!(session.message_count(), 3);
    }

    #[test]
    fn test_session_get_history_all() {
        let mut session = Session::new("test_key".to_string());
        session.add_message("user", "Hello 1");
        session.add_message("user", "Hello 2");
        session.add_message("user", "Hello 3");

        let history = session.get_history(10);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_session_get_history_limited() {
        let mut session = Session::new("test_key".to_string());
        session.add_message("user", "Hello 1");
        session.add_message("user", "Hello 2");
        session.add_message("user", "Hello 3");

        // Get only last 2 messages
        let history = session.get_history(2);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0]["content"], "Hello 2");
        assert_eq!(history[1]["content"], "Hello 3");
    }

    #[test]
    fn test_session_clear() {
        let mut session = Session::new("test_key".to_string());
        session.add_message("user", "Hello");
        assert_eq!(session.message_count(), 1);

        session.clear();
        assert_eq!(session.message_count(), 0);
    }

    #[test]
    fn test_safe_filename_basic() {
        assert_eq!(safe_filename("hello_world"), "hello_world");
        assert_eq!(safe_filename("file.txt"), "file.txt");
    }

    #[test]
    fn test_safe_filename_special_chars() {
        assert_eq!(safe_filename("hello:world"), "hello_world");
        assert_eq!(safe_filename("test/path"), "test_path");
        assert_eq!(safe_filename("file name"), "file_name");
        assert_eq!(safe_filename("data@v1.json"), "data_v1.json");
    }

    #[test]
    fn test_safe_filename_preserves_alphanumeric() {
        assert_eq!(safe_filename("abc123-xyz_789"), "abc123-xyz_789");
        assert_eq!(safe_filename("v1.2.3"), "v1.2.3");
    }
}
