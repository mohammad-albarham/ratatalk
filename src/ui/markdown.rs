use std::mem;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Parse markdown text into ratatui Lines
pub fn parse_markdown(text: &str, max_width: usize, base_style: Style) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(text, options);
    let mut raw_lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut current_style = base_style;
    let mut style_stack = Vec::new();

    let mut in_code_block = false;
    let mut blockquote_depth = 0usize;
    let mut list_stack = Vec::new();
    let mut in_list_item = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                style_stack.push(current_style);
                match tag {
                    Tag::Strong => {
                        current_style = current_style.add_modifier(Modifier::BOLD);
                    }
                    Tag::Emphasis => {
                        current_style = current_style.add_modifier(Modifier::ITALIC);
                    }
                    Tag::Strikethrough => {
                        current_style = current_style.add_modifier(Modifier::CROSSED_OUT);
                    }
                    Tag::Heading { level, .. } => {
                        push_current_line(&mut raw_lines, &mut current_line_spans);
                        if !raw_lines.is_empty() {
                            push_blank_line(&mut raw_lines);
                        }
                        current_style =
                            current_style.add_modifier(Modifier::BOLD).fg(match level {
                                pulldown_cmark::HeadingLevel::H1 => Color::Magenta,
                                pulldown_cmark::HeadingLevel::H2 => Color::Blue,
                                pulldown_cmark::HeadingLevel::H3 => Color::Cyan,
                                _ => Color::Yellow,
                            });
                    }
                    Tag::CodeBlock(_) => {
                        in_code_block = true;
                        current_style = current_style.fg(Color::Yellow);
                        push_current_line(&mut raw_lines, &mut current_line_spans);
                    }
                    Tag::List(start) => {
                        list_stack.push(start);
                    }
                    Tag::Item => {
                        push_current_line(&mut raw_lines, &mut current_line_spans);
                        in_list_item = true;
                        ensure_blockquote_prefix(
                            &mut current_line_spans,
                            current_style,
                            blockquote_depth,
                        );

                        let prefix = if let Some(Some(next_number)) = list_stack.last_mut() {
                            let prefix = format!("{}. ", *next_number);
                            *next_number += 1;
                            prefix
                        } else {
                            "• ".to_string()
                        };
                        current_line_spans
                            .push(Span::styled(prefix, Style::default().fg(Color::Yellow)));
                    }
                    Tag::Link { .. } => {
                        current_style = current_style
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED);
                    }
                    Tag::Paragraph => {
                        if !current_line_spans.is_empty() && !in_list_item {
                            push_current_line(&mut raw_lines, &mut current_line_spans);
                        }

                        if !raw_lines.is_empty() && !in_list_item {
                            push_blank_line(&mut raw_lines);
                        }
                    }
                    Tag::BlockQuote(_) => {
                        push_current_line(&mut raw_lines, &mut current_line_spans);
                        blockquote_depth += 1;
                        current_style = current_style
                            .add_modifier(Modifier::ITALIC)
                            .fg(Color::DarkGray);
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                if let Some(prev_style) = style_stack.pop() {
                    match tag {
                        TagEnd::Heading(_) => {
                            push_current_line(&mut raw_lines, &mut current_line_spans);
                            push_blank_line(&mut raw_lines);
                        }
                        TagEnd::CodeBlock => {
                            in_code_block = false;
                        }
                        TagEnd::List(_) => {
                            list_stack.pop();
                        }
                        TagEnd::Item => {
                            in_list_item = false;
                            push_current_line(&mut raw_lines, &mut current_line_spans);
                        }
                        TagEnd::BlockQuote(_) => {
                            push_current_line(&mut raw_lines, &mut current_line_spans);
                            if blockquote_depth > 0 {
                                blockquote_depth -= 1;
                            }
                        }
                        TagEnd::Paragraph => {
                            push_current_line(&mut raw_lines, &mut current_line_spans);
                        }
                        _ => {}
                    }
                    current_style = prev_style;
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    for line in text.split('\n') {
                        raw_lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            current_style,
                        )));
                    }
                } else {
                    for (idx, segment) in text.split('\n').enumerate() {
                        if idx > 0 {
                            push_current_line(&mut raw_lines, &mut current_line_spans);
                        }

                        ensure_blockquote_prefix(
                            &mut current_line_spans,
                            current_style,
                            blockquote_depth,
                        );

                        if !segment.is_empty() {
                            current_line_spans
                                .push(Span::styled(segment.to_string(), current_style));
                        }
                    }
                }
            }
            Event::Code(text) => {
                ensure_blockquote_prefix(&mut current_line_spans, current_style, blockquote_depth);
                current_line_spans.push(Span::styled(
                    text.to_string(),
                    Style::default()
                        .bg(Color::Rgb(40, 44, 52))
                        .fg(Color::Yellow),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                push_current_line(&mut raw_lines, &mut current_line_spans);
            }
            _ => {}
        }
    }

    push_current_line(&mut raw_lines, &mut current_line_spans);
    let mut lines = wrap_lines(raw_lines, max_width);

    // Fallback if empty
    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

fn ensure_blockquote_prefix(
    current_line_spans: &mut Vec<Span<'static>>,
    current_style: Style,
    blockquote_depth: usize,
) {
    if blockquote_depth > 0 && current_line_spans.is_empty() {
        current_line_spans.push(Span::styled("> ".repeat(blockquote_depth), current_style));
    }
}

