//! Event handling and keybindings
//!
//! Handles terminal input events and maps them to application actions.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::Rect;
use std::time::Duration;
use tracing::{info, warn};

use crate::app::{AppAction, AppState, InputMode};
use crate::persistence;
use crate::ui::AppLayout;

/// Event handler configuration
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next event, with timeout
    pub fn poll(&self) -> std::io::Result<Option<Event>> {
        if event::poll(self.tick_rate)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}

/// Map a key event to an application action based on current mode
pub fn handle_key_event(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Global keybindings (work in any mode)
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Some(AppAction::Quit),
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => return Some(AppAction::Quit),
        _ => {}
    }

    // Mode-specific keybindings
    match state.input_mode {
        InputMode::Normal => handle_normal_mode(key, state),
        InputMode::Editing => handle_editing_mode(key),
        InputMode::ModelSelect => handle_model_select_mode(key),
        InputMode::SessionSelect => handle_session_select_mode(key),
        InputMode::Help => handle_help_mode(key),
        InputMode::DeleteConfirm => handle_delete_confirm_mode(key),
    }
}

/// Handle keys in normal mode
fn handle_normal_mode(key: KeyEvent, _state: &AppState) -> Option<AppAction> {
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) => Some(AppAction::Quit),
        
        // Enter edit mode
        (KeyCode::Enter, _) | (KeyCode::Char('i'), KeyModifiers::NONE) => {
            Some(AppAction::EnterEditMode)
        }
        
        // Session navigation
        (KeyCode::Tab, KeyModifiers::NONE) => Some(AppAction::NextSession),
        (KeyCode::BackTab, _) => Some(AppAction::PrevSession),
        (KeyCode::Char('n'), KeyModifiers::CONTROL) => Some(AppAction::NewSession),
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => Some(AppAction::RequestDeleteSession),
        
        // Model selection
        (KeyCode::Char('m'), KeyModifiers::NONE) => Some(AppAction::OpenModelSelect),
        
        // Scrolling
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
            Some(AppAction::ScrollUp(1))
        }
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
            Some(AppAction::ScrollDown(1))
        }
        (KeyCode::PageUp, _) | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
            Some(AppAction::PageUp)
        }
        (KeyCode::PageDown, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            Some(AppAction::PageDown)
        }
        (KeyCode::Home, _) | (KeyCode::Char('g'), KeyModifiers::NONE) => {
            Some(AppAction::ScrollToTop)
        }
        (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
            Some(AppAction::ScrollToBottom)
        }
        
        // Help
        (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => Some(AppAction::ToggleHelp),
        
        // Refresh models
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some(AppAction::RefreshModels),
        
        // Clear error
        (KeyCode::Esc, _) => Some(AppAction::ClearError),
        
        _ => None,
    }
}

/// Handle keys in editing mode
fn handle_editing_mode(key: KeyEvent) -> Option<AppAction> {
    match (key.code, key.modifiers) {
        // Exit edit mode
        (KeyCode::Esc, _) => Some(AppAction::ExitEditMode),
        
        // Submit message
        (KeyCode::Enter, KeyModifiers::NONE) => Some(AppAction::SubmitMessage),
        
        // Character input
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            Some(AppAction::InsertChar(c))
        }
        
        // Deletion
        (KeyCode::Backspace, _) => Some(AppAction::DeleteChar),
        (KeyCode::Delete, _) => Some(AppAction::DeleteCharForward),
        (KeyCode::Char('h'), KeyModifiers::CONTROL) => Some(AppAction::DeleteChar),
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
            // Delete word - for now just clear all
            Some(AppAction::ClearInput)
        }
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => Some(AppAction::ClearInput),
        
        // Cursor movement
        (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
            Some(AppAction::MoveCursorLeft)
        }
        (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
            Some(AppAction::MoveCursorRight)
        }
        (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
            Some(AppAction::MoveCursorStart)
        }
        (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
            Some(AppAction::MoveCursorEnd)
        }
        
        _ => None,
    }
}

/// Handle keys in model selection mode
fn handle_model_select_mode(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(AppAction::CloseModelSelect),
        KeyCode::Enter => Some(AppAction::ConfirmModel),
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::PrevModel),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::NextModel),
        _ => None,
    }
}

/// Handle keys in session selection mode
fn handle_session_select_mode(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc => Some(AppAction::ExitEditMode),
        KeyCode::Enter => Some(AppAction::ExitEditMode),
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::PrevSession),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::NextSession),
        KeyCode::Char('n') => Some(AppAction::NewSession),
        KeyCode::Char('d') => Some(AppAction::DeleteSession),
        _ => None,
    }
}

/// Handle keys in help mode
fn handle_help_mode(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::F(1) => {
            Some(AppAction::ToggleHelp)
        }
        _ => None,
    }
}

/// Handle keys in delete confirmation mode
fn handle_delete_confirm_mode(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            Some(AppAction::ConfirmDeleteSession)
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            Some(AppAction::CancelDeleteSession)
        }
        _ => None,
    }
}

