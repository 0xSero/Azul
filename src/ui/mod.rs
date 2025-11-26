mod neon;

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus};
use crate::app::ViewMode;
use neon::NeonBorder;

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();
    if full.width < 4 || full.height < 4 {
        return;
    }

    // Render neon border
    frame.render_widget(
        NeonBorder {
            phase: app.animation_tick,
        },
        full,
    );

    let inner = full.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Main layout: Title | URL Bar | Content | Status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title bar
            Constraint::Length(3),  // URL bar
            Constraint::Min(0),      // Main content (fills all remaining space)
            Constraint::Length(3),  // Status bar
        ])
        .split(inner);

    render_title(frame, chunks[0], app);
    render_url_bar(frame, chunks[1], app);
    render_main_content(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    let accent = Color::Rgb(0, 191, 255); // Azul blue
    let title = vec![
        Span::styled("Azul", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
        Span::raw("-"),
        Span::styled("Browse", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled("v3.0", Style::default().fg(Color::Rgb(122, 162, 247))),
        Span::raw(" | "),
        Span::styled(
            "Terminal Web Browser",
            Style::default()
                .fg(Color::Rgb(192, 202, 245))
                .add_modifier(Modifier::ITALIC),
        ),
    ];

    let title_paragraph = Paragraph::new(Line::from(title))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(122, 162, 247))),
        )
        .style(Style::default().fg(Color::Rgb(192, 202, 245)));

    frame.render_widget(title_paragraph, area);
}

fn render_url_bar(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::URLBar;
    let border_color = if is_focused {
        Color::Rgb(0, 191, 255) // Azul blue when focused
    } else {
        Color::Rgb(122, 162, 247) // Tokyo Night blue when not focused
    };

    let display_text = if is_focused {
        // Show what user is typing with cursor
        format!("{}_", app.url_input)
    } else if let Some(page) = &app.current_page {
        // Show current page URL
        page.url.clone()
    } else {
        // Placeholder
        "Press / to enter URL or search...".to_string()
    };

    let url_paragraph = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(if is_focused { " URL (editing) " } else { " URL " }),
        )
        .style(if is_focused {
            Style::default()
                .fg(Color::Rgb(0, 191, 255))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(192, 202, 245))
        });

    frame.render_widget(url_paragraph, area);
}

fn render_main_content(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.show_help {
        render_help(frame, area);
        return;
    }

    // Split into content and sidebar
    let has_sidebar = app.current_page.as_ref().map(|p| !p.links.is_empty()).unwrap_or(false);

    if has_sidebar {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),  // Content
                Constraint::Percentage(30),  // Sidebar
            ])
            .split(area);

        render_content_area(frame, chunks[0], &*app);
        // render_sidebar needs mutable access to keep the selection in view
        render_sidebar(frame, chunks[1], app);
    } else {
        render_content_area(frame, area, &*app);
    }
}

