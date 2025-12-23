pub mod markdown;
pub mod theme;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, PanelMode, ViewMode};
use crate::mascot;
use markdown::StyledMarkdown;

// Warm Paper DARK - cozy dark with warm undertones
// Background layers (warm darks, not cold)
pub const BG_BASE: Color = Color::Rgb(28, 26, 24);          // Warm black #1c1a18
pub const BG_SURFACE: Color = Color::Rgb(38, 35, 32);       // Warm charcoal #262320
pub const BG_ELEVATED: Color = Color::Rgb(50, 46, 42);      // Warm brown-gray #322e2a
pub const BG_HIGHLIGHT: Color = Color::Rgb(65, 58, 50);     // Warm tan-dark #413a32

// Border colors - visible warm lines
pub const BORDER_DIM: Color = Color::Rgb(60, 55, 48);       // Subtle #3c3730
pub const BORDER_DEFAULT: Color = Color::Rgb(85, 75, 65);   // Normal #554b41
pub const BORDER_BRIGHT: Color = Color::Rgb(140, 120, 95);  // Focused #8c785f

// Accent colors - warm and visible on dark
pub const ACCENT_PRIMARY: Color = Color::Rgb(200, 160, 110);  // Warm gold #c8a06e
pub const ACCENT_SECONDARY: Color = Color::Rgb(170, 135, 95); // Tan #aa875f
pub const ACCENT_SUCCESS: Color = Color::Rgb(130, 165, 110);  // Sage #82a56e
pub const ACCENT_WARNING: Color = Color::Rgb(220, 170, 90);   // Amber #dcaa5a
pub const ACCENT_ERROR: Color = Color::Rgb(200, 110, 100);    // Terracotta #c86e64
pub const ACCENT_INFO: Color = Color::Rgb(120, 155, 185);     // Warm blue #789bb9

// Text colors - warm cream on dark
pub const TEXT_PRIMARY: Color = Color::Rgb(235, 228, 215);    // Warm cream #ebe4d7
pub const TEXT_SECONDARY: Color = Color::Rgb(175, 165, 150);  // Muted cream #afa596
pub const TEXT_DIM: Color = Color::Rgb(120, 112, 100);        // Dim warm #787064

// Legacy aliases
pub const AZUL_BLUE: Color = ACCENT_PRIMARY;
pub const TOKYO_BLUE: Color = BORDER_DEFAULT;
pub const TOKYO_PURPLE: Color = ACCENT_SECONDARY;
pub const TOKYO_ORANGE: Color = ACCENT_WARNING;
pub const TOKYO_GREEN: Color = ACCENT_SUCCESS;
pub const TOKYO_RED: Color = ACCENT_ERROR;
pub const TOKYO_TEXT: Color = TEXT_PRIMARY;
pub const TOKYO_COMMENT: Color = TEXT_DIM;
pub const TOKYO_BG: Color = BG_BASE;
pub const TOKYO_CYAN: Color = ACCENT_INFO;
pub const FOCUS_RING: Color = BORDER_BRIGHT;

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();
    if full.width < 4 || full.height < 4 {
        return;
    }

    // Fill entire frame with base background and simple border
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_DIM))
        .style(Style::default().bg(BG_BASE));
    frame.render_widget(outer_block, full);

    let inner = full.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Main layout with spacing
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Tab bar
            Constraint::Length(3),   // URL bar
            Constraint::Length(1),   // Spacer
            Constraint::Min(0),      // Main content
            Constraint::Length(2),   // Status bar (compact)
        ])
        .split(inner);

    render_tab_bar(frame, chunks[0], app);
    render_url_bar(frame, chunks[1], app);
    // chunks[2] is spacer - empty

    // 3-panel layout with gaps: Links | Content | Chat
    let content_area = chunks[3];
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(14),  // Links (left)
            Constraint::Length(1),       // Gap
            Constraint::Percentage(48),  // Content (middle)
            Constraint::Length(1),       // Gap
            Constraint::Percentage(36),  // Chat (right)
        ])
        .split(content_area);

    render_compact_sidebar(frame, split[0], app);
    // split[1] is gap
    render_content_only(frame, split[2], app);
    // split[3] is gap
    render_chat_side_panel(frame, split[4], app);

    render_status_bar(frame, chunks[4], app);

    // Render overlay panels (not chat anymore)
    match app.panel_mode {
        PanelMode::Bookmarks => render_bookmarks_panel(frame, inner, app),
        PanelMode::History => render_history_panel(frame, inner, app),
        PanelMode::Help => render_help_panel(frame, inner, app),
        PanelMode::Chat => {} // Handled above as side panel
        PanelMode::Settings => render_settings_panel(frame, inner, app),
        PanelMode::Rag => render_rag_panel(frame, inner, app),
        PanelMode::Memory => render_memory_panel(frame, inner, app),
        PanelMode::None => {}
    }
}

