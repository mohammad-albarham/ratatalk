//! Chat area rendering
//!
//! Renders the chat history with proper styling for different message types.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{AppState, InputMode, Message};
use crate::ollama::Role;

use super::{colors, styles};

/// Render the chat history area
pub fn render_chat(frame: &mut Frame, state: &AppState, area: Rect) {
    let is_focused = state.input_mode == InputMode::Normal;

    let border_style = if is_focused {
        styles::border_focused()
    } else {
        styles::border_normal()
    };

    let title = if state.streaming {
        " Chat (streaming...) "
    } else {
        " Chat "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Get messages from active session
    let messages = state
        .active_session()
        .map(|s| &s.messages[..])
        .unwrap_or(&[]);

    if messages.is_empty() {
        // Show placeholder text
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No messages yet. Press 'i' or Enter to start typing.",
                styles::dim(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press '?' for help, 'm' to select model.",
                styles::dim(),
            )),
        ]);
        frame.render_widget(placeholder, inner_area);
        return;
    }

    // Build text lines from messages
    let lines = build_chat_lines(messages, inner_area.width.saturating_sub(2) as usize);

    // Calculate scroll
    let total_lines = lines.len();
    let visible_lines = inner_area.height as usize;

    // scroll_offset of 0 means show most recent (bottom)
    // We need to calculate the starting line
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let effective_scroll = state.chat_scroll.min(max_scroll);

    // Show from (total - visible - scroll) to (total - scroll)
    let start_line = total_lines.saturating_sub(visible_lines + effective_scroll);

    let visible_text: Vec<Line> = lines
        .into_iter()
        .skip(start_line)
        .take(visible_lines)
        .collect();

    let paragraph = Paragraph::new(visible_text);
    frame.render_widget(paragraph, inner_area);

    // Show scroll indicator if needed
    if max_scroll > 0 {
        let scroll_indicator = if effective_scroll > 0 {
            format!("↑{}", effective_scroll)
        } else {
            String::new()
        };

        if !scroll_indicator.is_empty() {
            let indicator_area = Rect {
                x: area.x + area.width - scroll_indicator.len() as u16 - 2,
                y: area.y,
                width: scroll_indicator.len() as u16 + 1,
                height: 1,
            };
            let indicator = Paragraph::new(scroll_indicator).style(styles::dim());
            frame.render_widget(indicator, indicator_area);
        }
    }
}

/// Build text lines from messages with proper formatting
fn build_chat_lines(messages: &[Message], max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for (idx, message) in messages.iter().enumerate() {
        // Add separator between messages (except first)
        if idx > 0 {
            lines.push(Line::from(""));
        }

        // Role indicator and styling
        let (role_prefix, mut role_style, mut content_style) = match message.role {
            Role::User => (
                "You",
                Style::default()
                    .fg(colors::USER_MSG)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(colors::USER_MSG),
            ),
            Role::Assistant => (
                "Assistant",
                Style::default()
                    .fg(colors::ASSISTANT_MSG)
                    .add_modifier(Modifier::BOLD),
                if message.streaming {
                    styles::streaming()
                } else {
                    Style::default().fg(colors::ASSISTANT_MSG)
                },
            ),
            Role::System => (
                "System",
                Style::default()
                    .fg(colors::SYSTEM_MSG)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(colors::SYSTEM_MSG),
            ),
        };

        // Stream errors are rendered as assistant messages with a fixed marker.
        if message.role == Role::Assistant && message.content.starts_with("[Error:") {
            role_style = styles::error().add_modifier(Modifier::BOLD);
            content_style = styles::error();
        }

        // Header line with role and optional timestamp
        let timestamp = message.timestamp.format("%H:%M").to_string();
        lines.push(Line::from(vec![
            Span::styled(format!("{}:", role_prefix), role_style),
            Span::raw(" "),
            Span::styled(timestamp, styles::dim()),
            if message.streaming {
                Span::styled(" ⣾", styles::streaming())
            } else {
                Span::raw("")
            },
        ]));

        // Content lines (markdown rendered)
        let markdown_lines = super::markdown::parse_markdown(
            &message.content,
            max_width.saturating_sub(2),
            content_style,
        );
        for line in markdown_lines {
            let mut spans = line.spans;
            spans.insert(0, Span::raw("  ")); // Indent content
            lines.push(Line::from(spans));
        }
    }

    lines
}