fn render_content_area(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Content;
    let border_color = if is_focused {
        Color::Rgb(0, 191, 255) // Azul blue when focused
    } else {
        Color::Rgb(122, 162, 247) // Tokyo Night blue when not focused
    };

    let mode_label = match app.view_mode {
        ViewMode::Rendered => "content",
        ViewMode::Raw => "raw",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(if is_focused {
            format!(" {} ", mode_label.to_uppercase())
        } else {
            format!(" {} ", mode_label)
        });

    if let Some(page) = &app.current_page {
        // Calculate visible range
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let visible_height = inner.height as usize;
        let wrap_width = inner.width.saturating_sub(2).max(20) as usize; // leave padding

        // Choose base lines based on view mode
        let mut lines: Vec<String> = match app.view_mode {
            ViewMode::Rendered => page.content_lines.clone(),
            ViewMode::Raw => page.raw_content.lines().map(|s| s.to_string()).collect(),
        };

        // Optional re-formatting for readability
        if app.view_mode == ViewMode::Rendered && app.format_text {
            lines = format_lines(&lines, wrap_width);
        }

        // Prepend AI summary if available
        if let Some(summary) = &app.ai_summary {
            let mut summary_lines = vec!["AI Summary".to_string()];
            for wrapped in textwrap::wrap(summary, wrap_width) {
                summary_lines.push(wrapped.into_owned());
            }
            summary_lines.push(String::new());

            let mut combined = summary_lines;
            combined.extend(lines);
            lines = combined;
        }

        let total_lines = lines.len().max(1);
        let start = app.scroll_offset.min(total_lines.saturating_sub(1));
        let end = (start + visible_height).min(total_lines);

        let visible_lines: Vec<Line> = lines[start..end]
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        let content = Paragraph::new(visible_lines)
            .block(block)
            .style(Style::default().fg(Color::Rgb(192, 202, 245)))
            .wrap(Wrap { trim: false });

        frame.render_widget(content, area);
    } else {
        let placeholder = Paragraph::new("Press / to enter a URL or search")
            .block(block)
            .style(Style::default().fg(Color::Rgb(125, 137, 168)))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(placeholder, area);
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focus == Focus::Sidebar;
    let border_color = if is_focused {
        Color::Rgb(255, 169, 0) // Orange when focused
    } else {
        Color::Rgb(122, 162, 247)
    };

    let total_links = app
        .current_page
        .as_ref()
        .map(|p| p.links.len())
        .unwrap_or(0);

    let position = if total_links == 0 {
        0
    } else {
        app.sidebar_selected + 1
    };

    let title = if is_focused {
        format!(" LINKS {}/{}  j/k navigate, ⏎ open ", position, total_links)
    } else {
        format!(" links {}/{} ", position, total_links)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    if let Some(page) = &app.current_page {
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        // Leave space for block title; inner.height accounts for borders
        let mut visible_rows = inner.height.saturating_sub(1) as usize;
        if visible_rows == 0 {
            visible_rows = 1;
        }

        let mut offset = app.sidebar_offset;
        if app.sidebar_selected < offset {
            offset = app.sidebar_selected;
        } else if app.sidebar_selected >= offset + visible_rows {
            offset = app.sidebar_selected + 1 - visible_rows;
        }
        let max_offset = total_links.saturating_sub(visible_rows);
        offset = offset.min(max_offset);
        app.sidebar_offset = offset;

        // Build visible items
        let accent = Color::Rgb(255, 169, 0);
        let dim = Color::Rgb(122, 162, 247);
        let text_color = Color::Rgb(192, 202, 245);
        let width_for_text = inner.width.saturating_sub(6) as usize; // number + spacing + padding

        let mut items: Vec<ListItem> = page
            .links
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_rows)
            .map(|(i, link)| {
                let primary = if link.text.trim().is_empty() {
                    link.url.clone()
                } else {
                    link.text.clone()
                };

                let truncated = truncate_to_width(&primary, width_for_text);
                let number = format!("{:2}.", i + 1);

                let is_selected = i == app.sidebar_selected;
                let style = if is_selected && is_focused {
                    Style::default()
                        .fg(Color::Black)
                        .bg(accent)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default()
                        .fg(accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(text_color)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(number, Style::default().fg(dim)),
                    Span::raw(" "),
                    Span::styled(truncated, style),
                ]))
            })
            .collect();

        if items.is_empty() {
            items.push(ListItem::new(Span::styled(
                "No links on this page",
                Style::default().fg(Color::Rgb(125, 137, 168)),
            )));
        }

        let list = List::new(items)
            .block(block)
            .style(Style::default().fg(text_color));

        frame.render_widget(list, area);
    } else {
        frame.render_widget(block, area);
    }
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }

    let mut result = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + ch_width + 3 > max_width {
            result.push_str("...");
            break;
        }
        result.push(ch);
        width += ch_width;
    }
    result
}

fn format_lines(lines: &[String], max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return lines.to_owned();
    }

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
            paragraphs.push(String::new());
        } else {
            current.push(line.trim().to_string());
        }
    }

    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }

    let mut out = Vec::new();
    for para in paragraphs {
        if para.is_empty() {
            out.push(String::new());
            continue;
        }

        for wrapped in textwrap::wrap(&para, max_width) {
            out.push(wrapped.into_owned());
        }
    }

    out
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Azul-Browse Keyboard Shortcuts",
            Style::default()
                .fg(Color::Rgb(0, 191, 255))
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  / or Ctrl+L  - Focus URL bar"),
        Line::from("  j/k or ↓/↑   - Scroll content or navigate links"),
        Line::from("  g/G          - Go to top/bottom"),
        Line::from("  Tab/Shift+Tab - Cycle focus"),
        Line::from("  1 or F1      - Focus content"),
        Line::from("  2 or F2      - Focus sidebar"),
        Line::from("  Enter        - Open selected link (in sidebar)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Other:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  v            - Toggle raw/source view"),
        Line::from("  f            - Toggle text re-formatting"),
        Line::from("  s            - Summarize current page (AI)"),
        Line::from("  Ctrl+S       - Save scrape to scrapes/*.json"),
        Line::from("  ?            - Toggle this help"),
        Line::from("  q or Ctrl+C  - Quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ? to close", Style::default().fg(Color::Rgb(125, 137, 168))),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(0, 191, 255)))
                .title(" HELP "),
        )
        .style(Style::default().fg(Color::Rgb(192, 202, 245)))
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(help, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let focus_name = match app.focus {
        Focus::Content => "Content",
        Focus::Sidebar => "Sidebar",
        Focus::URLBar => "URL Bar",
    };

    let accent = Color::Rgb(0, 191, 255);
    let text = Color::Rgb(192, 202, 245);

    let primary_line = Line::from(vec![
        Span::styled("Focus: ", Style::default().fg(text)),
        Span::styled(focus_name, Style::default().fg(accent).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(&app.status_message, Style::default().fg(text)),
    ]);

    let url_display = if app.focus == Focus::URLBar {
        format!("URL: {}_", app.url_input)
    } else if let Some(page) = &app.current_page {
        format!("URL: {}", page.url)
    } else {
        String::new()
    };

    let secondary_line = Line::from(vec![Span::raw(url_display)]);

    let status = Paragraph::new(vec![primary_line, secondary_line])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(122, 162, 247))),
        )
        .style(Style::default().fg(text));

    frame.render_widget(status, area);
}