fn render_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let tabs = app.tabs.tabs();
    let active = app.tabs.active_index();
    let bg_color = BG_SURFACE;

    // Fill background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, area);

    let mut spans = vec![];

    for (i, tab) in tabs.iter().enumerate() {
        let is_active = i == active;
        let title = tab.display_title(15);

        let style = if is_active {
            Style::default().fg(BG_BASE).bg(AZUL_BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TOKYO_TEXT).bg(bg_color)
        };

        // Tab number
        spans.push(Span::styled(format!(" {} ", i + 1), Style::default().fg(TOKYO_COMMENT).bg(bg_color)));
        // Tab title
        spans.push(Span::styled(title, style));

        if tab.loading {
            spans.push(Span::styled(" *", Style::default().fg(TOKYO_ORANGE).bg(bg_color)));
        }

        spans.push(Span::styled(" |", Style::default().bg(bg_color)));
    }

    // Add new tab hint if not full
    if !app.tabs.is_full() {
        spans.push(Span::styled(" + (t)", Style::default().fg(TOKYO_COMMENT).bg(bg_color)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(paragraph, area);
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    // Check if current page is bookmarked
    let bookmark_indicator = if app.is_current_bookmarked() {
        Span::styled(" ★", Style::default().fg(TOKYO_ORANGE))
    } else {
        Span::raw("")
    };

    // Companion in title - always visible
    let companion = app.mascot.view();

    let title = vec![
        Span::styled(companion, Style::default().fg(AZUL_BLUE)),
        Span::raw(" "),
        Span::styled("azul", Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD)),
        bookmark_indicator,
    ];

    let title_paragraph = Paragraph::new(Line::from(title))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TOKYO_BLUE)),
        )
        .style(Style::default().fg(TOKYO_TEXT));

    frame.render_widget(title_paragraph, area);
}

fn render_url_bar(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::URLBar;
    let border_color = if is_focused { FOCUS_RING } else { TOKYO_BLUE };
    let bg_color = if is_focused { BG_ELEVATED } else { BG_SURFACE };

    // Companion appears in URL bar when active
    let companion_prefix = match app.mascot.state() {
        mascot::CompanionState::Loading | mascot::CompanionState::Searching => {
            format!("{} ", app.mascot.view())
        }
        _ => String::new(),
    };

    let display_text = if is_focused {
        format!("{}{}_", companion_prefix, app.url_input)
    } else if let Some(page) = app.current_page() {
        format!("{}{}", companion_prefix, page.url)
    } else {
        format!("{}/ to search...", companion_prefix)
    };

    let url_paragraph = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(bg_color))
                .title(if is_focused { " URL (editing) " } else { " URL " }),
        )
        .style(if is_focused {
            Style::default().fg(AZUL_BLUE).bg(bg_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TOKYO_TEXT).bg(bg_color)
        });

    frame.render_widget(url_paragraph, area);
}

fn render_main_content(frame: &mut Frame, area: Rect, app: &mut App) {
    // Split into content and sidebar
    let has_sidebar = app.current_page().map(|p| !p.links.is_empty()).unwrap_or(false);

    if has_sidebar {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(area);

        render_content_area(frame, chunks[0], app);
        render_sidebar(frame, chunks[1], app);
    } else {
        render_content_area(frame, area, app);
    }
}

/// Render only content without sidebar (used when chat is open)
fn render_content_only(frame: &mut Frame, area: Rect, app: &App) {
    render_content_area(frame, area, app);
}

