//! Styled markdown rendering for terminal display
//!
//! This module parses markdown-like text and converts it to styled ratatui Lines
//! with proper colors for headers, links, code blocks, etc.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use regex::Regex;

use super::{AZUL_BLUE, TOKYO_BLUE, TOKYO_COMMENT, TOKYO_GREEN, TOKYO_ORANGE, TOKYO_PURPLE, TOKYO_TEXT};

// Warm Paper DARK accent colors for visual hierarchy
const HEADER_ACCENT: Color = Color::Rgb(220, 170, 90);   // Amber for headers
const LINK_COLOR: Color = Color::Rgb(120, 155, 185);     // Warm blue for links

/// Represents a parsed markdown element
#[derive(Debug, Clone)]
pub enum MarkdownElement {
    Header1(String),
    Header2(String),
    Header3(String),
    Paragraph(String),
    CodeBlock(String),
    InlineCode(String),
    Link { text: String, url: String },
    ListItem(String),
    Quote(String),
    HorizontalRule,
    Empty,
}

/// Styled markdown renderer
pub struct StyledMarkdown {
    width: usize,
}

impl StyledMarkdown {
    pub fn new(width: usize) -> Self {
        Self { width: width.max(20) }
    }

    /// Render a list of content lines into styled Lines
    pub fn render(&self, lines: &[String]) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        let mut in_code_block = false;
        let mut code_buffer = Vec::new();

        for line in lines {
            // Handle code block delimiters
            if line.trim().starts_with("```") {
                if in_code_block {
                    // End code block
                    result.extend(self.render_code_block(&code_buffer));
                    code_buffer.clear();
                    in_code_block = false;
                } else {
                    // Start code block
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_buffer.push(line.clone());
                continue;
            }

            // Parse and render the line
            result.extend(self.render_line(line));
        }

        // Handle unclosed code block
        if !code_buffer.is_empty() {
            result.extend(self.render_code_block(&code_buffer));
        }

        result
    }

    /// Render a single line
    fn render_line(&self, line: &str) -> Vec<Line<'static>> {
        let trimmed = line.trim();

        // Empty line
        if trimmed.is_empty() {
            return vec![Line::from("")];
        }

        // Headers (check longest prefix first)
        if trimmed.starts_with("#### ") {
            return vec![self.render_header4(&trimmed[5..])];
        }
        if trimmed.starts_with("### ") {
            return vec![self.render_header3(&trimmed[4..])];
        }
        if trimmed.starts_with("## ") {
            return vec![Line::from(""), self.render_header2(&trimmed[3..])];
        }
        if trimmed.starts_with("# ") {
            return vec![Line::from(""), self.render_header1(&trimmed[2..]), Line::from("")];
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            return vec![self.render_hr()];
        }

        // Quote
        if trimmed.starts_with("> ") {
            return vec![self.render_quote(&trimmed[2..])];
        }

        // Nested list items (indented with spaces/tabs)
        let indent = line.len() - line.trim_start().len();
        let indent_level = indent / 2;  // 2 spaces per level

