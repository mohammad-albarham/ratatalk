//! Input box rendering
//!
//! Renders the text input area with cursor.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{AppState, InputMode};

use super::styles;

/// Render the input area
pub fn render_input(frame: &mut Frame, state: &AppState, area: Rect) {
    let is_editing = state.input_mode == InputMode::Editing;
    
    let border_style = if is_editing {
        styles::border_active()
    } else {
        styles::border_normal()
    };

    let title = if is_editing {
        " Input (Enter to send, Esc to cancel) "
    } else if state.streaming {
        " Input (waiting for response...) "
    } else {
        " Input (i or Enter to type) "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);

    // Build input line with cursor
    let input_text = if is_editing {
        // Show cursor
        let (before, after) = state.input.split_at(
            state.cursor_position.min(state.input.len())
        );
        
        Line::from(vec![
            Span::raw(before),
            Span::styled("█", styles::highlight()), // Block cursor
            Span::raw(after),
        ])
    } else if state.input.is_empty() {
        Line::from(Span::styled(
            "Press 'i' or Enter to start typing...",
            styles::dim(),
        ))
    } else {
        Line::from(state.input.clone())
    };

    let paragraph = Paragraph::new(input_text).block(block);
    
    frame.render_widget(paragraph, area);

    // Set cursor position for terminal cursor if editing
    if is_editing {
        // Calculate cursor position within the visible area
        let cursor_x = inner_area.x + state.cursor_position.min(inner_area.width as usize) as u16;
        let cursor_y = inner_area.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