fn render_content_area(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Content;
    let border_color = if is_focused { FOCUS_RING } else { TOKYO_BLUE };
    let bg_color = BG_SURFACE;

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
        })
        .style(Style::default().bg(bg_color));

    if let Some(page) = app.current_page() {
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let visible_height = inner.height as usize;
        let wrap_width = inner.width.saturating_sub(2).max(20) as usize;

        let mut lines: Vec<String> = match app.view_mode {
            ViewMode::Rendered => page.content_lines.clone(),
            ViewMode::Raw => page.raw_content.lines().map(|s| s.to_string()).collect(),
        };

        if app.view_mode == ViewMode::Rendered && app.format_text {
            lines = format_lines(&lines, wrap_width);
        }

        // Prepend AI summary if available
        if let Some(summary) = &app.ai_summary {
            let mut summary_lines = vec![
                "# AI Summary".to_string(),
                String::new(),
            ];
            for wrapped in textwrap::wrap(summary, wrap_width) {
                summary_lines.push(wrapped.into_owned());
            }
            summary_lines.push(String::new());
            summary_lines.push("---".to_string());
            summary_lines.push(String::new());

            let mut combined = summary_lines;
            combined.extend(lines);
            lines = combined;
        }

        let total_lines = lines.len().max(1);
        let scroll = app.scroll_offset();
        let start = scroll.min(total_lines.saturating_sub(1));
        let end = (start + visible_height).min(total_lines);

        // Use styled markdown rendering for rendered view
        let visible_lines: Vec<Line> = if app.view_mode == ViewMode::Rendered {
            let md_renderer = StyledMarkdown::new(wrap_width);
            let styled = md_renderer.render(&lines[start..end].to_vec());
            styled
        } else {
            // Raw view - plain text
            lines[start..end]
                .iter()
                .map(|line| Line::from(line.clone()))
                .collect()
        };

        let content = Paragraph::new(visible_lines)
            .block(block)
            .style(Style::default().fg(TOKYO_TEXT).bg(bg_color))
            .wrap(Wrap { trim: false });

        frame.render_widget(content, area);
    } else {
        // Show splash with mascot when no page loaded
        let splash_text = mascot::mini_splash();
        let placeholder = Paragraph::new(splash_text)
            .block(block)
            .style(Style::default().fg(AZUL_BLUE).bg(bg_color))
            .alignment(Alignment::Center);

        frame.render_widget(placeholder, area);
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focus == Focus::Sidebar;
    let border_color = if is_focused { FOCUS_RING } else { TOKYO_BLUE };
    let bg_color = BG_SURFACE;

    let total_links = app.current_page().map(|p| p.links.len()).unwrap_or(0);
    let selected = app.sidebar_selected();

    let position = if total_links == 0 { 0 } else { selected + 1 };

    let title = if is_focused {
        format!(" LINKS {}/{} ", position, total_links)
    } else {
        format!(" links {}/{} ", position, total_links)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(bg_color));

    if let Some(page) = app.current_page() {
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let mut visible_rows = inner.height.saturating_sub(1) as usize;
        if visible_rows == 0 {
            visible_rows = 1;
        }

        // Calculate offset for scrolling
        let mut offset = 0;
        if selected >= visible_rows {
            offset = selected + 1 - visible_rows;
        }

        let width_for_text = inner.width.saturating_sub(6) as usize;

        let items: Vec<ListItem> = page
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

                let is_selected = i == selected;
                let style = if is_selected && is_focused {
                    Style::default().fg(BG_BASE).bg(AZUL_BLUE).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(AZUL_BLUE).bg(BG_HIGHLIGHT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TOKYO_TEXT).bg(bg_color)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(number, Style::default().fg(TOKYO_COMMENT).bg(bg_color)),
                    Span::styled(" ", Style::default().bg(bg_color)),
                    Span::styled(truncated, style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

        frame.render_widget(list, area);
    } else {
        frame.render_widget(block, area);
    }
}

/// Compact sidebar for when chat is open (narrow links panel)
fn render_compact_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Sidebar;
    let bg_color = BG_SURFACE;

    let border_color = if is_focused { BORDER_BRIGHT } else { BORDER_DEFAULT };

    let total_links = app.current_page().map(|p| p.links.len()).unwrap_or(0);
    let selected = app.sidebar_selected();

    let title = format!(" {} ", total_links);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(bg_color));

    if let Some(page) = app.current_page() {
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let mut visible_rows = inner.height.saturating_sub(1) as usize;
        if visible_rows == 0 {
            visible_rows = 1;
        }

        // Calculate offset for scrolling
        let mut offset = 0;
        if selected >= visible_rows {
            offset = selected + 1 - visible_rows;
        }

        // Show link number and truncated text
        let max_text_width = inner.width.saturating_sub(5) as usize; // Leave room for number and space
        let items: Vec<ListItem> = page
            .links
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_rows)
            .map(|(i, link)| {
                let is_selected = i == selected;
                let style = if is_selected && is_focused {
                    Style::default().fg(BG_BASE).bg(AZUL_BLUE).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(AZUL_BLUE).bg(BG_HIGHLIGHT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TOKYO_TEXT).bg(bg_color)
                };

                // Show number + truncated link text
                let text = if link.text.is_empty() { &link.url } else { &link.text };
                let truncated = truncate_to_width(text, max_text_width);
                ListItem::new(Line::from(Span::styled(format!("{:2} {}", i + 1, truncated), style)))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

        frame.render_widget(list, area);
    } else {
        frame.render_widget(block, area);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &mut App) {
    // Compact single-line status
    let tab_info = format!("{}/{}", app.tabs.active_index() + 1, app.tabs.count());
    let companion = app.mascot.view_status();

    let status_line = Line::from(vec![
        Span::styled(format!(" {} ", companion), Style::default().fg(ACCENT_PRIMARY)),
        Span::styled(format!("[{}] ", tab_info), Style::default().fg(TEXT_SECONDARY)),
        Span::styled(&app.status_message, Style::default().fg(TEXT_PRIMARY)),
        Span::styled("  │  ", Style::default().fg(BORDER_DIM)),
        Span::styled("/ search  t tab  b marks  ? help  q quit", Style::default().fg(TEXT_DIM)),
    ]);

    let status = Paragraph::new(status_line)
        .style(Style::default().fg(TEXT_PRIMARY).bg(BG_SURFACE));

    frame.render_widget(status, area);
}

fn render_bookmarks_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);
    let bg_color = BG_ELEVATED;

    // Fill with elevated background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AZUL_BLUE))
        .title(format!(" Bookmarks ({}) | j/k nav | Enter open | d delete | Esc close ", app.bookmarks_list.len()))
        .style(Style::default().bg(bg_color));

    let inner = panel_area.inner(Margin { horizontal: 1, vertical: 1 });
    let visible_rows = inner.height.saturating_sub(1) as usize;

    if app.bookmarks_list.is_empty() {
        let empty = Paragraph::new("No bookmarks yet. Press B to bookmark current page.")
            .block(block)
            .style(Style::default().fg(TOKYO_COMMENT).bg(bg_color))
            .alignment(Alignment::Center);
        frame.render_widget(empty, panel_area);
        return;
    }

    // Calculate offset
    let mut offset = 0;
    if app.bookmarks_selected >= visible_rows {
        offset = app.bookmarks_selected + 1 - visible_rows;
    }

    let items: Vec<ListItem> = app.bookmarks_list
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_rows)
        .map(|(i, bookmark)| {
            let is_selected = i == app.bookmarks_selected;
            let style = if is_selected {
                Style::default().fg(BG_BASE).bg(AZUL_BLUE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TOKYO_TEXT).bg(bg_color)
            };

            let title = truncate_to_width(&bookmark.title, 40);
            let url = truncate_to_width(&bookmark.url, 30);

            ListItem::new(vec![
                Line::from(Span::styled(format!("  {} ", title), style)),
                Line::from(Span::styled(format!("    {}", url), Style::default().fg(TOKYO_COMMENT).bg(bg_color))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(list, panel_area);
}

fn render_history_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);
    let bg_color = BG_ELEVATED;

    // Fill with elevated background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_PURPLE))
        .title(format!(" History ({}) | j/k nav | Enter open | Esc close ", app.history_list.len()))
        .style(Style::default().bg(bg_color));

    let inner = panel_area.inner(Margin { horizontal: 1, vertical: 1 });
    let visible_rows = inner.height.saturating_sub(1) as usize;

    if app.history_list.is_empty() {
        let empty = Paragraph::new("No history yet. Start browsing!")
            .block(block)
            .style(Style::default().fg(TOKYO_COMMENT).bg(bg_color))
            .alignment(Alignment::Center);
        frame.render_widget(empty, panel_area);
        return;
    }

    let mut offset = 0;
    if app.history_selected >= visible_rows {
        offset = app.history_selected + 1 - visible_rows;
    }

    let items: Vec<ListItem> = app.history_list
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_rows)
        .map(|(i, entry)| {
            let is_selected = i == app.history_selected;
            let style = if is_selected {
                Style::default().fg(BG_BASE).bg(TOKYO_PURPLE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TOKYO_TEXT).bg(bg_color)
            };

            let title = truncate_to_width(&entry.title, 40);
            let url = truncate_to_width(&entry.url, 30);

            ListItem::new(vec![
                Line::from(Span::styled(format!("  {} ", title), style)),
                Line::from(Span::styled(format!("    {}", url), Style::default().fg(TOKYO_COMMENT).bg(bg_color))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(list, panel_area);
}

fn render_help_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(65, 75, area);
    let bg_color = BG_ELEVATED;

    // Fill with elevated background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Companion in help header
    let companion = app.mascot.view();

    let help_text = vec![
        Line::from(vec![
            Span::styled(companion, Style::default().fg(AZUL_BLUE)),
            Span::raw(" "),
            Span::styled("azul", Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD)),
            Span::styled(" help", Style::default().fg(TOKYO_TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("navigate", Style::default().fg(TOKYO_ORANGE).add_modifier(Modifier::BOLD))]),
        Line::from("  /        search or URL"),
        Line::from("  j k      scroll"),
        Line::from("  g G      top / bottom"),
        Line::from("  Tab      cycle focus"),
        Line::from("  Enter    open link"),
        Line::from(""),
        Line::from(vec![Span::styled("tabs", Style::default().fg(TOKYO_PURPLE).add_modifier(Modifier::BOLD))]),
        Line::from("  t        new tab"),
        Line::from("  Ctrl+W   close"),
        Line::from("  1-9      switch"),
        Line::from(""),
        Line::from(vec![Span::styled("tools", Style::default().fg(TOKYO_GREEN).add_modifier(Modifier::BOLD))]),
        Line::from("  b        bookmarks"),
        Line::from("  B        bookmark page"),
        Line::from("  H        history"),
        Line::from("  c        chat"),
        Line::from("  s        summarize"),
        Line::from(""),
        Line::from(vec![Span::styled("other", Style::default().fg(TOKYO_BLUE).add_modifier(Modifier::BOLD))]),
        Line::from("  v        raw view"),
        Line::from("  r        reload"),
        Line::from("  ?        this help"),
        Line::from("  q        quit"),
        Line::from(""),
        Line::from(vec![Span::styled("esc to close", Style::default().fg(TOKYO_COMMENT))]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AZUL_BLUE))
                .title(format!(" {} help ", companion))
                .style(Style::default().bg(bg_color)),
        )
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(help, panel_area);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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

/// Side panel chat - integrates with main content area (Warm Paper theme)
fn render_chat_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let bg_color = BG_SURFACE;  // Same papery feel

    // Fill background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, area);

    // Warm Paper theme colors
    let border_color = if app.chat_focused { BORDER_BRIGHT } else { BORDER_DEFAULT };
    let title = if app.chat_focused { " CHAT " } else { " chat " };

    // Create block with Warm Paper styling
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, Style::default().fg(AZUL_BLUE)))
        .style(Style::default().bg(bg_color));

    // Render block first to fill background
    frame.render_widget(block.clone(), area);

    if app.chat_session.is_none() {
        let empty = Paragraph::new("Configure AI in settings")
            .style(Style::default().fg(TOKYO_COMMENT).bg(bg_color))
            .alignment(Alignment::Center);
        frame.render_widget(empty, area.inner(Margin { horizontal: 1, vertical: 1 }));
        return;
    }

    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    let wrap_width = inner.width.saturating_sub(2).max(20) as usize;

    // Calculate input height based on text length (with wrapping)
    let input_len = app.chat_input.len() + 3; // +3 for "❯ " and "_"
    let input_lines = ((input_len / wrap_width.max(1)) + 1).max(1).min(6) as u16; // 1-6 lines
    let input_height = input_lines + 2; // +2 for border

    // Split: messages | input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(input_height),
        ])
        .split(inner);

    // Render messages with markdown styling
    if let Some(session) = &app.chat_session {
        let mut all_lines: Vec<Line> = Vec::new();
        let md_renderer = StyledMarkdown::new(wrap_width);

        for msg in &session.messages {
            match msg.role {
                crate::chat::Role::System => continue,
                crate::chat::Role::User => {
                    // User message - azul blue prefix, strip browser context for display
                    let display_content = if msg.content.contains("[Browser Context]") {
                        // Extract just the user's actual message after the context
                        msg.content.split("\n\n").last().unwrap_or(&msg.content).to_string()
                    } else {
                        msg.content.clone()
                    };
                    all_lines.push(Line::from(vec![
                        Span::styled("▸ ", Style::default().fg(AZUL_BLUE)),
                        Span::styled(display_content, Style::default().fg(TOKYO_TEXT)),
                    ]));
                    all_lines.push(Line::from(""));
                }
                crate::chat::Role::Assistant => {
                    // Check for tool calls in the message
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            all_lines.push(Line::from(vec![
                                Span::styled("⚙ ", Style::default().fg(TOKYO_ORANGE)),
                                Span::styled(&tc.name, Style::default().fg(TOKYO_ORANGE)),
                            ]));
                        }
                    }
                    // AI message - render as markdown with green prefix
                    if !msg.content.is_empty() {
                        all_lines.push(Line::from(Span::styled("◆ ", Style::default().fg(TOKYO_GREEN))));
                        let content_lines: Vec<String> = msg.content.lines().map(String::from).collect();
                        let styled = md_renderer.render(&content_lines);
                        all_lines.extend(styled);
                    }
                    all_lines.push(Line::from(""));
                }
                crate::chat::Role::Tool => {
                    // Tool result - show minimally
                    all_lines.push(Line::from(vec![
                        Span::styled("  ↳ ", Style::default().fg(TOKYO_COMMENT)),
                        Span::styled(truncate_to_width(&msg.content, wrap_width.saturating_sub(4)), Style::default().fg(TOKYO_COMMENT)),
                    ]));
                }
                _ => {}
            }
        }

        // Pre-wrap lines to get accurate line count for scrolling
        let wrap_width = chunks[0].width.saturating_sub(2) as usize;
        let mut wrapped_lines: Vec<Line> = Vec::new();
        for line in all_lines {
            if line.width() == 0 {
                wrapped_lines.push(line);
            } else {
                // Check if line needs wrapping
                let line_str: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                if line_str.len() > wrap_width && wrap_width > 0 {
                    // Simple wrap - split into chunks
                    for chunk in textwrap::wrap(&line_str, wrap_width) {
                        wrapped_lines.push(Line::from(Span::styled(
                            chunk.to_string(),
                            line.spans.first().map(|s| s.style).unwrap_or_default(),
                        )));
                    }
                } else {
                    wrapped_lines.push(line);
                }
            }
        }

        // Scrolling: chat_scroll = 0 means at bottom (newest), higher = scrolled up
        let visible_height = chunks[0].height as usize;
        let total_lines = wrapped_lines.len();

        // Calculate scroll - show from bottom by default
        let max_scroll = total_lines.saturating_sub(visible_height);
        let clamped_scroll = app.chat_scroll.min(max_scroll);
        let start_line = max_scroll.saturating_sub(clamped_scroll);

        let visible: Vec<Line> = wrapped_lines
            .into_iter()
            .skip(start_line)
            .take(visible_height)
            .collect();

        let msg_para = Paragraph::new(visible)
            .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));
        frame.render_widget(msg_para, chunks[0]);
    }

    // Input area - elevated for focus
    let input_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(BORDER_DEFAULT))
        .style(Style::default().bg(BG_ELEVATED));

    let input_text = format!(" ❯ {}_", app.chat_input);
    let input = Paragraph::new(input_text)
        .block(input_block)
        .style(Style::default().fg(ACCENT_PRIMARY).bg(BG_ELEVATED))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[1]);
}

