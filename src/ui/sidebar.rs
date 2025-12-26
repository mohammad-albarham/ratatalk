//! Sidebar rendering
//!
//! Renders the session list and model info in the sidebar.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;

use super::{colors, styles};

/// Render the sidebar
pub fn render_sidebar(frame: &mut Frame, state: &AppState, area: Rect) {
    // Split sidebar into sessions and model info
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Sessions list
            Constraint::Length(5), // Model info
        ])
        .split(area);

    render_sessions_list(frame, state, chunks[0]);
    render_model_info(frame, state, chunks[1]);
}

/// Render the sessions list
fn render_sessions_list(frame: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .title(" Sessions ")
        .borders(Borders::ALL)
        .border_style(styles::border_normal());

    let inner_area = block.inner(area);

    if state.sessions.is_empty() {
        let empty = Paragraph::new(Span::styled("No sessions", styles::dim()))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    // Build list items
    let items: Vec<ListItem> = state
        .sessions
        .iter()
        .enumerate()
        .map(|(idx, session)| {
            let is_selected = idx == state.active_session_idx;
            let is_streaming = session.is_streaming();
            
            // Session indicator
            let indicator = if is_streaming {
                "⣾"
            } else if is_selected {
                "▶"
            } else {
                " "
            };

            // Truncate name to fit
            let max_name_len = area.width.saturating_sub(6) as usize;
            let name = if session.name.len() > max_name_len {
                format!("{}…", &session.name[..max_name_len.saturating_sub(1)])
            } else {
                session.name.clone()
            };

            let style = if is_selected {
                styles::selected()
            } else {
                ratatui::style::Style::default()
            };

            let line = Line::from(vec![
                Span::raw(format!("{} ", indicator)),
                Span::styled(name, style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);

    // Show hint at bottom if there's space
    if inner_area.height > state.sessions.len() as u16 + 2 {
        let hint_y = area.y + area.height - 2;
        let hint = Paragraph::new(Span::styled("Ctrl+n: new", styles::dim()));
        frame.render_widget(
            hint,
            Rect {
                x: area.x + 1,
                y: hint_y,
                width: area.width - 2,
                height: 1,
            },
        );
    }
}

/// Render the model info box
fn render_model_info(frame: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .title(" Model ")
        .borders(Borders::ALL)
        .border_style(styles::border_normal());

    let inner_area = block.inner(area);

    // Current model name
    let model_name = state.current_model();
    let max_len = inner_area.width as usize;
    let display_name = if model_name.len() > max_len {
        format!("{}…", &model_name[..max_len.saturating_sub(1)])
    } else {
        model_name.to_string()
    };

    let lines = vec![
        Line::from(Span::styled(display_name, styles::highlight())),
        Line::from(""),
        Line::from(Span::styled("m: change", styles::dim())),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
