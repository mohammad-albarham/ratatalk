//! Persistence layer
//!
//! Handles saving and loading chat sessions to disk.

use crate::app::ChatSession;
use crate::error::PersistenceError;
use directories::ProjectDirs;
use std::path::PathBuf;

/// Get the data directory path
pub fn data_dir() -> Result<PathBuf, PersistenceError> {
    let proj_dirs = ProjectDirs::from("com", "ratatalk", "ratatalk")
        .ok_or(PersistenceError::NoDataDir)?;
    
    Ok(proj_dirs.data_dir().to_path_buf())
}

/// Get the sessions file path
pub fn sessions_path() -> Result<PathBuf, PersistenceError> {
    let dir = data_dir()?;
    Ok(dir.join("sessions.json"))
}

/// Load all sessions from disk
pub fn load_sessions() -> Result<Vec<ChatSession>, PersistenceError> {
    let path = sessions_path()?;
    
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(PersistenceError::Read)?;
    
    // Handle empty file
    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }

    let sessions: Vec<ChatSession> = serde_json::from_str(&contents)
        .map_err(PersistenceError::Parse)?;
    
    Ok(sessions)
}

/// Save all sessions to disk
pub fn save_sessions(sessions: &[ChatSession]) -> Result<(), PersistenceError> {
    let path = sessions_path()?;
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(PersistenceError::CreateDir)?;
    }

    let contents = serde_json::to_string_pretty(sessions)
        .map_err(PersistenceError::Serialize)?;
    
    std::fs::write(&path, contents)
        .map_err(PersistenceError::Write)?;
    
    Ok(())
}

/// Save a single session (merge with existing)
pub fn save_session(session: &ChatSession) -> Result<(), PersistenceError> {
    let mut sessions = load_sessions()?;
    
    // Find and update, or append
    if let Some(existing) = sessions.iter_mut().find(|s| s.id == session.id) {
        *existing = session.clone();
    } else {
        sessions.push(session.clone());
    }
    
    save_sessions(&sessions)
}

/// Delete a session by ID
pub fn delete_session(session_id: &uuid::Uuid) -> Result<(), PersistenceError> {
    let mut sessions = load_sessions()?;
    sessions.retain(|s| &s.id != session_id);
    save_sessions(&sessions)
}

/// Export a session to Markdown
pub fn export_session_to_markdown(session: &ChatSession) -> String {
    use crate::ollama::Role;
    
    let mut md = String::new();
    
    // Header
    md.push_str(&format!("# {}\n\n", session.name));
    md.push_str(&format!("**Model:** {}\n", session.model));
    md.push_str(&format!("**Created:** {}\n", session.created_at.format("%Y-%m-%d %H:%M")));
    md.push_str(&format!("**Updated:** {}\n\n", session.updated_at.format("%Y-%m-%d %H:%M")));
    
    // System prompt if present
    if let Some(system) = &session.system_prompt {
        md.push_str("## System Prompt\n\n");
        md.push_str(system);
        md.push_str("\n\n");
    }
    
    // Messages
    md.push_str("## Conversation\n\n");
    
    for message in &session.messages {
        let role_name = match message.role {
            Role::User => "**You**",
            Role::Assistant => "**Assistant**",
            Role::System => "**System**",
        };
        
        let timestamp = message.timestamp.format("%H:%M").to_string();
        md.push_str(&format!("{} ({})\n\n", role_name, timestamp));
        md.push_str(&message.content);
        md.push_str("\n\n---\n\n");
    }
    
    md
}

/// Export a session to a Markdown file
pub fn export_session_to_file(session: &ChatSession, path: &PathBuf) -> Result<(), PersistenceError> {
    let md = export_session_to_markdown(session);
    std::fs::write(path, md).map_err(PersistenceError::Write)
}

// ============================================================================
// Future: SQLite Schema (for reference)
// ============================================================================

/// SQL schema for future SQLite implementation
#[allow(dead_code)]
pub const SQLITE_SCHEMA: &str = r#"
-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    model TEXT NOT NULL,
    system_prompt TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    options_json TEXT
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    position INTEGER NOT NULL
);

-- Index for faster message retrieval
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, position);

-- Models cache table
CREATE TABLE IF NOT EXISTS models (
    name TEXT PRIMARY KEY,
    size INTEGER,
    modified_at TEXT,
    digest TEXT,
    details_json TEXT,
    last_fetched TEXT NOT NULL
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Message;
    use crate::ollama::Role;

    #[test]
    fn test_export_markdown() {
        let mut session = ChatSession::new("Test Chat", "llama3.2");
        session.messages.push(Message::user("Hello!"));
        session.messages.push(Message::assistant("Hi there!"));
        
        let md = export_session_to_markdown(&session);
        
        assert!(md.contains("# Test Chat"));
        assert!(md.contains("llama3.2"));
        assert!(md.contains("Hello!"));
        assert!(md.contains("Hi there!"));
    }

    #[test]
    fn test_sessions_serialization() {
        let session = ChatSession::new("Test", "llama3.2");
        let sessions = vec![session];
        
        let json = serde_json::to_string(&sessions).unwrap();
        let parsed: Vec<ChatSession> = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Test");
    }
}
