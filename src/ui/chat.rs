//! Chat area rendering
//!
//! Renders the chat history with proper styling for different message types.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
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
        let (role_prefix, role_style, content_style) = match message.role {
            Role::User => (
                "You",
                Style::default().fg(colors::USER_MSG).add_modifier(Modifier::BOLD),
                Style::default().fg(colors::USER_MSG),
            ),
            Role::Assistant => (
                "Assistant",
                Style::default().fg(colors::ASSISTANT_MSG).add_modifier(Modifier::BOLD),
                if message.streaming {
                    styles::streaming()
                } else {
                    Style::default().fg(colors::ASSISTANT_MSG)
                },
            ),
            Role::System => (
                "System",
                Style::default().fg(colors::SYSTEM_MSG).add_modifier(Modifier::BOLD),
                Style::default().fg(colors::SYSTEM_MSG),
            ),
        };

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

        // Content lines (word-wrapped)
        let content_lines = wrap_text(&message.content, max_width);
        for content_line in content_lines {
            lines.push(Line::from(vec![
                Span::raw("  "), // Indent content
                Span::styled(content_line, content_style),
            ]));
        }
    }

    lines
}

/// Simple word wrapping
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                if word.len() > max_width {
                    // Word is too long, split it
                    for chunk in word.chars().collect::<Vec<_>>().chunks(max_width) {
                        lines.push(chunk.iter().collect());
                    }
                } else {
                    current_line = word.to_string();
                }
            } else if current_line.len() + 1 + word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_simple() {
        let result = wrap_text("hello world", 20);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_multiline() {
        let result = wrap_text("hello world this is a test", 10);
        assert_eq!(result, vec!["hello", "world this", "is a test"]);
    }

    #[test]
    fn test_wrap_text_newlines() {
        let result = wrap_text("line1\nline2", 20);
        assert_eq!(result, vec!["line1", "line2"]);
    }
}