/// Process an action and update state
pub fn process_action(action: AppAction, state: &mut AppState) {
    // Clear transient error messages on most actions
    match &action {
        AppAction::ClearError => {}
        _ => state.clear_error(),
    }

    match action {
        // Navigation
        AppAction::NextSession => state.next_session(),
        AppAction::PrevSession => state.prev_session(),
        AppAction::NewSession => state.new_session(),
        AppAction::DeleteSession => state.delete_current_session(),
        AppAction::SelectSession(idx) => {
            if idx < state.sessions.len() {
                state.active_session_idx = idx;
                state.chat_scroll = 0;
            }
        }
        AppAction::RequestDeleteSession => {
            // Check if we can delete (not the last session, not streaming)
            if state.sessions.len() <= 1 {
                state.set_error("Cannot delete the last remaining session");
            } else if state.streaming {
                state.set_error("Cannot delete session while receiving response");
            } else {
                state.input_mode = InputMode::DeleteConfirm;
            }
        }
        AppAction::ConfirmDeleteSession => {
            let session_name = state.active_session()
                .map(|s| s.name.clone())
                .unwrap_or_default();
            
            state.delete_current_session();
            info!("Session deleted: {}", session_name);
            state.set_status(format!("Session deleted: {}", session_name));
            state.input_mode = InputMode::Normal;
            
            // Save sessions after deletion
            if let Err(e) = persistence::save_sessions(&state.sessions) {
                warn!("Failed to save sessions after deletion: {}", e);
            }
        }
        AppAction::CancelDeleteSession => {
            state.input_mode = InputMode::Normal;
        }

        // Model selection
        AppAction::OpenModelSelect => {
            state.input_mode = InputMode::ModelSelect;
            // Try to select current model in the list
            if let Some(current) = state.active_session() {
                if let Some(idx) = state.models.iter().position(|m| m.name == current.model) {
                    state.selected_model_idx = idx;
                }
            }
        }
        AppAction::CloseModelSelect => {
            state.input_mode = InputMode::Normal;
        }
        AppAction::NextModel => state.next_model(),
        AppAction::PrevModel => state.prev_model(),
        AppAction::ConfirmModel => {
            if let Some(model) = state.selected_model() {
                let model_name = model.name.clone();
                state.set_model(&model_name);
                state.set_status(format!("Switched to model: {}", model_name));
            }
            state.input_mode = InputMode::Normal;
        }
        AppAction::SelectModel(idx) => {
            if idx < state.models.len() {
                state.selected_model_idx = idx;
            }
        }

        // Input
        AppAction::EnterEditMode => {
            state.input_mode = InputMode::Editing;
        }
        AppAction::ExitEditMode => {
            state.input_mode = InputMode::Normal;
        }
        AppAction::SubmitMessage => {
            // Don't submit empty messages or while streaming
            if !state.input.trim().is_empty() && !state.streaming {
                // Message submission is handled by the main loop
                // This action just signals intent
            }
        }
        AppAction::InsertChar(c) => state.insert_char(c),
        AppAction::DeleteChar => state.delete_char(),
        AppAction::DeleteCharForward => state.delete_char_forward(),
        AppAction::MoveCursorLeft => state.move_cursor_left(),
        AppAction::MoveCursorRight => state.move_cursor_right(),
        AppAction::MoveCursorStart => state.move_cursor_start(),
        AppAction::MoveCursorEnd => state.move_cursor_end(),
        AppAction::ClearInput => state.clear_input(),

        // Scrolling
        AppAction::ScrollUp(n) => state.scroll_up(n),
        AppAction::ScrollDown(n) => state.scroll_down(n),
        AppAction::ScrollToTop => {
            // Set to max value to show oldest messages
            state.chat_scroll = usize::MAX / 2;
        }
        AppAction::ScrollToBottom => state.scroll_to_bottom(),
        AppAction::PageUp => state.scroll_up(10),
        AppAction::PageDown => state.scroll_down(10),

        // Misc
        AppAction::ToggleHelp => {
            state.input_mode = if state.input_mode == InputMode::Help {
                InputMode::Normal
            } else {
                InputMode::Help
            };
        }
        AppAction::ClearError => state.clear_error(),
        AppAction::Quit => state.should_quit = true,

        // Server actions are handled by the main loop
        AppAction::RefreshModels => {
            state.set_status("Refreshing models...");
        }
    }
}

/// Get help text for keybindings
pub fn get_help_text() -> Vec<(&'static str, &'static str)> {
    vec![
        ("General", ""),
        ("  q / Ctrl+c", "Quit"),
        ("  ?", "Toggle help"),
        ("  Ctrl+r", "Refresh models"),
        ("", ""),
        ("Navigation", ""),
        ("  Tab", "Next session"),
        ("  Shift+Tab", "Previous session"),
        ("  Ctrl+n", "New session"),
        ("  Ctrl+w", "Delete session"),
        ("  m", "Select model"),
        ("", ""),
        ("Chat", ""),
        ("  i / Enter", "Start typing"),
        ("  Esc", "Stop typing"),
        ("  Enter", "Send message (while typing)"),
        ("", ""),
        ("Scrolling", ""),
        ("  j/k or ↑/↓", "Scroll up/down"),
        ("  Ctrl+u/d", "Page up/down"),
        ("  g / G", "Top / Bottom"),
        ("", ""),
        ("Input Editing", ""),
        ("  Ctrl+a/e", "Start/end of line"),
        ("  Ctrl+u", "Clear input"),
        ("  Ctrl+w", "Delete word"),
    ]
}

