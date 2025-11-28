mod neon;
pub mod markdown;
pub mod theme;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, PanelMode, ViewMode};
use crate::mascot;
use neon::NeonBorder;
use markdown::StyledMarkdown;

// Tokyo Night color palette (made pub for markdown module)
pub const AZUL_BLUE: Color = Color::Rgb(0, 191, 255);
pub const TOKYO_BLUE: Color = Color::Rgb(122, 162, 247);
pub const TOKYO_PURPLE: Color = Color::Rgb(187, 154, 247);
pub const TOKYO_ORANGE: Color = Color::Rgb(255, 169, 0);
pub const TOKYO_GREEN: Color = Color::Rgb(158, 206, 106);
pub const TOKYO_RED: Color = Color::Rgb(247, 118, 142);
pub const TOKYO_TEXT: Color = Color::Rgb(192, 202, 245);
pub const TOKYO_COMMENT: Color = Color::Rgb(125, 137, 168);
pub const TOKYO_BG: Color = Color::Rgb(26, 27, 38);
pub const TOKYO_CYAN: Color = Color::Rgb(125, 207, 255);

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

    // Main layout: Tab Bar | URL Bar | Content | Status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Tab bar
            Constraint::Length(3),   // URL bar
            Constraint::Min(0),      // Main content
            Constraint::Length(3),   // Status bar
        ])
        .split(inner);

    render_tab_bar(frame, chunks[0], app);
    render_url_bar(frame, chunks[1], app);

    // Always clear the main content area to prevent artifacts
    frame.render_widget(Clear, chunks[2]);

    // Chat mode: split into content and chat (hides links)
    if app.panel_mode == PanelMode::Chat {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(65),
                Constraint::Percentage(35),
            ])
            .split(chunks[2]);
        render_content_only(frame, split[0], app);
        render_chat_side_panel(frame, split[1], app);
    } else {
        render_main_content(frame, chunks[2], app);
    }

    render_status_bar(frame, chunks[3], app);

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

    let mut spans = vec![];

    for (i, tab) in tabs.iter().enumerate() {
        let is_active = i == active;
        let title = tab.display_title(15);

        let style = if is_active {
            Style::default().fg(TOKYO_BG).bg(AZUL_BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TOKYO_TEXT)
        };

        // Tab number
        spans.push(Span::styled(format!(" {} ", i + 1), Style::default().fg(TOKYO_COMMENT)));
        // Tab title
        spans.push(Span::styled(title, style));

        if tab.loading {
            spans.push(Span::styled(" *", Style::default().fg(TOKYO_ORANGE)));
        }

        spans.push(Span::raw(" |"));
    }

    // Add new tab hint if not full
    if !app.tabs.is_full() {
        spans.push(Span::styled(" + (t)", Style::default().fg(TOKYO_COMMENT)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line)
        .style(Style::default().fg(TOKYO_TEXT));

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
    let border_color = if is_focused { AZUL_BLUE } else { TOKYO_BLUE };

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
                .title(if is_focused { " URL (editing) " } else { " URL " }),
        )
        .style(if is_focused {
            Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TOKYO_TEXT)
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
    let border_color = if is_focused { AZUL_BLUE } else { TOKYO_BLUE };

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
        .style(Style::default().bg(TOKYO_BG));

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
            .style(Style::default().fg(TOKYO_TEXT))
            .wrap(Wrap { trim: false });

        frame.render_widget(content, area);
    } else {
        // Show splash with mascot when no page loaded
        let splash_text = mascot::mini_splash();
        let placeholder = Paragraph::new(splash_text)
            .block(block)
            .style(Style::default().fg(AZUL_BLUE))
            .alignment(Alignment::Center);

        frame.render_widget(placeholder, area);
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focus == Focus::Sidebar;
    let border_color = if is_focused { TOKYO_ORANGE } else { TOKYO_BLUE };

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
        .title(title);

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
                    Style::default().fg(TOKYO_BG).bg(TOKYO_ORANGE).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(TOKYO_ORANGE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TOKYO_TEXT)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(number, Style::default().fg(TOKYO_COMMENT)),
                    Span::raw(" "),
                    Span::styled(truncated, style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .style(Style::default().fg(TOKYO_TEXT));

        frame.render_widget(list, area);
    } else {
        frame.render_widget(block, area);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let focus_name = match app.focus {
        Focus::Content => "Content",
        Focus::Sidebar => "Sidebar",
        Focus::URLBar => "URL Bar",
        Focus::TabBar => "Tab Bar",
        Focus::Bookmarks => "Bookmarks",
        Focus::History => "History",
    };

    // Tab info
    let tab_info = format!("[{}/{}]", app.tabs.active_index() + 1, app.tabs.count());

    // Companion follows you in the status bar
    let companion = app.mascot.view_status();

    let primary_line = Line::from(vec![
        Span::styled(companion, Style::default().fg(AZUL_BLUE)),
        Span::raw(" "),
        Span::styled(tab_info, Style::default().fg(TOKYO_PURPLE)),
        Span::raw(" "),
        Span::styled(focus_name, Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(&app.status_message, Style::default().fg(TOKYO_TEXT)),
    ]);

    let shortcuts = "/ search  t tab  b bookmarks  H history  c chat  ? help  q quit";
    let secondary_line = Line::from(vec![
        Span::styled(shortcuts, Style::default().fg(TOKYO_COMMENT)),
    ]);

    let status = Paragraph::new(vec![primary_line, secondary_line])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TOKYO_BLUE)),
        )
        .style(Style::default().fg(TOKYO_TEXT));

    frame.render_widget(status, area);
}