// Keep old function for compatibility (unused but prevents compile errors)
#[allow(dead_code)]
fn render_chat_panel(frame: &mut Frame, area: Rect, app: &App) {
    render_chat_side_panel(frame, area, app);
}

fn render_settings_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(70, 70, area);
    let bg_color = BG_ELEVATED;

    // Fill with elevated background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_PURPLE))
        .title(" Settings | Esc to close ")
        .style(Style::default().bg(bg_color));

    let settings_text = vec![
        Line::from(vec![Span::styled("Browser Settings", Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("AI Configuration:", Style::default().fg(TOKYO_ORANGE).add_modifier(Modifier::BOLD))]),
        Line::from(""),
    ];

    let mut lines = settings_text;

    // Show AI config
    if let Some(ai) = &app.config.ai {
        if let Some(provider) = &ai.provider {
            lines.push(Line::from(vec![
                Span::raw("  Provider: "),
                Span::styled(provider, Style::default().fg(TOKYO_GREEN)),
            ]));
        }
        if let Some(model) = &ai.model {
            lines.push(Line::from(vec![
                Span::raw("  Model: "),
                Span::styled(model, Style::default().fg(TOKYO_GREEN)),
            ]));
        }
        if let Some(key) = &ai.api_key {
            let masked = if key.len() > 10 {
                format!("{}...{}", &key[..7], &key[key.len()-4..])
            } else {
                "***".to_string()
            };
            lines.push(Line::from(vec![
                Span::raw("  API Key: "),
                Span::styled(masked, Style::default().fg(TOKYO_COMMENT)),
            ]));
        }
        if let Some(models) = &ai.fallback_models {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Fallback Models:", Style::default().fg(TOKYO_COMMENT))]));
            for m in models.iter().take(5) {
                lines.push(Line::from(vec![
                    Span::raw("    - "),
                    Span::styled(m, Style::default().fg(TOKYO_TEXT)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  No AI configuration found", Style::default().fg(TOKYO_COMMENT)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Edit ~/.config/azul/config.json to change settings", Style::default().fg(TOKYO_COMMENT)),
    ]));

    let settings = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(settings, panel_area);
}

fn render_rag_panel(frame: &mut Frame, area: Rect, app: &App) {
    let bg_color = BG_SURFACE;
    // Calculate content area position (same as main content panel: skip 15% links, use 50% content)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Tab bar
            Constraint::Length(3),   // URL bar
            Constraint::Min(0),      // Main content
            Constraint::Length(3),   // Status bar
        ])
        .split(area);

    let content_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),  // Links
            Constraint::Percentage(50),  // Content - this is where RAG goes
            Constraint::Percentage(35),  // Chat
        ])
        .split(main_chunks[2]);

    let panel_area = content_split[1];  // Use the content area

    // Fill with surface background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Loading spinner animation
    let spinner = if app.rag_loading {
        let spinners = ["◐", "◓", "◑", "◒"];
        format!(" {} ", spinners[app.animation_tick % spinners.len()])
    } else {
        String::new()
    };

    let title = if app.rag_loading {
        format!(" RAG{} | Searching... ", spinner)
    } else {
        " RAG | Enter search | j/k scroll | Esc close ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AZUL_BLUE))
        .title(title)
        .style(Style::default().bg(bg_color));

    let inner = panel_area.inner(Margin { horizontal: 1, vertical: 1 });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input area
            Constraint::Min(0),     // Results area
        ])
        .split(inner);

    // Input box - purple border when loading, blue otherwise
    let input_border_color = if app.rag_loading { TOKYO_PURPLE } else { AZUL_BLUE };
    let input_text = if app.rag_loading {
        format!("{} Searching...", spinner)
    } else {
        format!("❯ {}_", app.rag_query)
    };
    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(input_border_color))
                .title(" Query ")
                .style(Style::default().bg(BG_ELEVATED)),
        )
        .style(Style::default().fg(TOKYO_TEXT).bg(BG_ELEVATED));

    frame.render_widget(input, chunks[0]);

    // Results with markdown rendering and scrolling
    let md_renderer = StyledMarkdown::new(chunks[1].width.saturating_sub(4) as usize);
    let results_height = chunks[1].height.saturating_sub(2) as usize;

    let all_lines: Vec<Line> = if app.rag_client.is_none() {
        vec![Line::from(vec![Span::styled(
            "RAG not available. Start home-rag: cd ~/github/home-rag && python -m src.api",
            Style::default().fg(TOKYO_ORANGE),
        )])]
    } else if app.rag_loading {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("  {}  Searching knowledge base...", spinner),
                Style::default().fg(TOKYO_PURPLE),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  This may take a few seconds.",
                Style::default().fg(TOKYO_COMMENT),
            )]),
        ]
    } else if app.rag_results.is_empty() {
        vec![
            Line::from(vec![Span::styled(
                "Enter a query and press Enter to search your knowledge base.",
                Style::default().fg(TOKYO_COMMENT),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "home-rag uses:",
                Style::default().fg(TOKYO_PURPLE),
            )]),
            Line::from(vec![Span::styled(
                "  • Neo4j knowledge graph for relationships",
                Style::default().fg(TOKYO_TEXT),
            )]),
            Line::from(vec![Span::styled(
                "  • Vector embeddings for semantic search",
                Style::default().fg(TOKYO_TEXT),
            )]),
            Line::from(vec![Span::styled(
                "  • LLM (Ollama/OpenRouter) for answer synthesis",
                Style::default().fg(TOKYO_TEXT),
            )]),
        ]
    } else {
        // Render results with markdown styling
        let mut lines: Vec<Line> = Vec::new();
        for result in &app.rag_results {
            let result_lines: Vec<String> = result.lines().map(String::from).collect();
            lines.extend(md_renderer.render(&result_lines));
        }
        lines
    };

    // Calculate scrolling (rag_scroll = 0 means at top, higher = scrolled down)
    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(results_height);
    let start_line = app.rag_scroll.min(max_scroll);

    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(start_line)
        .take(results_height)
        .collect();

    // Show scroll indicator in title if there's more content
    let results_title = if total_lines > results_height {
        let pos = if max_scroll > 0 {
            (start_line * 100) / max_scroll
        } else {
            0
        };
        format!(" Results ({}/{}  {}%) ", start_line + 1, total_lines, pos)
    } else {
        " Results ".to_string()
    };

    let results = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TOKYO_PURPLE))
                .title(results_title)
                .style(Style::default().bg(bg_color)),
        )
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color))
        .wrap(Wrap { trim: false });

    frame.render_widget(results, chunks[1]);
    frame.render_widget(block, panel_area);
}

