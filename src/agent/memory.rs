use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

/// Long-term memory (persistent across sessions)
#[derive(Debug, Clone)]
pub struct LongTermMemory {
    path: PathBuf,
}

impl LongTermMemory {
    pub fn new(workspace: &PathBuf) -> Self {
        let path = workspace.join("memory").join("MEMORY.md");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Self { path }
    }

    /// Read the long-term memory
    pub fn read(&self) -> String {
        if self.path.exists() {
            if let Ok(content) = fs::read_to_string(&self.path) {
                return content;
            }
        }
        String::new()
    }

    /// Write to long-term memory
    pub fn write(&self, content: &str) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&self.path, content)
    }

    /// Append to long-term memory
    pub fn append(&self, content: &str) -> std::io::Result<()> {
        let existing = self.read();
        let new_content = if existing.is_empty() {
            content.to_string()
        } else {
            format!("{}\n\n{}", existing, content)
        };
        self.write(&new_content)
    }
}

/// Daily notes (session-specific, dated)
#[derive(Debug, Clone)]
pub struct DailyNotes {
    memory_dir: PathBuf,
}

impl DailyNotes {
    pub fn new(workspace: &PathBuf) -> Self {
        let memory_dir = workspace.join("memory");
        let _ = fs::create_dir_all(&memory_dir);
        Self { memory_dir }
    }

    /// Get today's note path
    pub fn today_path(&self) -> PathBuf {
        let today = Utc::now().format("%Y-%m-%d");
        self.memory_dir.join(format!("{}.md", today))
    }

    /// Read today's notes
    pub fn read_today(&self) -> String {
        let path = self.today_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                return content;
            }
        }
        String::new()
    }

    /// Append to today's notes
    pub fn append_today(&self, content: &str) -> std::io::Result<()> {
        let path = self.today_path();
        let existing = if path.exists() {
            fs::read_to_string(&path).unwrap_or_default()
        } else {
            format!("# Daily Notes - {}\n\n", Utc::now().format("%Y-%m-%d"))
        };

        let new_content = format!("{}\n\n{}", existing, content);
        fs::write(&path, new_content)
    }

    /// Read recent notes (last N days)
    pub fn read_recent(&self, days: usize) -> Vec<(DateTime<Utc>, String)> {
        let mut notes = Vec::new();
        let today = Utc::now().date_naive();

        for i in 0..days {
            if let Some(date) = today.pred_opt() {
                let path = self.memory_dir.join(format!("{}.md", date.format("%Y-%m-%d")));
                if path.exists() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let dt = date.and_hms_opt(0, 0, 0).unwrap_or_else(|| Utc::now())
                            .fixed_offset();
                        notes.push((dt, content));
                    }
                }
            }
        }

        notes
    }
}

/// Memory manager combining all memory types
#[derive(Debug, Clone)]
pub struct MemoryManager {
    long_term: LongTermMemory,
    daily: DailyNotes,
}

impl MemoryManager {
    pub fn new(workspace: &PathBuf) -> Self {
        Self {
            long_term: LongTermMemory::new(workspace),
            daily: DailyNotes::new(workspace),
        }
    }

    /// Get memory for context
    pub fn get_context(&self) -> String {
        let mut context = String::new();

        // Add long-term memory
        let long_term = self.long_term.read();
        if !long_term.is_empty() {
            context.push_str("## Long-term Memory\n\n");
            context.push_str(&long_term);
            context.push_str("\n\n");
        }

        // Add recent daily notes (last 3 days)
        let recent_notes = self.daily.read_recent(3);
        if !recent_notes.is_empty() {
            context.push_str("## Recent Notes\n\n");
            for (_, note) in recent_notes {
                context.push_str(&note);
                context.push_str("\n\n");
            }
        }

        context
    }

    /// Remember something important
    pub fn remember(&self, content: &str) -> std::io::Result<()> {
        self.long_term.append(content)
    }

    /// Take a note for today
    pub fn note(&self, content: &str) -> std::io::Result<()> {
        self.daily.append_today(content)
    }
}
