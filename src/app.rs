//! Application state and event handling
//!
//! Central state management and event-driven architecture for ratatalk.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::ollama::{ChatMessage, GenerationOptions, ModelInfo, Role};

// ============================================================================
// Core Data Structures
// ============================================================================

/// A message in a chat session with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    /// True if this message is still being streamed
    #[serde(default)]
    pub streaming: bool,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content: content.into(),
            timestamp: Utc::now(),
            streaming: false,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a new streaming assistant message (initially empty)
    pub fn assistant_streaming() -> Self {
        Self {
            id: Uuid::new_v4(),
            role: Role::Assistant,
            content: String::new(),
            timestamp: Utc::now(),
            streaming: true,
        }
    }

    /// Append content to this message (for streaming)
    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Mark streaming as complete
    pub fn finish_streaming(&mut self) {
        self.streaming = false;
    }

    /// Convert to Ollama ChatMessage
    pub fn to_chat_message(&self) -> ChatMessage {
        ChatMessage {
            role: self.role,
            content: self.content.clone(),
            images: None,
        }
    }
}

/// A chat session containing a conversation with a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: Uuid,
    pub name: String,
    pub model: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Optional system prompt for this session
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Session-specific generation options
    #[serde(default)]
    pub options: Option<GenerationOptions>,
}

impl ChatSession {
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            model: model.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            system_prompt: None,
            options: None,
        }
    }

    /// Create with a default name based on timestamp
    pub fn with_default_name(model: impl Into<String>) -> Self {
        let now = Utc::now();
        let name = now.format("Chat %Y-%m-%d %H:%M").to_string();
        Self::new(name, model)
    }

    /// Add a user message to the session
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.messages.push(Message::user(content));
        self.updated_at = Utc::now();
    }

    /// Start a new streaming assistant response
    pub fn start_assistant_response(&mut self) -> usize {
        self.messages.push(Message::assistant_streaming());
        self.updated_at = Utc::now();
        self.messages.len() - 1
    }

    /// Append to the current streaming response
    pub fn append_to_response(&mut self, text: &str) {
        if let Some(msg) = self.messages.last_mut() {
            if msg.streaming {
                msg.append(text);
                self.updated_at = Utc::now();
            }
        }
    }

    /// Finish the current streaming response
    pub fn finish_response(&mut self) {
        if let Some(msg) = self.messages.last_mut() {
            msg.finish_streaming();
            self.updated_at = Utc::now();
        }
    }

    /// Get messages formatted for Ollama API
    pub fn to_chat_messages(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        
        // Add system prompt if present
        if let Some(system) = &self.system_prompt {
            messages.push(ChatMessage::system(system.clone()));
        }
        
        // Add all conversation messages
        for msg in &self.messages {
            messages.push(msg.to_chat_message());
        }
        
        messages
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if there's an active streaming response
    pub fn is_streaming(&self) -> bool {
        self.messages.last().map(|m| m.streaming).unwrap_or(false)
    }

    /// Get a preview of the last message for sidebar display
    pub fn preview(&self) -> &str {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.as_str())
            .unwrap_or("(empty)")
    }
}

// ============================================================================
// Application State
// ============================================================================

/// Input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
    ModelSelect,
    SessionSelect,
    Help,
}

/// Focus area in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusArea {
    #[default]
    Chat,
    Input,
    Sidebar,
}

/// Statistics from the last response
#[derive(Debug, Clone, Default)]
pub struct ResponseStats {
    pub tokens: u32,
    pub tokens_per_second: f64,
    pub total_duration_ms: u64,
}

/// Central application state
#[derive(Debug)]
pub struct AppState {
    /// Configuration
    pub config: Config,
    
    /// Available models from Ollama
    pub models: Vec<ModelInfo>,
    
    /// All chat sessions
    pub sessions: Vec<ChatSession>,
    
    /// Index of the currently active session
    pub active_session_idx: usize,
    
    /// Index of the currently selected model (for model picker)
    pub selected_model_idx: usize,
    
    /// User input buffer
    pub input: String,
    
    /// Cursor position in input
    pub cursor_position: usize,
    
    /// Current input mode
    pub input_mode: InputMode,
    
    /// Current focus area
    pub focus: FocusArea,
    
    /// Scroll offset for chat history
    pub chat_scroll: usize,
    
    /// Scroll offset for sidebar
    pub sidebar_scroll: usize,
    
    /// Status message (shown in status bar)
    pub status_message: Option<String>,
    
    /// Error message (shown prominently)
    pub error_message: Option<String>,
    
    /// Whether we're currently loading (models, sending, etc.)
    pub loading: bool,
    
    /// Whether a response is currently streaming
    pub streaming: bool,
    
    /// Stats from the last completed response
    pub last_response_stats: Option<ResponseStats>,
    
    /// Whether the app should quit
    pub should_quit: bool,
    
    /// Whether Ollama server is connected
    pub server_connected: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let default_model = config.model.default_model.clone();
        
