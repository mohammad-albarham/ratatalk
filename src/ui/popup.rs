//! Popup dialogs
//!
//! Modal dialogs for model selection, help, etc.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{AppState, InputMode};
use crate::events::get_help_text;

use super::{colors, styles};

/// Render the model selection popup
pub fn render_model_popup(frame: &mut Frame, state: &AppState) {
    if state.input_mode != InputMode::ModelSelect {
        return;
    }

    let area = centered_rect(60, 70, frame.area());
    
    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Model (↑/↓ to navigate, Enter to select, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_style(styles::border_focused());

    let _inner_area = block.inner(area);

    if state.models.is_empty() {
        let msg = if state.loading {
            "Loading models..."
        } else {
            "No models available. Is Ollama running?"
        };
        let paragraph = Paragraph::new(Span::styled(msg, styles::dim()))
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    // Build list items
    let items: Vec<ListItem> = state
        .models
        .iter()
        .enumerate()
        .map(|(idx, model)| {
            let is_selected = idx == state.selected_model_idx;
            let is_current = state.current_model() == model.name;

            let indicator = if is_selected {
                "▶"
            } else {
                " "
            };

            let current_marker = if is_current { " (current)" } else { "" };

            // Format size
            let size_str = format_size(model.size);

            let style = if is_selected {
                styles::selected()
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::raw(format!("{} ", indicator)),
                Span::styled(model.name.clone(), style),
                Span::styled(current_marker, styles::dim()),
                Span::raw(" "),
                Span::styled(format!("[{}]", size_str), styles::dim()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Render the help popup
pub fn render_help_popup(frame: &mut Frame, state: &AppState) {
    if state.input_mode != InputMode::Help {
        return;
    }

    let area = centered_rect(50, 80, frame.area());
    
    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help (press ? or Esc to close) ")
        .borders(Borders::ALL)
        .border_style(styles::border_focused());

    let help_items = get_help_text();
    
    let lines: Vec<Line> = help_items
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() && desc.is_empty() {
                Line::from("")
            } else if desc.is_empty() {
                // Section header
                Line::from(Span::styled(
                    *key,
                    Style::default().add_modifier(Modifier::BOLD).fg(colors::HIGHLIGHT),
                ))
            } else {
                Line::from(vec![
                    Span::styled(format!("{:<16}", key), styles::highlight()),
                    Span::raw(*desc),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    
    frame.render_widget(paragraph, area);
}

/// Render the delete confirmation popup
pub fn render_delete_confirm_popup(frame: &mut Frame, state: &AppState) {
    if state.input_mode != InputMode::DeleteConfirm {
        return;
    }

    let area = centered_rect(50, 40, frame.area());
    
    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Delete Session? ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::ERROR));

    let _inner_area = block.inner(area);

    // Get session details
    let (session_name, message_count) = state
        .active_session()
        .map(|s| (s.name.clone(), s.message_count()))
        .unwrap_or_else(|| ("Unknown".to_string(), 0));

    // Truncate long session names
    let display_name = if session_name.len() > 35 {
        format!("{}...", &session_name[..32])
    } else {
        session_name.clone()
    };

    // Build confirmation message
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Session: "),
            Span::styled(display_name, Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("Messages: "),
            Span::styled(message_count.to_string(), Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "⚠ This action cannot be undone.",
            Style::default().fg(colors::WARNING),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", styles::highlight()),
            Span::raw(" Yes, delete    "),
            Span::styled("[N]", styles::highlight()),
            Span::raw(" No, cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);
    
    frame.render_widget(paragraph, area);
}

/// Create a centered rect with percentage of parent
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Format file size in human-readable form
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1_500_000), "1.4 MB");
        assert_eq!(format_size(4_000_000_000), "3.7 GB");
    }
}
