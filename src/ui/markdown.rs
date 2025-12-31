//! Clean, readable markdown rendering - OpenCode-inspired typography
//!
//! Focus: generous spacing, clear hierarchy, excellent readability

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use regex::Regex;

// OpenCode-inspired clean color palette
const TEXT_BODY: Color = Color::Rgb(229, 231, 235);      // Clean white #e5e7eb
const TEXT_MUTED: Color = Color::Rgb(156, 163, 175);     // Gray #9ca3af
const TEXT_DIM: Color = Color::Rgb(75, 85, 99);          // Darker muted #4b5563

const HEADER_PRIMARY: Color = Color::Rgb(82, 139, 255);  // Primary blue for H1 #528bff
const HEADER_SECONDARY: Color = Color::Rgb(167, 139, 250); // Purple for H2 #a78bfa
const HEADER_TERTIARY: Color = Color::Rgb(129, 140, 248); // Indigo for H3 #818cf8

const ACCENT_LINK: Color = Color::Rgb(96, 165, 250);     // Light blue #60a5fa
const ACCENT_CODE: Color = Color::Rgb(52, 211, 153);     // Emerald #34d399
const ACCENT_QUOTE: Color = Color::Rgb(148, 163, 184);   // Slate #94a3b8
const ACCENT_LIST: Color = Color::Rgb(251, 191, 36);     // Amber bullet #fbbf24
const ACCENT_MEDIA: Color = Color::Rgb(245, 158, 11);    // Warm amber for media labels #f59e0b
const ACCENT_REF: Color = Color::Rgb(148, 163, 184);     // Slate for references #94a3b8

const CODE_BG: Color = Color::Rgb(24, 24, 27);           // Darker code bg #18181b

// Layout constants for consistent spacing
const INDENT: &str = "   ";                              // 3 spaces base indent
const MAX_EXTRA_INDENT: usize = 24;                      // Cap extra indent for readability

/// Styled markdown renderer - clean, PDF-like output
pub struct StyledMarkdown {
    width: usize,
}

impl StyledMarkdown {
    pub fn new(width: usize) -> Self {
        Self { width: width.max(20) }
    }

    /// Render content with proper typography
    pub fn render(&self, lines: &[String]) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        let mut in_code_block = false;
        let mut in_think_block = false;
        let mut code_buffer = Vec::new();
        let mut think_buffer = Vec::new();
        let mut prev_was_empty = true;

        for line in lines {
            // Handle <think> blocks (LLM reasoning)
            if line.trim() == "<think>" || line.trim().starts_with("<think>") {
                if !prev_was_empty {
                    result.push(Line::from(""));
                }
                in_think_block = true;
                // If there's content on the same line as <think>
                let after_tag = line.trim().strip_prefix("<think>").unwrap_or("").trim();
                if !after_tag.is_empty() && after_tag != "</think>" {
                    think_buffer.push(after_tag.to_string());
                }
                continue;
            }
            if line.trim() == "</think>" || line.trim().ends_with("</think>") {
                // Render accumulated think content
                if !think_buffer.is_empty() {
                    result.extend(self.render_think_block(&think_buffer));
                    result.push(Line::from(""));
                }
                think_buffer.clear();
                in_think_block = false;
                prev_was_empty = true;
                continue;
            }
            if in_think_block {
                think_buffer.push(line.clone());
                continue;
            }

            // Handle ANSI escape codes (e.g., from chafa image output)
            if line.contains('\x1b') {
                let spans = parse_ansi_line(line);
                result.push(Line::from(spans));
                prev_was_empty = false;
                continue;
            }

            // Handle code block delimiters
            if line.trim().starts_with("```") {
                if in_code_block {
                    result.extend(self.render_code_block(&code_buffer));
                    result.push(Line::from("")); // Space after code
                    code_buffer.clear();
                    in_code_block = false;
                    prev_was_empty = true;
                } else {
                    if !prev_was_empty {
                        result.push(Line::from("")); // Space before code
                    }
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_buffer.push(line.clone());
                continue;
            }

            let rendered = self.render_line(line, prev_was_empty);
            prev_was_empty = line.trim().is_empty();
            result.extend(rendered);
        }

        // Handle unclosed code block
        if !code_buffer.is_empty() {
            result.extend(self.render_code_block(&code_buffer));
        }

        // Handle unclosed think block
        if !think_buffer.is_empty() {
            result.extend(self.render_think_block(&think_buffer));
        }

        result
    }