fn render_bookmarks_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);

    // Clear the area first
    frame.render_widget(Clear, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_ORANGE))
        .title(format!(" Bookmarks ({}) | j/k nav | Enter open | d delete | Esc close ", app.bookmarks_list.len()))
        .style(Style::default().bg(TOKYO_BG));

    let inner = panel_area.inner(Margin { horizontal: 1, vertical: 1 });
    let visible_rows = inner.height.saturating_sub(1) as usize;

    if app.bookmarks_list.is_empty() {
        let empty = Paragraph::new("No bookmarks yet. Press B to bookmark current page.")
            .block(block)
            .style(Style::default().fg(TOKYO_COMMENT))
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
                Style::default().fg(TOKYO_BG).bg(TOKYO_ORANGE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TOKYO_TEXT)
            };

            let title = truncate_to_width(&bookmark.title, 40);
            let url = truncate_to_width(&bookmark.url, 30);

            ListItem::new(vec![
                Line::from(Span::styled(format!("  {} ", title), style)),
                Line::from(Span::styled(format!("    {}", url), Style::default().fg(TOKYO_COMMENT))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT));

    frame.render_widget(list, panel_area);
}

fn render_history_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);

    frame.render_widget(Clear, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_PURPLE))
        .title(format!(" History ({}) | j/k nav | Enter open | Esc close ", app.history_list.len()))
        .style(Style::default().bg(TOKYO_BG));

    let inner = panel_area.inner(Margin { horizontal: 1, vertical: 1 });
    let visible_rows = inner.height.saturating_sub(1) as usize;

    if app.history_list.is_empty() {
        let empty = Paragraph::new("No history yet. Start browsing!")
            .block(block)
            .style(Style::default().fg(TOKYO_COMMENT))
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
                Style::default().fg(TOKYO_BG).bg(TOKYO_PURPLE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TOKYO_TEXT)
            };

            let title = truncate_to_width(&entry.title, 40);
            let url = truncate_to_width(&entry.url, 30);

            ListItem::new(vec![
                Line::from(Span::styled(format!("  {} ", title), style)),
                Line::from(Span::styled(format!("    {}", url), Style::default().fg(TOKYO_COMMENT))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT));

    frame.render_widget(list, panel_area);
}

fn render_help_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(65, 75, area);

    frame.render_widget(Clear, panel_area);

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
                .style(Style::default().bg(TOKYO_BG)),
        )
        .style(Style::default().fg(TOKYO_TEXT));

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

// Cosmic dark theme for chat panel - deep space with subtle purple glow
const COSMIC_BG: Color = Color::Rgb(13, 13, 23);           // Deep space black
const COSMIC_BORDER: Color = Color::Rgb(88, 66, 139);      // Cosmic purple
const COSMIC_ACCENT: Color = Color::Rgb(138, 99, 210);     // Nebula violet
const COSMIC_TEXT_DIM: Color = Color::Rgb(140, 140, 170);  // Starlight dim
const COSMIC_GLOW: Color = Color::Rgb(100, 149, 237);      // Cornflower blue glow

