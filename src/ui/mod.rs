//! UI module
//!
//! All ratatui rendering logic for ratatalk.

mod chat;
mod input;
mod layout;
mod popup;
mod sidebar;

pub use chat::render_chat;
pub use input::render_input;
pub use layout::{render_layout, AppLayout};
pub use popup::{render_help_popup, render_model_popup, render_delete_confirm_popup};
pub use sidebar::render_sidebar;

use ratatui::style::{Color, Modifier, Style};

/// Color scheme for the application
pub mod colors {
    use super::*;

    #[allow(dead_code)]
    pub const BG: Color = Color::Reset;
    #[allow(dead_code)]
    pub const FG: Color = Color::Reset;
    
    pub const BORDER: Color = Color::DarkGray;
    pub const BORDER_FOCUSED: Color = Color::Cyan;
    pub const BORDER_ACTIVE: Color = Color::Green;
    
    pub const USER_MSG: Color = Color::Cyan;
    pub const ASSISTANT_MSG: Color = Color::Green;
    pub const SYSTEM_MSG: Color = Color::Yellow;
    
    pub const ERROR: Color = Color::Red;
    pub const WARNING: Color = Color::Yellow;
    pub const SUCCESS: Color = Color::Green;
    #[allow(dead_code)]
    pub const INFO: Color = Color::Blue;
    
    pub const SELECTED: Color = Color::Yellow;
    pub const HIGHLIGHT: Color = Color::Cyan;
    
    pub const STATUS_BG: Color = Color::DarkGray;
    pub const STATUS_FG: Color = Color::White;
}

/// Common styles
pub mod styles {
    use super::*;

    pub fn border_normal() -> Style {
        Style::default().fg(colors::BORDER)
    }

    pub fn border_focused() -> Style {
        Style::default().fg(colors::BORDER_FOCUSED)
    }

    pub fn border_active() -> Style {
        Style::default().fg(colors::BORDER_ACTIVE)
    }

    #[allow(dead_code)]
    pub fn user_message() -> Style {
        Style::default().fg(colors::USER_MSG)
    }

    #[allow(dead_code)]
    pub fn assistant_message() -> Style {
        Style::default().fg(colors::ASSISTANT_MSG)
    }

    #[allow(dead_code)]
    pub fn system_message() -> Style {
        Style::default().fg(colors::SYSTEM_MSG)
    }

    pub fn error() -> Style {
        Style::default().fg(colors::ERROR)
    }

    pub fn selected() -> Style {
        Style::default()
            .fg(colors::SELECTED)
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlight() -> Style {
        Style::default()
            .fg(colors::HIGHLIGHT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_bar() -> Style {
        Style::default()
            .bg(colors::STATUS_BG)
            .fg(colors::STATUS_FG)
    }

    pub fn dim() -> Style {
        Style::default().add_modifier(Modifier::DIM)
    }

    #[allow(dead_code)]
    pub fn bold() -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    pub fn streaming() -> Style {
        Style::default()
            .fg(colors::ASSISTANT_MSG)
            .add_modifier(Modifier::DIM)
    }
}