// ============================================================================
// Mouse Event Handling
// ============================================================================

/// Map a mouse event to an application action based on current mode and UI layout
pub fn handle_mouse_event(
    mouse: MouseEvent,
    state: &AppState,
    layout: &AppLayout,
) -> Option<AppAction> {
    let x = mouse.column;
    let y = mouse.row;

    match mouse.kind {
        // Left click
        MouseEventKind::Down(MouseButton::Left) => {
            handle_mouse_click(x, y, state, layout)
        }
        
        // Scroll wheel (anywhere in the window scrolls chat)
        MouseEventKind::ScrollUp => {
            // Only scroll in normal or editing mode, not in popups
            match state.input_mode {
                InputMode::Normal | InputMode::Editing => Some(AppAction::ScrollUp(3)),
                InputMode::ModelSelect => Some(AppAction::PrevModel),
                _ => None,
            }
        }
        MouseEventKind::ScrollDown => {
            match state.input_mode {
                InputMode::Normal | InputMode::Editing => Some(AppAction::ScrollDown(3)),
                InputMode::ModelSelect => Some(AppAction::NextModel),
                _ => None,
            }
        }
        
        _ => None,
    }
}

/// Handle a left mouse click based on position
fn handle_mouse_click(
    x: u16,
    y: u16,
    state: &AppState,
    layout: &AppLayout,
) -> Option<AppAction> {
    // Handle popup modes first (they overlay the main UI)
    match state.input_mode {
        InputMode::Help => {
            // Any click dismisses help
            return Some(AppAction::ToggleHelp);
        }
        InputMode::DeleteConfirm => {
            // For delete confirmation, any click outside could cancel
            // We keep it simple: clicking anywhere cancels
            return Some(AppAction::CancelDeleteSession);
        }
        InputMode::ModelSelect => {
            // Clicking outside the popup closes it
            // The popup is centered, so we'd need popup bounds
            // For now, let clicks through or close on edge
            // TODO: Implement proper popup hit-testing
            return Some(AppAction::CloseModelSelect);
        }
        _ => {}
    }

    // Check if click is in sidebar (sessions list area at top of sidebar)
    if contains(layout.sidebar, x, y) {
        return handle_sidebar_click(x, y, state, layout);
    }
    
    // Check if click is in input area
    if contains(layout.input, x, y) {
        // Enter editing mode when clicking input
        if state.input_mode != InputMode::Editing {
            return Some(AppAction::EnterEditMode);
        }
        return None;
    }
    
    // Check if click is in chat area
    if contains(layout.chat, x, y) {
        // Clicking in chat in normal mode does nothing special for now
        // Future: could scroll to clicked message or select text
        return None;
    }
    
    None
}

/// Handle clicks within the sidebar area
fn handle_sidebar_click(
    _x: u16,
    y: u16,
    state: &AppState,
    layout: &AppLayout,
) -> Option<AppAction> {
    // The sidebar is split into two parts:
    // - Sessions list (top, takes most space)
    // - Model info box (bottom, 5 lines)
    
    // Model info box is at the bottom 5 lines of sidebar
    let model_box_height = 5u16;
    let model_box_y = layout.sidebar.y + layout.sidebar.height.saturating_sub(model_box_height);
    
    // Check if click is in model info box
    if y >= model_box_y {
        // Clicking model box opens model selector
        return Some(AppAction::OpenModelSelect);
    }
    
    // Otherwise, click is in sessions list
    // Sessions list has a border, so actual items start at y+1
    let list_area_y = layout.sidebar.y + 1; // After top border
    let list_area_height = layout.sidebar.height.saturating_sub(model_box_height + 2); // Minus borders and model box
    
    if y >= list_area_y && y < list_area_y + list_area_height {
        let clicked_idx = (y - list_area_y) as usize;
        
        if clicked_idx < state.sessions.len() {
            return Some(AppAction::SelectSession(clicked_idx));
        }
    }
    
    None
}

/// Helper: check if (x, y) is within a Rect
fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width &&
    y >= rect.y && y < rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_normal_mode_quit() {
        let config = Config::default();
        let state = AppState::new(config);
        
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action = handle_key_event(key, &state);
        
        assert!(matches!(action, Some(AppAction::Quit)));
    }

    #[test]
    fn test_edit_mode_escape() {
        let config = Config::default();
        let mut state = AppState::new(config);
        state.input_mode = InputMode::Editing;
        
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key_event(key, &state);
        
        assert!(matches!(action, Some(AppAction::ExitEditMode)));
    }

    #[test]
    fn test_ctrl_c_always_quits() {
        let config = Config::default();
        let mut state = AppState::new(config);
        state.input_mode = InputMode::Editing;
        
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = handle_key_event(key, &state);
        
        assert!(matches!(action, Some(AppAction::Quit)));
    }
}