/// Side panel chat - integrates with main content area (cosmic dark theme)
fn render_chat_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    // Clear the area first to avoid artifacts
    frame.render_widget(Clear, area);

    // Cosmic themed border colors
    let border_color = if app.chat_focused { COSMIC_GLOW } else { COSMIC_BORDER };
    let title = if app.chat_focused { " CHAT " } else { " chat " };

    // Create block first and render it to establish cosmic background
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, Style::default().fg(COSMIC_ACCENT)))
        .style(Style::default().bg(COSMIC_BG));

    // Render block first to fill background
    frame.render_widget(block.clone(), area);

    if app.chat_session.is_none() {
        let empty = Paragraph::new("Configure AI in settings")
            .style(Style::default().fg(COSMIC_TEXT_DIM).bg(COSMIC_BG))
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
                    // User message - cosmic accent prefix
                    all_lines.push(Line::from(vec![
                        Span::styled("▸ ", Style::default().fg(COSMIC_GLOW)),
                        Span::styled(&msg.content, Style::default().fg(TOKYO_TEXT)),
                    ]));
                    all_lines.push(Line::from(""));
                }
                crate::chat::Role::Assistant => {
                    // AI message - render as markdown with cosmic prefix
                    all_lines.push(Line::from(Span::styled("◆ ", Style::default().fg(COSMIC_ACCENT))));
                    let content_lines: Vec<String> = msg.content.lines().map(String::from).collect();
                    let styled = md_renderer.render(&content_lines);
                    all_lines.extend(styled);
                    all_lines.push(Line::from(""));
                }
                _ => {}
            }
        }

        // Calculate visible area with scrolling
        // chat_scroll represents "lines scrolled back from bottom"
        // 0 = showing newest messages, higher = scrolled back to see older
        let visible_height = chunks[0].height as usize;
        let total_lines = all_lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_back = (app.chat_scroll as usize).min(max_scroll);
        let scroll_from_top = max_scroll.saturating_sub(scroll_back);

        let visible: Vec<Line> = all_lines
            .into_iter()
            .skip(scroll_from_top)
            .take(visible_height)
            .collect();

        let msg_para = Paragraph::new(visible)
            .style(Style::default().fg(TOKYO_TEXT).bg(COSMIC_BG))
            .wrap(Wrap { trim: false });
        frame.render_widget(msg_para, chunks[0]);
    }

    // Input area with cosmic styling
    let input_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(COSMIC_BORDER))
        .style(Style::default().bg(COSMIC_BG));

    let input_text = format!("❯ {}_", app.chat_input);
    let input = Paragraph::new(input_text)
        .block(input_block)
        .style(Style::default().fg(COSMIC_GLOW).bg(COSMIC_BG))
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
    frame.render_widget(Clear, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_PURPLE))
        .title(" Settings | Esc to close ")
        .style(Style::default().bg(TOKYO_BG));

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
        .style(Style::default().fg(TOKYO_TEXT));

    frame.render_widget(settings, panel_area);
}

fn render_rag_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_PURPLE))
        .title(" RAG Query | Enter to search | Esc to close ")
        .style(Style::default().bg(TOKYO_BG));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input area
            Constraint::Min(0),     // Results area
        ])
        .split(panel_area.inner(Margin { horizontal: 1, vertical: 1 }));

    // Input box
    let input_text = format!("{}_", app.rag_query);
    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AZUL_BLUE))
                .title(" Query "),
        )
        .style(Style::default().fg(TOKYO_TEXT));

    frame.render_widget(input, chunks[0]);

    // Results
    let status = if app.rag_client.is_none() {
        vec![Line::from(vec![Span::styled(
            "RAG service not available. Check home-rag at http://localhost:8000",
            Style::default().fg(TOKYO_RED),
        )])]
    } else if app.rag_results.is_empty() {
        vec![Line::from(vec![Span::styled(
            "No results yet. Enter a query and press Enter.",
            Style::default().fg(TOKYO_COMMENT),
        )])]
    } else {
        let mut lines = vec![Line::from(vec![Span::styled(
            format!("Found {} results:", app.rag_results.len()),
            Style::default().fg(TOKYO_GREEN).add_modifier(Modifier::BOLD),
        )])];
        lines.push(Line::from(""));

        for (i, result) in app.rag_results.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(TOKYO_ORANGE)),
                Span::styled(result, Style::default().fg(TOKYO_TEXT)),
            ]));
            lines.push(Line::from(""));
        }

        lines
    };

    let results = Paragraph::new(status)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TOKYO_BLUE))
                .title(" Results "),
        )
        .style(Style::default().fg(TOKYO_TEXT))
        .wrap(Wrap { trim: true });

    frame.render_widget(results, chunks[1]);
    frame.render_widget(block, panel_area);
}

fn render_memory_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(70, 70, area);
    frame.render_widget(Clear, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TOKYO_PURPLE))
        .title(" Memory Graph (mem-layer) | r to refresh | Esc to close ")
        .style(Style::default().bg(TOKYO_BG));

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
                Span::styled(format!("{}. ", i + 1), Style::default().fg(TOKYO_ORANGE)),
                Span::styled(node, Style::default().fg(TOKYO_TEXT)),
            ]));
        }

        lines
    };

    let memory = Paragraph::new(status)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT))
        .wrap(Wrap { trim: true });

    frame.render_widget(memory, panel_area);
}