        // List items
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            return vec![self.render_list_item(&trimmed[2..], indent_level)];
        }

        // Numbered list
        if let Some(captures) = Regex::new(r"^(\d+)\.\s+(.*)$").ok().and_then(|re| re.captures(trimmed)) {
            if let (Some(num), Some(text)) = (captures.get(1), captures.get(2)) {
                return vec![self.render_numbered_item(num.as_str(), text.as_str(), indent_level)];
            }
        }

        // Task list items: - [ ] or - [x]
        if trimmed.starts_with("- [ ] ") {
            return vec![self.render_task_item(&trimmed[6..], false, indent_level)];
        }
        if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
            return vec![self.render_task_item(&trimmed[6..], true, indent_level)];
        }

        // Regular paragraph with inline formatting
        vec![self.render_paragraph(line)]
    }

    /// Render H1 header - large, prominent
    fn render_header1(&self, text: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("━━ ", Style::default().fg(HEADER_ACCENT)),
            Span::styled(
                text.to_uppercase(),
                Style::default()
                    .fg(HEADER_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(
                "━".repeat((self.width.saturating_sub(text.len() + 5)).min(40)),
                Style::default().fg(HEADER_ACCENT),
            ),
        ])
    }

    /// Render H2 header - section headers
    fn render_header2(&self, text: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("◆ ", Style::default().fg(TOKYO_PURPLE)),
            Span::styled(
                text.to_string(),
                Style::default()
                    .fg(TOKYO_PURPLE)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    /// Render H3 header - subsection
    fn render_header3(&self, text: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("  ▸ ", Style::default().fg(TOKYO_ORANGE)),
            Span::styled(
                text.to_string(),
                Style::default()
                    .fg(TOKYO_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    /// Render H4 header - smaller subsection
    fn render_header4(&self, text: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("    ◦ ", Style::default().fg(TOKYO_COMMENT)),
            Span::styled(
                text.to_string(),
                Style::default()
                    .fg(TOKYO_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    /// Render horizontal rule
    fn render_hr(&self) -> Line<'static> {
        let rule = "─".repeat(self.width.saturating_sub(4));
        Line::from(Span::styled(
            rule,
            Style::default().fg(TOKYO_COMMENT),
        ))
    }

    /// Render blockquote
    fn render_quote(&self, text: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(TOKYO_BLUE)),
            Span::styled(
                text.to_string(),
                Style::default()
                    .fg(TOKYO_COMMENT)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    }

    /// Render list item with indentation support
    fn render_list_item(&self, text: &str, indent_level: usize) -> Line<'static> {
        let indent = "  ".repeat(indent_level);
        let bullet = match indent_level {
            0 => "•",
            1 => "◦",
            2 => "▪",
            _ => "·",
        };
        let bullet_color = match indent_level {
            0 => TOKYO_GREEN,
            1 => TOKYO_BLUE,
            _ => TOKYO_COMMENT,
        };

        let mut spans = vec![
            Span::styled(format!("{}  {} ", indent, bullet), Style::default().fg(bullet_color)),
        ];
        spans.extend(self.render_inline_spans(text));
        Line::from(spans)
    }

    /// Render numbered list item with indentation
    fn render_numbered_item(&self, num: &str, text: &str, indent_level: usize) -> Line<'static> {
        let indent = "  ".repeat(indent_level);
        let num_color = match indent_level {
            0 => TOKYO_ORANGE,
            _ => TOKYO_COMMENT,
        };

        let mut spans = vec![
            Span::styled(format!("{} {}. ", indent, num), Style::default().fg(num_color)),
        ];
        spans.extend(self.render_inline_spans(text));
        Line::from(spans)
    }

    /// Render task list item (checkbox)
    fn render_task_item(&self, text: &str, checked: bool, indent_level: usize) -> Line<'static> {
        let indent = "  ".repeat(indent_level);
        let (checkbox, style) = if checked {
            ("☑", Style::default().fg(TOKYO_GREEN))
        } else {
            ("☐", Style::default().fg(TOKYO_COMMENT))
        };

        let text_style = if checked {
            Style::default().fg(TOKYO_COMMENT).add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::default().fg(TOKYO_TEXT)
        };

        Line::from(vec![
            Span::styled(format!("{}  {} ", indent, checkbox), style),
            Span::styled(text.to_string(), text_style),
        ])
    }

    /// Render code block
    fn render_code_block(&self, lines: &[String]) -> Vec<Line<'static>> {
        let mut result = vec![
            Line::from(Span::styled(
                "┌─ code ".to_string() + &"─".repeat(self.width.saturating_sub(12)),
                Style::default().fg(TOKYO_COMMENT),
            )),
        ];

        for line in lines {
            result.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(TOKYO_COMMENT)),
                Span::styled(
                    line.clone(),
                    Style::default().fg(TOKYO_GREEN),
                ),
            ]));
        }

        result.push(Line::from(Span::styled(
            "└".to_string() + &"─".repeat(self.width.saturating_sub(5)),
            Style::default().fg(TOKYO_COMMENT),
        )));

        result
    }

    /// Render paragraph with inline formatting
    fn render_paragraph(&self, text: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                // Bold: **text**
                '*' if chars.peek() == Some(&'*') => {
                    chars.next(); // consume second *
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), Style::default().fg(TOKYO_TEXT)));
                        current.clear();
                    }
                    let mut bold_text = String::new();
                    while let Some(bc) = chars.next() {
                        if bc == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold_text.push(bc);
                    }
                    spans.push(Span::styled(
                        bold_text,
                        Style::default().fg(TOKYO_TEXT).add_modifier(Modifier::BOLD),
                    ));
                }
                // Italic: *text* or _text_
                '*' | '_' => {
                    let delimiter = c;
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), Style::default().fg(TOKYO_TEXT)));
                        current.clear();
                    }
                    let mut italic_text = String::new();
                    while let Some(ic) = chars.next() {
                        if ic == delimiter {
                            break;
                        }
                        italic_text.push(ic);
                    }
                    spans.push(Span::styled(
                        italic_text,
                        Style::default().fg(TOKYO_TEXT).add_modifier(Modifier::ITALIC),
                    ));
                }
                // Inline code: `code`
                '`' => {
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), Style::default().fg(TOKYO_TEXT)));
                        current.clear();
                    }
                    let mut code_text = String::new();
                    while let Some(cc) = chars.next() {
                        if cc == '`' {
                            break;
                        }
                        code_text.push(cc);
                    }
                    spans.push(Span::styled(
                        format!(" {} ", code_text),
                        Style::default().fg(Color::Rgb(130, 165, 110)).bg(Color::Rgb(35, 32, 28)),
                    ));
                }
                // Link: [text](url)
                '[' => {
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), Style::default().fg(TOKYO_TEXT)));
                        current.clear();
                    }
                    let mut link_text = String::new();
                    let mut url = String::new();
                    let mut found_close = false;

                    while let Some(lc) = chars.next() {
                        if lc == ']' {
                            found_close = true;
                            break;
                        }
                        link_text.push(lc);
                    }

                    if found_close && chars.peek() == Some(&'(') {
                        chars.next(); // consume (
                        while let Some(uc) = chars.next() {
                            if uc == ')' {
                                break;
                            }
                            url.push(uc);
                        }
                        spans.push(Span::styled(
                            format!("⌁ {}", link_text),
                            Style::default()
                                .fg(LINK_COLOR)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                    } else {
                        // Not a valid link, just add as text
                        current.push('[');
                        current.push_str(&link_text);
                        if found_close {
                            current.push(']');
                        }
                    }
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            spans.push(Span::styled(current, Style::default().fg(TOKYO_TEXT)));
        }

        if spans.is_empty() {
            Line::from(Span::styled(text.to_string(), Style::default().fg(TOKYO_TEXT)))
        } else {
            Line::from(spans)
        }
    }

    /// Simplified inline formatting for list items
    fn render_inline_formatting(&self, text: &str) -> String {
        // For list items, just strip the markdown syntax and return plain text
        // The actual styling is handled in render_paragraph
        text.to_string()
    }

    /// Render inline formatting and return spans (for list items etc)
    fn render_inline_spans(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                // Bold: **text**
                '*' if chars.peek() == Some(&'*') => {
                    chars.next();
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), Style::default().fg(TOKYO_TEXT)));
                        current.clear();
                    }
                    let mut bold_text = String::new();
                    while let Some(bc) = chars.next() {
                        if bc == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold_text.push(bc);
                    }
                    spans.push(Span::styled(
                        bold_text,
                        Style::default().fg(TOKYO_TEXT).add_modifier(Modifier::BOLD),
                    ));
                }
                // Inline code: `code`
                '`' => {
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), Style::default().fg(TOKYO_TEXT)));
                        current.clear();
                    }
                    let mut code_text = String::new();
                    while let Some(cc) = chars.next() {
                        if cc == '`' {
                            break;
                        }
                        code_text.push(cc);
                    }
                    spans.push(Span::styled(
                        format!(" {} ", code_text),
                        Style::default().fg(Color::Rgb(130, 165, 110)).bg(Color::Rgb(35, 32, 28)),
                    ));
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            spans.push(Span::styled(current, Style::default().fg(TOKYO_TEXT)));
        }

        spans
    }
}

