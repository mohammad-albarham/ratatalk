//! Application state and event handling
//!
//! Central state management and event-driven architecture for ratatalk.

use std::cell::Cell;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;
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

    #[allow(dead_code)]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    SessionSelect,
    Help,
    DeleteConfirm,
}

/// Focus area in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusArea {
    #[default]
    Chat,
    Input,
    #[allow(dead_code)]
    Sidebar,
}

/// Statistics from the last response
#[derive(Debug, Clone, Default)]
pub struct ResponseStats {
    pub tokens: u32,
    pub tokens_per_second: f64,
    #[allow(dead_code)]
    pub total_duration_ms: u64,
}

#[derive(Debug)]
struct Cursor {
    base: Cell<usize>,
    position: Cell<usize>,
}
impl Cursor {
    #[inline]
    fn update_base(&self, delta: isize) {
        self.base.set((self.get_base() as isize + delta) as usize);
    }
    #[inline]
    fn update_position(&self, delta: isize) {
        self.position
            .set((self.get_position() as isize + delta) as usize);
    }
    /// new cursor instance
    pub fn new() -> Self {
        Self {
            base: Cell::new(0),
            position: Cell::new(0),
        }
    }

    /// get base
    #[inline]
    pub fn get_base(&self) -> usize {
        self.base.get()
    }

    /// get position
    #[inline]
    pub fn get_position(&self) -> usize {
        self.position.get()
    }

    /// get absolute position
    #[inline]
    pub fn get_absolute(&self) -> usize {
        self.get_base() + self.get_position()
    }

    /// move cursor to begin
    pub fn move_home(&self) {
        self.base.set(0);
        self.position.set(0);
    }

    /// move cursor to end
    pub fn move_end(&self, total: usize) -> bool {
        let base = self.get_base();
        let r = total >= base;
        if r {
            self.position.set(total - base);
        }
        r
    }

    /// move one char left
    pub fn move_left(&self) -> bool {
        let r = self.get_absolute() > 0;
        if r {
            if self.get_position() > 0 {
                self.update_position(-1);
            } else if self.get_base() > 0 {
                self.update_base(-1);
            }
        }
        r
    }

    /// move one char right
    pub fn move_right(&self, total: usize) -> bool {
        let r = total > self.get_absolute();
        if r {
            self.update_position(1);
        }
        r
    }

    /// move the base depend on the total width
    pub fn move_base(&self, mut delta: isize) {
        let base = self.get_base();
        let position = self.get_position();
        delta = if delta >= 0 {
            delta.min(position as isize)
        } else {
            delta.max(-(base as isize))
        };
        self.update_base(delta);
        self.update_position(-delta);
    }
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
    input: Vec<char>,
    
    /// Cursor position in input
    cursor: Cursor,
    
    /// Current input mode
    pub input_mode: InputMode,
    
    /// Current focus area
    #[allow(dead_code)]
    pub focus: FocusArea,
    
    /// Scroll offset for chat history
    pub chat_scroll: usize,
    
    /// Scroll offset for sidebar
    #[allow(dead_code)]
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
            input: Vec::new(),
            cursor: Cursor::new(),
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

    /// get the cursor position
    pub fn get_cursor(&self, max_width: usize) -> usize {
        let shown = &self.input[self.cursor.get_base()..][..self.cursor.get_position()];
        let width = shown.iter().map(|c| c.width_cjk().unwrap_or(1)).sum();
        max_width.min(width)
    }

    /// split the input at cursor
    pub fn split_at_cursor(&self, mut max_width: usize) -> (String, String) {
        // compute all chars with in tui
        let chars_width: Vec<_> = self
            .input
            .iter()
            .map(|c| c.width_cjk().unwrap_or(1))
            .collect();
        let mut before = String::new();
        let mut after = String::new();
        // rebase if chars can fit the input box
        if chars_width.iter().sum::<usize>() <= max_width {
            let delta = self.cursor.get_base() as isize;
            self.cursor.move_base(-delta);
            // split at cursor position
            let position = self.cursor.get_absolute();
            before.extend(&self.input[..position]);
            after.extend(&self.input[position..]);
        } else {
            // find the almost left of position of char in input box, rebase if possible
            let chars_width = &chars_width[self.cursor.get_base()..][..self.cursor.get_position()];
            let mut it = chars_width.iter().enumerate().rev();
            let position = loop {
                match it.next() {
                    Some((i, char_width)) => {
                        if *char_width < max_width - 1 {
                            max_width -= char_width;
                        } else {
                            self.cursor.move_base(i as isize);
                            break i;
                        }
                    }
                    None => break 0,
                }
            };

            // split at cursor position
            let shown = &self.input[self.cursor.get_base()..][..self.cursor.get_position()];
            before.extend(&shown[position..]);
            after.extend(&self.input[self.cursor.get_absolute()..]);
        }
        (before, after)
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor.get_absolute(), c);
        self.cursor.move_right(self.input.len());
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor.move_left() {
            self.input.remove(self.cursor.get_absolute());
        }
    }

    /// Delete character at cursor
    pub fn delete_char_forward(&mut self) {
        if self.cursor.get_absolute() < self.input.len() {
            self.input.remove(self.cursor.get_absolute());
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        self.cursor.move_left();
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        self.cursor.move_right(self.input.len());
    }

    /// Move cursor to start
    pub fn move_cursor_start(&mut self) {
        self.cursor.move_home();
    }

    /// Move cursor to end
    pub fn move_cursor_end(&mut self) {
        self.cursor.move_end(self.input.len());
    }

    /// Clear input buffer
    pub fn clear_input(&mut self) {
        self.move_cursor_start();
        self.input.clear();
    }

    /// Take and clear input, returning the content
    pub fn take_input(&mut self) -> String {
        self.move_cursor_start();
        let mut str = String::new();
        str.extend(std::mem::take(&mut self.input));
        str
    }

    /// conver input into String
    pub fn clone_input(&self) -> String {
        let mut str = String::new();
        str.extend(self.input.iter());
        str
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
    #[allow(dead_code)]
    Input(crossterm::event::KeyEvent),
    
    /// Terminal resize event
    #[allow(dead_code)]
    Resize(u16, u16),
    
    /// Tick event for animations/updates
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    SelectSession(usize),  // Direct session selection (for mouse clicks)
    RequestDeleteSession,
    ConfirmDeleteSession,
    CancelDeleteSession,
    
    // Model selection
    OpenModelSelect,
    CloseModelSelect,
    NextModel,
    PrevModel,
    ConfirmModel,
    SelectModel(usize),  // Direct model selection (for mouse clicks)
    
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
        
        assert_eq!(state.clone_input(), "hi");
        assert_eq!(state.get_cursor(10), 2);
        
        state.delete_char();
        assert_eq!(state.clone_input(), "h");
    }
}