fn push_current_line(lines: &mut Vec<Line<'static>>, current_line_spans: &mut Vec<Span<'static>>) {
    if !current_line_spans.is_empty() {
        lines.push(Line::from(mem::take(current_line_spans)));
    }
}

fn push_blank_line(lines: &mut Vec<Line<'static>>) {
    if !matches!(lines.last(), Some(line) if is_blank_line(line)) {
        lines.push(Line::from(""));
    }
}

fn is_blank_line(line: &Line<'_>) -> bool {
    line.spans.is_empty() || (line.spans.len() == 1 && line.spans[0].content.is_empty())
}

fn wrap_lines(lines: Vec<Line<'static>>, max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 {
        return lines;
    }

    let mut wrapped_lines = Vec::new();

    for line in lines {
        if is_blank_line(&line) {
            wrapped_lines.push(line);
            continue;
        }

        let mut current_line_spans = Vec::new();
        let mut current_width = 0usize;

        for span in line.spans {
            let style = span.style;
            let content = span.content.to_string();

            for token in tokenize_text(&content) {
                if token.is_empty() {
                    continue;
                }

                let token_width = token.width();
                let is_whitespace = token.chars().all(|ch| ch.is_whitespace());

                if is_whitespace {
                    if current_line_spans.is_empty() {
                        continue;
                    }

                    if current_width + token_width > max_width {
                        trim_trailing_whitespace(&mut current_line_spans);
                        push_current_line(&mut wrapped_lines, &mut current_line_spans);
                        current_width = 0;
                    } else {
                        current_line_spans.push(Span::styled(token.to_string(), style));
                        current_width += token_width;
                    }

                    continue;
                }

                if current_width > 0 && current_width + token_width > max_width {
                    trim_trailing_whitespace(&mut current_line_spans);
                    push_current_line(&mut wrapped_lines, &mut current_line_spans);
                    current_width = 0;
                }

                if token_width > max_width {
                    for chunk in split_by_width(token, max_width) {
                        let chunk_width = chunk.width();

                        if current_width > 0 && current_width + chunk_width > max_width {
                            push_current_line(&mut wrapped_lines, &mut current_line_spans);
                            current_width = 0;
                        }

                        current_line_spans.push(Span::styled(chunk, style));
                        current_width += chunk_width;

                        if current_width >= max_width {
                            push_current_line(&mut wrapped_lines, &mut current_line_spans);
                            current_width = 0;
                        }
                    }
                } else {
                    current_line_spans.push(Span::styled(token.to_string(), style));
                    current_width += token_width;
                }
            }
        }

        trim_trailing_whitespace(&mut current_line_spans);
        push_current_line(&mut wrapped_lines, &mut current_line_spans);
    }

    wrapped_lines
}

fn trim_trailing_whitespace(current_line_spans: &mut Vec<Span<'static>>) {
    while let Some(last_span) = current_line_spans.last_mut() {
        let trimmed = last_span.content.trim_end_matches(char::is_whitespace);

        if trimmed.len() == last_span.content.len() {
            break;
        }

        if trimmed.is_empty() {
            current_line_spans.pop();
        } else {
            last_span.content = trimmed.to_string().into();
            break;
        }
    }
}

fn tokenize_text(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut current_is_whitespace = None;

    for (idx, ch) in text.char_indices() {
        let is_whitespace = ch.is_whitespace();

        match current_is_whitespace {
            Some(prev) if prev != is_whitespace => {
                tokens.push(&text[start..idx]);
                start = idx;
            }
            None => {
                start = idx;
            }
            _ => {}
        }

        current_is_whitespace = Some(is_whitespace);
    }

    if current_is_whitespace.is_some() {
        tokens.push(&text[start..]);
    }

    tokens
}

fn split_by_width(text: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);

        if current_width > 0 && current_width + ch_width > max_width {
            chunks.push(mem::take(&mut current));
            current_width = 0;
        }

        current.push(ch);
        current_width += ch_width;

        if current_width >= max_width {
            chunks.push(mem::take(&mut current));
            current_width = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered_text(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn test_parse_markdown_simple() {
        let lines = parse_markdown("Hello **world**", 20, Style::default());
        assert!(!lines.is_empty());
        assert_eq!(rendered_text(&lines), vec!["Hello world"]);
    }

    #[test]
    fn test_parse_markdown_heading() {
        let lines = parse_markdown("# Heading", 20, Style::default());
        assert!(lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("Heading"))));
    }

    #[test]
    fn test_parse_markdown_list() {
        let lines = parse_markdown("- Item 1\n- Item 2", 20, Style::default());
        assert_eq!(rendered_text(&lines), vec!["• Item 1", "• Item 2"]);
    }

    #[test]
    fn test_parse_markdown_wraps_plain_text() {
        let lines = parse_markdown("hello world this is a test", 10, Style::default());
        assert_eq!(
            rendered_text(&lines),
            vec!["hello", "world this", "is a test"]
        );
    }

    #[test]
    fn test_parse_markdown_resets_list_numbering() {
        let lines = parse_markdown("1. One\n2. Two\n\n- Three", 20, Style::default());
        assert_eq!(rendered_text(&lines), vec!["1. One", "2. Two", "• Three"]);
    }
}