/// Highlight URLs in plain text
pub fn highlight_urls(text: &str) -> Vec<Span<'static>> {
    let url_re = Regex::new(r"(https?://[^\s]+)").unwrap();
    let mut spans = Vec::new();
    let mut last_end = 0;

    for captures in url_re.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Add text before URL
            if m.start() > last_end {
                spans.push(Span::styled(
                    text[last_end..m.start()].to_string(),
                    Style::default().fg(TOKYO_TEXT),
                ));
            }
            // Add URL
            spans.push(Span::styled(
                m.as_str().to_string(),
                Style::default()
                    .fg(TOKYO_BLUE)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            last_end = m.end();
        }
    }

    // Add remaining text
    if last_end < text.len() {
        spans.push(Span::styled(
            text[last_end..].to_string(),
            Style::default().fg(TOKYO_TEXT),
        ));
    }

    if spans.is_empty() {
        vec![Span::styled(text.to_string(), Style::default().fg(TOKYO_TEXT))]
    } else {
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_markdown_headers() {
        let md = StyledMarkdown::new(80);
        let lines = vec![
            "# Header 1".to_string(),
            "## Header 2".to_string(),
            "### Header 3".to_string(),
        ];
        let result = md.render(&lines);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_styled_markdown_code_block() {
        let md = StyledMarkdown::new(80);
        let lines = vec![
            "```".to_string(),
            "fn main() {}".to_string(),
            "```".to_string(),
        ];
        let result = md.render(&lines);
        assert!(result.len() >= 3); // code block wrapper + content
    }
}
