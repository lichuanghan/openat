//! Session module - manages conversation sessions with JSONL persistence.
//!
//! # Features
//!
//! - Session creation and management
//! - Message history with automatic trimming
//! - JSONL file format for persistence
//! - Thread-safe operations
//! - In-memory cache for performance
//! - Last consolidated tracking for memory integration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// Number of messages already consolidated to memory
    #[serde(default)]
    pub last_consolidated: usize,
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
            last_consolidated: 0,
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

    /// Get unconsolidated messages for LLM (since last consolidation)
    pub fn get_unconsolidated(&self, max_messages: usize) -> Vec<HashMap<String, String>> {
        let unconsolidated = &self.messages[self.last_consolidated..];
        let sliced = if unconsolidated.len() > max_messages {
            &unconsolidated[unconsolidated.len() - max_messages..]
        } else {
            unconsolidated
        };

        // Drop leading non-user messages to avoid orphaned tool_result blocks
        let start_idx = sliced.iter()
            .position(|m| m.role == "user")
            .unwrap_or(0);

        sliced[start_idx..]
            .iter()
            .map(|m| {
                let mut map = HashMap::new();
                map.insert("role".to_string(), m.role.clone());
                map.insert("content".to_string(), m.content.clone());
                map
            })
            .collect()
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
        self.last_consolidated = 0;
        self.updated_at = Utc::now();
    }

    /// Get number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

/// Session manager - handles persistence with in-memory cache
#[derive(Clone)]
pub struct SessionManager {
    sessions_dir: PathBuf,
    /// In-memory session cache
    cache: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(sessions_dir: PathBuf) -> Self {
        if let Err(e) = fs::create_dir_all(&sessions_dir) {
            tracing::warn!("Failed to create sessions directory: {}", e);
        }

        Self {
            sessions_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a session (with cache)
    pub async fn get_or_create(&self, key: &str) -> Session {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(session) = cache.get(key) {
                return session.clone();
            }
        }

        // Load from disk if not in cache
        let session = self.load(key).unwrap_or_else(|| Session::new(key.to_string()));

        // Add to cache
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), session.clone());

        session
    }

    /// Save session to cache and disk
    pub async fn save(&self, session: &Session) {
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(session.key.clone(), session.clone());
        }

        // Save to disk (blocking)
        let session_clone = session.clone();
        let sessions_dir = self.sessions_dir.clone();
        std::thread::spawn(move || {
            Self::save_to_disk_internal(&session_clone, &sessions_dir);
        });
    }

    /// Internal: Save session to disk (sync)
    fn save_to_disk_internal(session: &Session, sessions_dir: &PathBuf) {
        let path = sessions_dir.join(format!("{}.jsonl", session.key.replace(":", "_")));

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
            "last_consolidated": session.last_consolidated,
            "metadata": session.metadata
        });

        if let Err(e) = writeln!(file, "{}", metadata_line) {
            tracing::error!("Failed to write metadata: {}", e);
            return;
        }

        // Write messages
        for msg in &session.messages {
            if let Ok(line) = serde_json::to_string(msg) {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    /// Remove session from cache and optionally from disk
    pub async fn remove(&self, key: &str, delete_from_disk: bool) {
        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            cache.remove(key);
        }

        // Optionally delete from disk
        if delete_from_disk {
            let _ = self.delete(key);
        }
    }

    /// Get cached session count
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Clear all cached sessions (without deleting from disk)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::info!("Session cache cleared");
    }

    /// Get sessions directory
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// Load a session from disk
    pub fn load(&self, key: &str) -> Option<Session> {
        let path = self.get_session_path(key);

        let mut messages = Vec::new();
        let mut created_at = Utc::now();
        let mut metadata = HashMap::new();
        let mut last_consolidated = 0;

        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
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
                        if let Some(lc) = data.get("last_consolidated").and_then(|v| v.as_u64()) {
                            last_consolidated = lc as usize;
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
            last_consolidated,
        })
    }

    /// Delete a session
    pub fn delete(&self, key: &str) -> bool {
        let path = self.get_session_path(key);
        fs::remove_file(path).is_ok()
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