    /// Render a single line with context awareness
    fn render_line(&self, line: &str, prev_empty: bool) -> Vec<Line<'static>> {
        let trimmed = line.trim();

        // Empty line - paragraph break
        if trimmed.is_empty() {
            return vec![Line::from("")];
        }

        // Images: ![alt](url)
        if let Some((alt, url)) = parse_image_syntax(trimmed) {
            return vec![self.render_image(&alt, &url)];
        }

        // Headers - clean, simple hierarchy
        if trimmed.starts_with("#### ") {
            return self.with_spacing(self.render_h4(&trimmed[5..]), prev_empty, false);
        }
        if trimmed.starts_with("### ") {
            return self.with_spacing(self.render_h3(&trimmed[4..]), prev_empty, false);
        }
        if trimmed.starts_with("## ") {
            return self.with_spacing(self.render_h2(&trimmed[3..]), prev_empty, true);
        }
        if trimmed.starts_with("# ") {
            return self.with_spacing(self.render_h1(&trimmed[2..]), prev_empty, true);
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            return vec![Line::from(""), self.render_hr(), Line::from("")];
        }

        // Blockquote
        if trimmed.starts_with("> ") {
            return vec![self.render_quote(&trimmed[2..])];
        }

        // List items
        let indent = line.len() - line.trim_start().len();
        let level = indent / 2;

        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            return vec![self.render_bullet(&trimmed[2..], level)];
        }

        // Numbered list
        if let Some(caps) = Regex::new(r"^(\d+)\.\s+(.*)$").ok().and_then(|re| re.captures(trimmed)) {
            if let (Some(num), Some(text)) = (caps.get(1), caps.get(2)) {
                return vec![self.render_number(num.as_str(), text.as_str(), level)];
            }
        }

        // Task items
        if trimmed.starts_with("- [ ] ") {
            return vec![self.render_task(&trimmed[6..], false, level)];
        }
        if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
            return vec![self.render_task(&trimmed[6..], true, level)];
        }

        // Regular paragraph
        vec![self.render_paragraph(line)]
    }

    /// Add generous spacing around headers for clear visual hierarchy
    fn with_spacing(&self, line: Line<'static>, prev_empty: bool, major: bool) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        // Always add breathing room before headers
        if !prev_empty {
            result.push(Line::from(""));
        }
        if major {
            result.push(Line::from("")); // Extra space before major headers (H1, H2)
        }
        result.push(line);
        // Add space after headers too for clarity
        result.push(Line::from(""));
        result
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Headers - Clean hierarchy with visual markers
    // ─────────────────────────────────────────────────────────────────────────

    fn render_h1(&self, text: &str) -> Line<'static> {
        // H1: Bold uppercase-style with decorative marker
        Line::from(vec![
            Span::styled("━━ ", Style::default().fg(HEADER_PRIMARY)),
            Span::styled(
                text.to_uppercase(),
                Style::default()
                    .fg(HEADER_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ━━", Style::default().fg(HEADER_PRIMARY)),
        ])
    }

    fn render_h2(&self, text: &str) -> Line<'static> {
        // H2: Clear section marker
        Line::from(vec![
            Span::styled("── ", Style::default().fg(HEADER_SECONDARY)),
            Span::styled(
                text.to_string(),
                Style::default()
                    .fg(HEADER_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    fn render_h3(&self, text: &str) -> Line<'static> {
        // H3: Subtle subsection
        Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("▸ ", Style::default().fg(HEADER_TERTIARY)),
            Span::styled(
                text.to_string(),
                Style::default()
                    .fg(HEADER_TERTIARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    fn render_h4(&self, text: &str) -> Line<'static> {
        // H4: Minor heading
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled(
                text.to_string(),
                Style::default().fg(TEXT_BODY).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED),
            ),
        ])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Horizontal rule
    // ─────────────────────────────────────────────────────────────────────────

    fn render_hr(&self) -> Line<'static> {
        // Clean horizontal divider
        Line::from(Span::styled(
            "─".repeat(self.width.min(50)),
            Style::default().fg(TEXT_DIM),
        ))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Blockquote - elegant left border with padding
    // ─────────────────────────────────────────────────────────────────────────

    fn render_quote(&self, text: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("| ", Style::default().fg(ACCENT_QUOTE)),
            Span::styled(
                text.to_string(),
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            ),
        ])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Lists - clean, well-aligned bullets and numbers
    // ─────────────────────────────────────────────────────────────────────────

    fn render_bullet(&self, text: &str, level: usize) -> Line<'static> {
        let indent = "    ".repeat(level);  // 4 spaces per level
        let bullet = match level {
            0 => "-",
            1 => "*",
            _ => "+",
        };

        let mut spans = vec![
            Span::styled(format!("   {}", indent), Style::default()),
            Span::styled(format!("{} ", bullet), Style::default().fg(ACCENT_LIST)),
        ];
        spans.extend(self.parse_inline(text));
        Line::from(spans)
    }

    fn render_number(&self, num: &str, text: &str, level: usize) -> Line<'static> {
        let indent = "    ".repeat(level);

        let mut spans = vec![
            Span::styled(format!("   {}", indent), Style::default()),
            Span::styled(format!("{}. ", num), Style::default().fg(ACCENT_LIST)),
        ];
        spans.extend(self.parse_inline(text));
        Line::from(spans)
    }

    fn render_task(&self, text: &str, done: bool, level: usize) -> Line<'static> {
        let indent = "    ".repeat(level);
        let (check, check_style) = if done {
            ("[x]", Style::default().fg(ACCENT_CODE))  // Done
        } else {
            ("[ ]", Style::default().fg(TEXT_DIM))
        };

        let text_style = if done {
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::default().fg(TEXT_BODY)
        };

        Line::from(vec![
            Span::styled(format!("   {}", indent), Style::default()),
            Span::styled(format!("{} ", check), check_style),
            Span::styled(text.to_string(), text_style),
        ])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Code blocks - bordered and distinct
    // ─────────────────────────────────────────────────────────────────────────

    fn render_code_block(&self, lines: &[String]) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        let code_border = Color::Rgb(55, 55, 62);  // BORDER_DEFAULT equivalent

        // Top border
        result.push(Line::from(vec![
            Span::styled("   +", Style::default().fg(code_border)),
            Span::styled("-".repeat(self.width.saturating_sub(8).min(70)), Style::default().fg(code_border)),
        ]));

        // Code lines with left border
        for line in lines {
            // Pad line to consistent width for clean bg
            let padded = format!("{:<width$}", line, width = self.width.saturating_sub(8).min(70));
            result.push(Line::from(vec![
                Span::styled("   | ", Style::default().fg(code_border)),
                Span::styled(
                    padded,
                    Style::default().fg(ACCENT_CODE).bg(CODE_BG),
                ),
            ]));
        }

        // Bottom border
        result.push(Line::from(vec![
            Span::styled("   +", Style::default().fg(code_border)),
            Span::styled("-".repeat(self.width.saturating_sub(8).min(70)), Style::default().fg(code_border)),
        ]));

        result
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Think blocks - LLM reasoning/thinking with distinct styling
    // ─────────────────────────────────────────────────────────────────────────

    fn render_think_block(&self, lines: &[String]) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        let think_border = Color::Rgb(88, 88, 110);   // Muted purple-gray
        let think_text = Color::Rgb(140, 140, 170);   // Dim text for thinking
        let think_bg = Color::Rgb(20, 20, 25);        // Slightly different bg

        // Header
        result.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("╭─ ", Style::default().fg(think_border)),
            Span::styled("thinking", Style::default().fg(think_border).add_modifier(Modifier::ITALIC)),
            Span::styled(" ─".to_string() + &"─".repeat(self.width.saturating_sub(20).min(50)), Style::default().fg(think_border)),
        ]));

        // Think content with left border
        for line in lines {
            if line.trim().is_empty() {
                result.push(Line::from(vec![
                    Span::styled("   │", Style::default().fg(think_border)),
                ]));
            } else {
                result.push(Line::from(vec![
                    Span::styled("   │ ", Style::default().fg(think_border)),
                    Span::styled(
                        line.clone(),
                        Style::default().fg(think_text).add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }

        // Footer
        result.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("╰", Style::default().fg(think_border)),
            Span::styled("─".repeat(self.width.saturating_sub(8).min(60)), Style::default().fg(think_border)),
        ]));

        result
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Paragraph - proper typography with inline formatting
    // ─────────────────────────────────────────────────────────────────────────

    fn render_paragraph(&self, text: &str) -> Line<'static> {
        // Consistent left margin for body text, while respecting input indentation
        // (used for wrapped list continuations, nested blocks, etc.)
        let extra_indent = (text.len() - text.trim_start().len()).min(MAX_EXTRA_INDENT);
        let prefix = format!("{}{}", INDENT, " ".repeat(extra_indent));

        let mut spans = vec![Span::styled(prefix, Style::default())];
        spans.extend(self.parse_inline(text.trim_start()));
        Line::from(spans)
    }

    fn render_image(&self, alt: &str, url: &str) -> Line<'static> {
        let url = truncate_end(url, self.width.saturating_sub(20).max(20));
        Line::from(vec![
            Span::styled(INDENT, Style::default()),
            Span::styled("[image] ", Style::default().fg(ACCENT_MEDIA).add_modifier(Modifier::BOLD)),
            Span::styled(alt.to_string(), Style::default().fg(ACCENT_LINK).add_modifier(Modifier::UNDERLINED)),
            Span::styled("  ", Style::default()),
            Span::styled(url, Style::default().fg(TEXT_DIM)),
        ])
    }

    /// Parse inline formatting: **bold**, *italic*, `code`, [links](url)
    fn parse_inline(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                // Bold: **text**
                '*' if chars.peek() == Some(&'*') => {
                    chars.next();
                    self.flush(&mut spans, &mut current);
                    let content = self.read_until(&mut chars, |c, peek| c == '*' && peek == Some(&'*'));
                    if chars.peek() == Some(&'*') { chars.next(); }
                    spans.push(Span::styled(
                        content,
                        Style::default().fg(TEXT_BODY).add_modifier(Modifier::BOLD),
                    ));
                }
                // Italic: *text* (single asterisk)
                '*' => {
                    self.flush(&mut spans, &mut current);
                    let content = self.read_until(&mut chars, |c, _| c == '*');
                    spans.push(Span::styled(
                        content,
                        Style::default().fg(TEXT_BODY).add_modifier(Modifier::ITALIC),
                    ));
                }
                // Inline code: `code`
                '`' => {
                    self.flush(&mut spans, &mut current);
                    let content = self.read_until(&mut chars, |c, _| c == '`');
                    spans.push(Span::styled(
                        format!(" {} ", content),
                        Style::default().fg(ACCENT_CODE).bg(CODE_BG),
                    ));
                }
                // Link: [text](url)
                '[' => {
                    self.flush(&mut spans, &mut current);
                    let link_text = self.read_until(&mut chars, |c, _| c == ']');

                    // Reference marker: [1], [12], [123] (but not a link like [text](url))
                    if link_text.chars().all(|c| c.is_ascii_digit())
                        && (1..=3).contains(&link_text.len())
                        && chars.peek() != Some(&'(')
                    {
                        spans.push(Span::styled(
                            format!("[{}]", link_text),
                            Style::default().fg(ACCENT_REF).add_modifier(Modifier::BOLD),
                        ));
                    } else if chars.peek() == Some(&'(') {
                        chars.next();
                        let _url = self.read_until(&mut chars, |c, _| c == ')');
                        spans.push(Span::styled(
                            link_text,
                            Style::default().fg(ACCENT_LINK).add_modifier(Modifier::UNDERLINED),
                        ));
                    } else {
                        current.push('[');
                        current.push_str(&link_text);
                        current.push(']');
                    }
                }
                _ => current.push(c),
            }
        }

        self.flush(&mut spans, &mut current);

        if spans.is_empty() {
            vec![Span::styled(text.to_string(), Style::default().fg(TEXT_BODY))]
        } else {
            spans
        }
    }

    fn flush(&self, spans: &mut Vec<Span<'static>>, current: &mut String) {
        if !current.is_empty() {
            spans.push(Span::styled(current.clone(), Style::default().fg(TEXT_BODY)));
            current.clear();
        }
    }

    fn read_until<F>(&self, chars: &mut std::iter::Peekable<std::str::Chars>, end: F) -> String
    where
        F: Fn(char, Option<&char>) -> bool,
    {
        let mut result = String::new();
        while let Some(&c) = chars.peek() {
            if end(c, chars.clone().nth(1).as_ref()) {
                chars.next();
                break;
            }
            result.push(chars.next().unwrap());
        }
        result
    }
}

