//! Layout management
//!
//! Defines the main screen layout structure.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::app::AppState;

use super::{render_chat, render_input, render_sidebar};

/// Layout areas for the application
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppLayout {
    /// Main content area (chat + input)
    pub main: Rect,
    /// Chat history area
    pub chat: Rect,
    /// Input area
    pub input: Rect,
    /// Sidebar area
    pub sidebar: Rect,
    /// Status bar area
    pub status: Rect,
}

impl AppLayout {
    /// Calculate layout from terminal size
    pub fn new(area: Rect, sidebar_width: u16) -> Self {
        // First split: main content vs status bar
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),     // Main content
                Constraint::Length(1),  // Status bar
            ])
            .split(area);

        let main_area = vertical[0];
        let status = vertical[1];

        // Second split: main content vs sidebar
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),              // Main content
                Constraint::Length(sidebar_width), // Sidebar
            ])
            .split(main_area);

        let content_area = horizontal[0];
        let sidebar = horizontal[1];

        // Third split: chat area vs input area
        let content_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),     // Chat
                Constraint::Length(3), // Input (single line + borders)
            ])
            .split(content_area);

        let chat = content_vertical[0];
        let input = content_vertical[1];

        Self {
            main: main_area,
            chat,
            input,
            sidebar,
            status,
        }
    }
}

/// Render the main layout
pub fn render_layout(frame: &mut Frame, state: &AppState) {
    let layout = AppLayout::new(frame.area(), state.config.ui.sidebar_width);

    // Render each section
    render_chat(frame, state, layout.chat);
    render_input(frame, state, layout.input);
    render_sidebar(frame, state, layout.sidebar);
    render_status_bar(frame, state, layout.status);
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    use ratatui::{
        text::{Line, Span},
        widgets::Paragraph,
    };
    use super::{colors, styles};

    let mut spans = Vec::new();

    // Connection status
    let status_icon = if state.server_connected { "●" } else { "○" };
    let status_color = if state.server_connected {
        colors::SUCCESS
    } else {
        colors::ERROR
    };
    spans.push(Span::styled(
        format!(" {} ", status_icon),
        ratatui::style::Style::default().fg(status_color),
    ));

    // Current model
    spans.push(Span::styled(
        format!("[{}]", state.current_model()),
        styles::highlight(),
    ));
    spans.push(Span::raw(" "));

    // Session info
    if let Some(session) = state.active_session() {
        spans.push(Span::styled(
            format!("Session: {} ", session.name),
            styles::dim(),
        ));
        spans.push(Span::styled(
            format!("({} msgs)", session.message_count()),
            styles::dim(),
        ));
    }

    // Streaming indicator
    if state.streaming {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("⣾ Generating...", styles::streaming()));
    }

    // Stats from last response
    if let Some(stats) = &state.last_response_stats {
        if !state.streaming {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("[{:.1} tok/s]", stats.tokens_per_second),
                styles::dim(),
            ));
        }
    }

    // Status or error message (right-aligned conceptually, but we'll just append)
    if let Some(error) = &state.error_message {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(format!("⚠ {}", error), styles::error()));
    } else if let Some(status) = &state.status_message {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(status.clone(), styles::dim()));
    }

    // Mode indicator (far right)
    let mode_str = match state.input_mode {
        crate::app::InputMode::Normal => "NORMAL",
        crate::app::InputMode::Editing => "INSERT",
        crate::app::InputMode::ModelSelect => "MODEL",
        crate::app::InputMode::SessionSelect => "SESSION",
        crate::app::InputMode::Help => "HELP",
        crate::app::InputMode::DeleteConfirm => "DELETE?",
    };
    
    // Calculate padding to right-align mode
    let content_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let padding = area.width.saturating_sub(content_width as u16 + mode_str.len() as u16 + 2) as usize;
    
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }
    spans.push(Span::styled(
        format!(" {} ", mode_str),
        styles::status_bar(),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    
    frame.render_widget(paragraph, area);
}