fn render_memory_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(70, 70, area);
    let bg_color = BG_ELEVATED;

    // Fill with elevated background first
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_CYAN))
        .title(" Memory Graph (mem-layer) | r to refresh | Esc to close ")
        .style(Style::default().bg(bg_color));

    let status = if app.memory_client.is_none() {
        vec![Line::from(vec![Span::styled(
            "mem-layer not available. Install it from: ~/.local/bin/mem-layer",
            Style::default().fg(TOKYO_RED),
        )])]
    } else if app.memory_nodes.is_empty() {
        vec![Line::from(vec![Span::styled(
            "No memory nodes found. Memory graph is empty.",
            Style::default().fg(TOKYO_COMMENT),
        )])]
    } else {
        let mut lines = vec![Line::from(vec![Span::styled(
            format!("Memory Nodes ({}):", app.memory_nodes.len()),
            Style::default().fg(TOKYO_GREEN).add_modifier(Modifier::BOLD),
        )])];
        lines.push(Line::from(""));

        for (i, node) in app.memory_nodes.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(AZUL_BLUE)),
                Span::styled(node, Style::default().fg(TOKYO_TEXT)),
            ]));
        }

        lines
    };

    let memory = Paragraph::new(status)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color))
        .wrap(Wrap { trim: true });

    frame.render_widget(memory, panel_area);
}