fn parse_image_syntax(line: &str) -> Option<(String, String)> {
    // Minimal parser for the common inline image syntax: ![alt](url)
    let s = line.trim();
    if !s.starts_with("![") {
        return None;
    }
    let alt_end = s.find("](")?;
    if !s.ends_with(')') {
        return None;
    }

    let alt = &s[2..alt_end]; // after "!["
    let url = &s[(alt_end + 2)..(s.len() - 1)]; // after "](" .. before ")"
    if url.trim().is_empty() {
        return None;
    }
    Some((alt.trim().to_string(), url.trim().to_string()))
}

fn truncate_end(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i + 1 >= max_len {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

/// Parse a line containing ANSI escape codes into styled spans
fn parse_ansi_line(s: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_fg: Option<Color> = None;
    let mut current_bg: Option<Color> = None;

    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Flush current text
            if !current_text.is_empty() {
                let mut style = Style::default();
                if let Some(fg) = current_fg {
                    style = style.fg(fg);
                }
                if let Some(bg) = current_bg {
                    style = style.bg(bg);
                }
                spans.push(Span::styled(std::mem::take(&mut current_text), style));
            }

            // Parse escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut code = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_alphabetic() {
                        chars.next(); // consume final letter
                        break;
                    }
                    code.push(chars.next().unwrap());
                }

                // Parse SGR codes (e.g., "38;2;255;128;0" for 24-bit fg color)
                let parts: Vec<&str> = code.split(';').collect();
                let mut i = 0;
                while i < parts.len() {
                    match parts[i] {
                        "0" => {
                            current_fg = None;
                            current_bg = None;
                        }
                        "38" if i + 4 < parts.len() && parts[i + 1] == "2" => {
                            // 24-bit foreground: 38;2;r;g;b
                            if let (Ok(r), Ok(g), Ok(b)) = (
                                parts[i + 2].parse::<u8>(),
                                parts[i + 3].parse::<u8>(),
                                parts[i + 4].parse::<u8>(),
                            ) {
                                current_fg = Some(Color::Rgb(r, g, b));
                            }
                            i += 4;
                        }
                        "48" if i + 4 < parts.len() && parts[i + 1] == "2" => {
                            // 24-bit background: 48;2;r;g;b
                            if let (Ok(r), Ok(g), Ok(b)) = (
                                parts[i + 2].parse::<u8>(),
                                parts[i + 3].parse::<u8>(),
                                parts[i + 4].parse::<u8>(),
                            ) {
                                current_bg = Some(Color::Rgb(r, g, b));
                            }
                            i += 4;
                        }
                        // Basic colors
                        "30" => current_fg = Some(Color::Black),
                        "31" => current_fg = Some(Color::Red),
                        "32" => current_fg = Some(Color::Green),
                        "33" => current_fg = Some(Color::Yellow),
                        "34" => current_fg = Some(Color::Blue),
                        "35" => current_fg = Some(Color::Magenta),
                        "36" => current_fg = Some(Color::Cyan),
                        "37" => current_fg = Some(Color::White),
                        "39" => current_fg = None,
                        "40" => current_bg = Some(Color::Black),
                        "41" => current_bg = Some(Color::Red),
                        "42" => current_bg = Some(Color::Green),
                        "43" => current_bg = Some(Color::Yellow),
                        "44" => current_bg = Some(Color::Blue),
                        "45" => current_bg = Some(Color::Magenta),
                        "46" => current_bg = Some(Color::Cyan),
                        "47" => current_bg = Some(Color::White),
                        "49" => current_bg = None,
                        _ => {}
                    }
                    i += 1;
                }
            }
        } else {
            current_text.push(c);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        let mut style = Style::default();
        if let Some(fg) = current_fg {
            style = style.fg(fg);
        }
        if let Some(bg) = current_bg {
            style = style.bg(bg);
        }
        spans.push(Span::styled(current_text, style));
    }

    if spans.is_empty() {
        spans.push(Span::raw(s.to_string()));
    }

    spans
}

/// Strip ANSI escape codes from a string (fallback when parsing fails)
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            // End of escape sequence
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers() {
        let md = StyledMarkdown::new(80);
        let lines = vec![
            "# Main Title".to_string(),
            "## Section".to_string(),
            "### Subsection".to_string(),
        ];
        let result = md.render(&lines);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_code_block() {
        let md = StyledMarkdown::new(80);
        let lines = vec![
            "```".to_string(),
            "let x = 1;".to_string(),
            "```".to_string(),
        ];
        let result = md.render(&lines);
        assert!(!result.is_empty());
    }
}