        Self {
            config,
            models: Vec::new(),
            sessions: vec![ChatSession::with_default_name(&default_model)],
            active_session_idx: 0,
            selected_model_idx: 0,
            input: String::new(),
            cursor_position: 0,
            input_mode: InputMode::Normal,
            focus: FocusArea::Input,
            chat_scroll: 0,
            sidebar_scroll: 0,
            status_message: None,
            error_message: None,
            loading: false,
            streaming: false,
            last_response_stats: None,
            should_quit: false,
            server_connected: false,
        }
    }

    /// Get the current active session
    pub fn active_session(&self) -> Option<&ChatSession> {
        self.sessions.get(self.active_session_idx)
    }

    /// Get the current active session mutably
    pub fn active_session_mut(&mut self) -> Option<&mut ChatSession> {
        self.sessions.get_mut(self.active_session_idx)
    }

    /// Get the current model name
    pub fn current_model(&self) -> &str {
        self.active_session()
            .map(|s| s.model.as_str())
            .unwrap_or(&self.config.model.default_model)
    }

    /// Create a new session with the current model
    pub fn new_session(&mut self) {
        let model = self.current_model().to_string();
        let session = ChatSession::with_default_name(model);
        self.sessions.push(session);
        self.active_session_idx = self.sessions.len() - 1;
        self.chat_scroll = 0;
        self.clear_status();
    }

    /// Switch to the next session
    pub fn next_session(&mut self) {
        if !self.sessions.is_empty() {
            self.active_session_idx = (self.active_session_idx + 1) % self.sessions.len();
            self.chat_scroll = 0;
        }
    }

    /// Switch to the previous session
    pub fn prev_session(&mut self) {
        if !self.sessions.is_empty() {
            self.active_session_idx = if self.active_session_idx == 0 {
                self.sessions.len() - 1
            } else {
                self.active_session_idx - 1
            };
            self.chat_scroll = 0;
        }
    }

    /// Delete the current session
    pub fn delete_current_session(&mut self) {
        if self.sessions.len() > 1 {
            self.sessions.remove(self.active_session_idx);
            if self.active_session_idx >= self.sessions.len() {
                self.active_session_idx = self.sessions.len() - 1;
            }
            self.chat_scroll = 0;
        }
    }

    /// Set the model for the current session
    pub fn set_model(&mut self, model: impl Into<String>) {
        if let Some(session) = self.active_session_mut() {
            session.model = model.into();
        }
    }

    /// Get the selected model from the model list
    pub fn selected_model(&self) -> Option<&ModelInfo> {
        self.models.get(self.selected_model_idx)
    }

    /// Select next model in list
    pub fn next_model(&mut self) {
        if !self.models.is_empty() {
            self.selected_model_idx = (self.selected_model_idx + 1) % self.models.len();
        }
    }

    /// Select previous model in list
    pub fn prev_model(&mut self) {
        if !self.models.is_empty() {
            self.selected_model_idx = if self.selected_model_idx == 0 {
                self.models.len() - 1
            } else {
                self.selected_model_idx - 1
            };
        }
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input.remove(self.cursor_position);
        }
    }

    /// Delete character at cursor
    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.input.len() {
            self.input.remove(self.cursor_position);
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to start
    pub fn move_cursor_start(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end
    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.input.len();
    }

    /// Clear input buffer
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Take and clear input, returning the content
    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input);
        self.cursor_position = 0;
        input
    }

    /// Set status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Set error message
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Scroll chat up
    pub fn scroll_up(&mut self, amount: usize) {
        self.chat_scroll = self.chat_scroll.saturating_add(amount);
    }

    /// Scroll chat down
    pub fn scroll_down(&mut self, amount: usize) {
        self.chat_scroll = self.chat_scroll.saturating_sub(amount);
    }

    /// Reset scroll to bottom (most recent messages)
    pub fn scroll_to_bottom(&mut self) {
        self.chat_scroll = 0;
    }
}

// ============================================================================
// Application Events
// ============================================================================

/// Events that can occur in the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Terminal input event
    Input(crossterm::event::KeyEvent),
    
    /// Terminal resize event
    Resize(u16, u16),
    
    /// Tick event for animations/updates
    Tick,
    
    /// Models loaded from Ollama
    ModelsLoaded(Vec<ModelInfo>),
    
    /// Error loading models
    ModelsError(String),
    
    /// New token chunk received from streaming response
    StreamChunk(String),
    
    /// Stream completed with stats
    StreamComplete(ResponseStats),
    
    /// Stream error
    StreamError(String),
    
    /// Server connection status changed
    ServerStatus(bool),
    
    /// Request to quit
    Quit,
}

/// Actions that can be dispatched to update state
#[derive(Debug, Clone)]
pub enum AppAction {
    // Navigation
    NextSession,
    PrevSession,
    NewSession,
    DeleteSession,
    
    // Model selection
    OpenModelSelect,
    CloseModelSelect,
    NextModel,
    PrevModel,
    ConfirmModel,
    
    // Input
    EnterEditMode,
    ExitEditMode,
    SubmitMessage,
    InsertChar(char),
    DeleteChar,
    DeleteCharForward,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStart,
    MoveCursorEnd,
    ClearInput,
    
    // Scrolling
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollToTop,
    ScrollToBottom,
    PageUp,
    PageDown,
    
    // Misc
    ToggleHelp,
    ClearError,
    Quit,
    
    // Server
    RefreshModels,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
        assert!(!msg.streaming);
    }

    #[test]
    fn test_session_streaming() {
        let mut session = ChatSession::new("Test", "llama3.2");
        session.add_user_message("Hi");
        session.start_assistant_response();
        
        assert!(session.is_streaming());
        
        session.append_to_response("Hello");
        session.append_to_response(" world!");
        session.finish_response();
        
        assert!(!session.is_streaming());
        assert_eq!(session.messages.last().unwrap().content, "Hello world!");
    }

    #[test]
    fn test_app_state_input() {
        let config = Config::default();
        let mut state = AppState::new(config);
        
        state.insert_char('h');
        state.insert_char('i');
        
        assert_eq!(state.input, "hi");
        assert_eq!(state.cursor_position, 2);
        
        state.delete_char();
        assert_eq!(state.input, "h");
    }
}
