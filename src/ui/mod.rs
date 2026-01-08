pub mod markdown;
pub mod panes;
pub mod shadow;
pub mod theme;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, PanelMode, ViewMode};
use crate::mascot;
use markdown::StyledMarkdown;
use panes::PaneContext;

// OpenCode-inspired clean dark theme - minimal, professional
// Background layers with clear hierarchy
pub const BG_BASE: Color = Color::Rgb(17, 17, 19); // Deep dark #111113
pub const BG_SURFACE: Color = Color::Rgb(24, 24, 27); // Panel surface #18181b
pub const BG_ELEVATED: Color = Color::Rgb(32, 32, 36); // Elevated/focused #202024
pub const BG_HIGHLIGHT: Color = Color::Rgb(45, 45, 50); // Selection highlight #2d2d32

// Border colors - subtle, clean lines
pub const BORDER_DIM: Color = Color::Rgb(40, 40, 45); // Subtle separator #28282d
pub const BORDER_DEFAULT: Color = Color::Rgb(55, 55, 62); // Normal border #37373e
pub const BORDER_BRIGHT: Color = Color::Rgb(82, 139, 255); // Focus blue #528bff

// Accent colors - clean and modern
pub const ACCENT_PRIMARY: Color = Color::Rgb(82, 139, 255); // Primary blue #528bff
pub const ACCENT_SECONDARY: Color = Color::Rgb(150, 120, 200); // Purple #9678c8
pub const ACCENT_SUCCESS: Color = Color::Rgb(80, 200, 120); // Green #50c878
pub const ACCENT_WARNING: Color = Color::Rgb(255, 180, 80); // Orange #ffb450
pub const ACCENT_ERROR: Color = Color::Rgb(255, 100, 100); // Red #ff6464
pub const ACCENT_INFO: Color = Color::Rgb(100, 180, 255); // Light blue #64b4ff

// Text colors - clean whites and grays
pub const TEXT_PRIMARY: Color = Color::Rgb(229, 231, 235); // Near white #e5e7eb
pub const TEXT_SECONDARY: Color = Color::Rgb(156, 163, 175); // Gray #9ca3af
pub const TEXT_DIM: Color = Color::Rgb(107, 114, 128); // Muted gray #6b7280

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

/// Creates a pulsing focus color based on animation phase.
/// Uses sine wave interpolation for smooth breathing effect.
pub fn pulsing_focus_color(phase: u8) -> Color {
    // Convert phase (0-255) to a sine wave (0.0 to 1.0 to 0.0)
    let intensity = ((phase as f32 / 255.0) * std::f32::consts::PI).sin();

    // Base color is ACCENT_PRIMARY blue: RGB(82, 139, 255)
    // Pulse between 70% and 100% brightness
    let factor = 0.7 + (intensity * 0.3);
    Color::Rgb((82.0 * factor) as u8 + 40, (139.0 * factor) as u8 + 30, 255)
}

/// Panel background - consistent dark base for all panels (no distracting highlight changes)
pub fn focused_bg(_focused: bool) -> Color {
    BG_BASE
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();
    if full.width < 4 || full.height < 4 {
        return;
    }

    // Fill entire frame with base background
    let bg_fill = Block::default().style(Style::default().bg(BG_BASE));
    frame.render_widget(bg_fill, full);

    // Render shadow border for depth effect (light on top/left, shadow on bottom/right)
    let shadow_border = shadow::ShadowBorder::new(BORDER_DIM).rounded(true);
    frame.render_widget(shadow_border, full);

    let inner = full.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Main layout with spacing
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Length(3), // URL bar
            Constraint::Length(1), // Spacer
            Constraint::Min(0),    // Main content
            Constraint::Length(2), // Status bar (compact)
        ])
        .split(inner);

    render_tab_bar(frame, chunks[0], app);
    render_url_bar(frame, chunks[1], app);
    // chunks[2] is spacer - empty

    let content_area = chunks[3];
    let context = PaneContext {
        focus: app.focus,
        chat_focused: app.chat_focused,
        hard_content: app.is_hard_content(),
    };
    let layout = app.panes.layout(content_area, context);

    if let Some(sidebar) = layout.sidebar {
        render_compact_sidebar(frame, sidebar, app);
    }
    if let Some(content) = layout.content {
        render_content_only(frame, content, app);
    }
    if let Some(assistant) = layout.assistant {
        render_chat_side_panel(frame, assistant, app);
    }

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
            Style::default()
                .fg(BG_BASE)
                .bg(AZUL_BLUE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TOKYO_TEXT).bg(bg_color)
        };

        // Tab number
        spans.push(Span::styled(
            format!(" {} ", i + 1),
            Style::default().fg(TOKYO_COMMENT).bg(bg_color),
        ));
        // Tab title
        spans.push(Span::styled(title, style));

        if tab.loading {
            spans.push(Span::styled(
                " *",
                Style::default().fg(TOKYO_ORANGE).bg(bg_color),
            ));
        }

        spans.push(Span::styled(" |", Style::default().bg(bg_color)));
    }

    // Add new tab hint if not full
    if !app.tabs.is_full() {
        spans.push(Span::styled(
            " + (t)",
            Style::default().fg(TOKYO_COMMENT).bg(bg_color),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(paragraph, area);
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    // Check if current page is bookmarked
    let bookmark_indicator = if app.is_current_bookmarked() {
        Span::styled(" [*]", Style::default().fg(TOKYO_ORANGE))
    } else {
        Span::raw("")
    };

    // Companion in title - always visible
    let companion = app.mascot.view();

    let title = vec![
        Span::styled(companion, Style::default().fg(AZUL_BLUE)),
        Span::raw(" "),
        Span::styled(
            "azul",
            Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD),
        ),
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
    // Pulsing focus color for smooth animation
    let border_color = if is_focused {
        pulsing_focus_color(app.focus_pulse_phase)
    } else {
        BORDER_DEFAULT
    };
    let bg_color = focused_bg(is_focused);

    // Companion appears in URL bar when active
    let companion_prefix = match app.mascot.state() {
        mascot::CompanionState::Loading | mascot::CompanionState::Searching => {
            format!("{} ", app.mascot.view())
        }
        _ => String::new(),
    };

    // Add padding to display text
    let display_text = if is_focused {
        format!("  {}{}_", companion_prefix, app.url_input)
    } else if let Some(page) = app.current_page() {
        format!("  {}{}", companion_prefix, page.url)
    } else {
        format!("  {}/ to search...", companion_prefix)
    };

    // Title style changes with focus
    let title_style = if is_focused {
        Style::default()
            .fg(ACCENT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_DIM)
    };

    let url_paragraph = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(bg_color))
                .title(Span::styled(
                    if is_focused {
                        " URL (editing) "
                    } else {
                        " URL "
                    },
                    title_style,
                )),
        )
        .style(if is_focused {
            Style::default()
                .fg(ACCENT_PRIMARY)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_PRIMARY).bg(bg_color)
        });

    frame.render_widget(url_paragraph, area);
}

fn render_main_content(frame: &mut Frame, area: Rect, app: &mut App) {
    // Split into content and sidebar
    let has_sidebar = app
        .current_page()
        .map(|p| !p.links.is_empty())
        .unwrap_or(false);

    if has_sidebar {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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
    // Pulsing focus color for smooth animation when focused
    let border_color = if is_focused {
        pulsing_focus_color(app.focus_pulse_phase)
    } else {
        BORDER_DIM
    };
    // Elevated background when focused for depth
    let bg_color = focused_bg(is_focused);

    let mode_label = match app.view_mode {
        ViewMode::Rendered => "content",
        ViewMode::Raw => "raw",
    };

    // Title style changes with focus
    let title_style = if is_focused {
        Style::default()
            .fg(ACCENT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_DIM)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            if is_focused {
                format!(" {} ", mode_label.to_uppercase())
            } else {
                format!(" {} ", mode_label)
            },
            title_style,
        ))
        .style(Style::default().bg(bg_color));

    // Render the chrome first; we render the paragraph separately so we can center a reading column.
    frame.render_widget(block.clone(), area);

    if let Some(page) = app.current_page() {
        // Inner area excluding borders and a small gutter
        let inner = area
            .inner(Margin {
                horizontal: 1,
                vertical: 1,
            })
            .inner(Margin {
                horizontal: 1,
                vertical: 0,
            });

        // Optimal reading column: 66-72 chars for readability (research-backed)
        // See: https://baymard.com/blog/line-length-readability
        let max_column_width: u16 = 72;
        let column_width = inner.width.min(max_column_width).max(20);
        let column_x = inner.x + (inner.width.saturating_sub(column_width) / 2);
        let column = Rect {
            x: column_x,
            y: inner.y,
            width: column_width,
            height: inner.height,
        };

        let visible_height = column.height.max(1) as usize;
        let wrap_width = column.width.saturating_sub(1).max(20) as usize;

        let mut lines: Vec<String> = match app.view_mode {
            ViewMode::Rendered => page.content_lines.clone(),
            ViewMode::Raw => page.raw_content.lines().map(|s| s.to_string()).collect(),
        };

        if app.view_mode == ViewMode::Rendered && app.format_text {
            lines = format_lines(&lines, wrap_width);
        }

        // Prepend AI summary if available
        if let Some(summary) = &app.ai_summary {
            let mut summary_lines = vec!["# AI Summary".to_string(), String::new()];
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

        let scroll = app.scroll_offset();
        let max_scroll = lines.len().saturating_sub(visible_height);
        let start = scroll.min(max_scroll);
        let end = (start + visible_height).min(lines.len());

        // Use styled markdown rendering for rendered view
        let visible_lines: Vec<Line> = if app.view_mode == ViewMode::Rendered {
            let md_renderer = StyledMarkdown::new(wrap_width);
            md_renderer.render(&lines[start..end])
        } else {
            // Raw view - plain text
            lines[start..end]
                .iter()
                .map(|line| Line::from(line.clone()))
                .collect()
        };

        let content = Paragraph::new(visible_lines)
            .style(Style::default().fg(TOKYO_TEXT).bg(bg_color))
            .wrap(Wrap { trim: false });

        frame.render_widget(content, column);
    } else {
        // Show splash with mascot when no page loaded
        let splash_text = mascot::mini_splash();
        let placeholder = Paragraph::new(splash_text)
            .style(Style::default().fg(AZUL_BLUE).bg(bg_color))
            .alignment(Alignment::Center);

        let inner = area
            .inner(Margin {
                horizontal: 1,
                vertical: 1,
            })
            .inner(Margin {
                horizontal: 2,
                vertical: 1,
            });
        frame.render_widget(placeholder, inner);
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
        let inner = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
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
                // Subtle selection: just color change, no heavy background
                let (num_style, text_style) = if is_selected && is_focused {
                    (
                        Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD),
                        Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD),
                    )
                } else if is_selected {
                    (
                        Style::default().fg(TEXT_SECONDARY),
                        Style::default()
                            .fg(TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default().fg(TEXT_DIM),
                        Style::default().fg(TEXT_SECONDARY),
                    )
                };

                ListItem::new(Line::from(vec![
                    Span::styled(number, num_style),
                    Span::styled(" ", Style::default()),
                    Span::styled(truncated, text_style),
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
    // Elevated background when focused
    let bg_color = focused_bg(is_focused);
    // Pulsing focus color
    let border_color = if is_focused {
        pulsing_focus_color(app.focus_pulse_phase)
    } else {
        BORDER_DIM
    };

    let total_links = app.current_page().map(|p| p.links.len()).unwrap_or(0);
    let selected = app.sidebar_selected();

    // Title style changes with focus
    let title_style = if is_focused {
        Style::default()
            .fg(ACCENT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_DIM)
    };
    let title = Span::styled(format!(" {} ", total_links), title_style);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1))
        .title(title)
        .style(Style::default().bg(bg_color));

    if let Some(page) = app.current_page() {
        // Increased padding: horizontal 2, vertical 2
        let inner = area.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });
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
                // Subtle selection styling - no heavy backgrounds
                let style = if is_selected && is_focused {
                    Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default()
                        .fg(TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT_SECONDARY)
                };

                // Show number + truncated link text
                let text = if link.text.is_empty() {
                    &link.url
                } else {
                    &link.text
                };
                let truncated = truncate_to_width(text, max_text_width);
                ListItem::new(Line::from(Span::styled(
                    format!("{:2} {}", i + 1, truncated),
                    style,
                )))
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
    // Clean, minimal status bar - OpenCode style
    let tab_info = format!("{}/{}", app.tabs.active_index() + 1, app.tabs.count());
    let companion = app.mascot.view_status();

    // Left side: status indicator + tab info + message
    // Right side: minimal hints
    let status_line = Line::from(vec![
        Span::styled("  ", Style::default().bg(BG_BASE)),
        Span::styled(companion, Style::default().fg(ACCENT_PRIMARY)),
        Span::styled("  ", Style::default().bg(BG_BASE)),
        Span::styled(tab_info, Style::default().fg(TEXT_DIM)),
        Span::styled("  ", Style::default().bg(BG_BASE)),
        Span::styled(&app.status_message, Style::default().fg(TEXT_SECONDARY)),
        Span::styled("   ", Style::default().bg(BG_BASE)),
        Span::styled("|", Style::default().fg(BORDER_DIM)),
        Span::styled("   ", Style::default().bg(BG_BASE)),
        Span::styled("/", Style::default().fg(TEXT_DIM)),
        Span::styled(" search  ", Style::default().fg(TEXT_DIM)),
        Span::styled("?", Style::default().fg(TEXT_DIM)),
        Span::styled(" help  ", Style::default().fg(TEXT_DIM)),
        Span::styled("q", Style::default().fg(TEXT_DIM)),
        Span::styled(" quit", Style::default().fg(TEXT_DIM)),
    ]);

    let status = Paragraph::new(status_line).style(Style::default().bg(BG_BASE));

    frame.render_widget(status, area);
}

fn render_bookmarks_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);
    let bg_color = BG_ELEVATED;

    // CRITICAL: Clear the area first to remove any content behind the overlay
    frame.render_widget(Clear, panel_area);

    // Fill with elevated background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Pulsing border for overlay panels (always focused when visible)
    let border_color = pulsing_focus_color(app.focus_pulse_phase);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(2, 2, 1, 1)) // Generous padding
        .title(Span::styled(
            format!(" Bookmarks ({}) ", app.bookmarks_list.len()),
            Style::default()
                .fg(ACCENT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " j/k scroll  Enter open  d del  Esc close ",
            Style::default().fg(TEXT_DIM),
        ))
        .style(Style::default().bg(bg_color));

    // Generous padding: horizontal 3, vertical 2
    let inner = panel_area.inner(Margin {
        horizontal: 3,
        vertical: 2,
    });
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

    let items: Vec<ListItem> = app
        .bookmarks_list
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_rows)
        .map(|(i, bookmark)| {
            let is_selected = i == app.bookmarks_selected;
            let style = if is_selected {
                Style::default()
                    .fg(BG_BASE)
                    .bg(AZUL_BLUE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TOKYO_TEXT).bg(bg_color)
            };

            let title = truncate_to_width(&bookmark.title, 40);
            let url = truncate_to_width(&bookmark.url, 30);

            ListItem::new(vec![
                Line::from(Span::styled(format!("  {} ", title), style)),
                Line::from(Span::styled(
                    format!("    {}", url),
                    Style::default().fg(TOKYO_COMMENT).bg(bg_color),
                )),
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

    // CRITICAL: Clear the area first to remove any content behind the overlay
    frame.render_widget(Clear, panel_area);

    // Fill with elevated background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Pulsing border for overlay panels
    let border_color = pulsing_focus_color(app.focus_pulse_phase);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(2, 2, 1, 1)) // Generous padding
        .title(Span::styled(
            format!(" History ({}) ", app.history_list.len()),
            Style::default()
                .fg(ACCENT_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " j/k scroll  Enter open  Esc close ",
            Style::default().fg(TEXT_DIM),
        ))
        .style(Style::default().bg(bg_color));

    // Generous padding: horizontal 3, vertical 2
    let inner = panel_area.inner(Margin {
        horizontal: 3,
        vertical: 2,
    });
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

    let items: Vec<ListItem> = app
        .history_list
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_rows)
        .map(|(i, entry)| {
            let is_selected = i == app.history_selected;
            let style = if is_selected {
                Style::default()
                    .fg(BG_BASE)
                    .bg(TOKYO_PURPLE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TOKYO_TEXT).bg(bg_color)
            };

            let title = truncate_to_width(&entry.title, 40);
            let url = truncate_to_width(&entry.url, 30);

            ListItem::new(vec![
                Line::from(Span::styled(format!("  {} ", title), style)),
                Line::from(Span::styled(
                    format!("    {}", url),
                    Style::default().fg(TOKYO_COMMENT).bg(bg_color),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(list, panel_area);
}

fn render_help_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(55, 70, area);
    let bg_color = BG_ELEVATED;

    // CRITICAL: Clear the area first to remove any content behind the overlay
    frame.render_widget(Clear, panel_area);

    // Fill with elevated background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Pulsing border for overlay panels
    let border_color = pulsing_focus_color(app.focus_pulse_phase);

    // Companion in help header
    let companion = app.mascot.view();

    let help_text = vec![
        Line::from(vec![
            Span::styled(companion, Style::default().fg(AZUL_BLUE)),
            Span::raw(" "),
            Span::styled(
                "azul",
                Style::default().fg(AZUL_BLUE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" help", Style::default().fg(TOKYO_TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "navigate",
            Style::default()
                .fg(TOKYO_ORANGE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  /        search or URL"),
        Line::from("  H L      back / forward"),
        Line::from("  j k      scroll line"),
        Line::from("  J K      scroll 5 lines"),
        Line::from("  ^D ^U    half page"),
        Line::from("  Space    page down"),
        Line::from("  g G      top / bottom"),
        Line::from("  Tab      cycle focus"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "tabs",
            Style::default()
                .fg(TOKYO_PURPLE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  t        new tab"),
        Line::from("  Ctrl+W   close"),
        Line::from("  1-9      switch"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "tools",
            Style::default()
                .fg(TOKYO_GREEN)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  b        bookmarks"),
        Line::from("  B        bookmark page"),
        Line::from("  H        history"),
        Line::from("  r        rag panel"),
        Line::from("  m        memory panel"),
        Line::from("  c        chat panel"),
        Line::from("  s        summarize"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "other",
            Style::default().fg(TOKYO_BLUE).add_modifier(Modifier::BOLD),
        )]),
        Line::from("  v        raw view"),
        Line::from("  f        reflow text"),
        Line::from("  z        focus mode"),
        Line::from("  r        reload"),
        Line::from("  ?        this help"),
        Line::from("  q        quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "tips",
            Style::default().fg(TOKYO_COMMENT),
        )]),
        Line::from("  Open [img] links to render images"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "esc to close",
            Style::default().fg(TOKYO_COMMENT),
        )]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(border_color))
                .padding(Padding::new(3, 3, 1, 1)) // Extra generous padding for help
                .title(Span::styled(
                    format!(" {} azul ", companion),
                    Style::default()
                        .fg(ACCENT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .title_bottom(Span::styled(" Esc close ", Style::default().fg(TEXT_DIM)))
                .style(Style::default().bg(bg_color)),
        )
        .style(Style::default().fg(TEXT_PRIMARY).bg(bg_color));

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

    // Markdown-aware reflow: wrap prose while preserving structure (headers, lists, code blocks, etc.)
    let mut out: Vec<String> = Vec::new();
    let mut paragraph = String::new();
    let mut in_code_block = false;

    let flush_paragraph = |out: &mut Vec<String>, paragraph: &mut String| {
        if paragraph.trim().is_empty() {
            paragraph.clear();
            return;
        }
        for wrapped in textwrap::wrap(paragraph.trim(), max_width) {
            out.push(wrapped.into_owned());
        }
        paragraph.clear();
    };

    for line in lines {
        let raw = line.trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();

        // Code fences: preserve exactly and stop reflow inside.
        if trimmed.starts_with("```") {
            flush_paragraph(&mut out, &mut paragraph);
            out.push(trimmed.to_string());
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            out.push(raw.to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut out, &mut paragraph);
            out.push(String::new());
            continue;
        }

        if is_markdown_block_line(trimmed) {
            flush_paragraph(&mut out, &mut paragraph);
            out.extend(wrap_structured_line(raw, max_width));
            continue;
        }

        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(trimmed);
    }

    flush_paragraph(&mut out, &mut paragraph);
    out
}

fn is_markdown_block_line(trimmed: &str) -> bool {
    if trimmed.starts_with("# ")
        || trimmed.starts_with("## ")
        || trimmed.starts_with("### ")
        || trimmed.starts_with("#### ")
        || trimmed == "---"
        || trimmed == "***"
        || trimmed == "___"
        || trimmed.starts_with("> ")
        || trimmed.starts_with("![")
        || trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || trimmed.starts_with("- [ ] ")
        || trimmed.starts_with("- [x] ")
        || trimmed.starts_with("- [X] ")
    {
        return true;
    }

    Regex::new(r"^\\d+\\.\\s+")
        .ok()
        .is_some_and(|re| re.is_match(trimmed))
}

fn wrap_structured_line(raw: &str, max_width: usize) -> Vec<String> {
    let trimmed = raw.trim_end();
    let t = trimmed.trim_start();

    if let Some(stripped) = t.strip_prefix("> ") {
        return wrap_with_prefix(trimmed, "> ", stripped, max_width);
    }
    if let Some(stripped) = t.strip_prefix("- [ ] ") {
        return wrap_list_like(trimmed, "- [ ] ", stripped, max_width);
    }
    if let Some(stripped) = t.strip_prefix("- [x] ") {
        return wrap_list_like(trimmed, "- [x] ", stripped, max_width);
    }
    if let Some(stripped) = t.strip_prefix("- [X] ") {
        return wrap_list_like(trimmed, "- [x] ", stripped, max_width);
    }
    if let Some(stripped) = t.strip_prefix("- ") {
        return wrap_list_like(trimmed, "- ", stripped, max_width);
    }
    if let Some(stripped) = t.strip_prefix("* ") {
        return wrap_list_like(trimmed, "* ", stripped, max_width);
    }
    if let Some(stripped) = t.strip_prefix("+ ") {
        return wrap_list_like(trimmed, "+ ", stripped, max_width);
    }

    if let Some(caps) = Regex::new(r"^(\\d+\\.\\s+)(.*)$")
        .ok()
        .and_then(|re| re.captures(t))
    {
        let marker = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        return wrap_list_like(trimmed, marker, text, max_width);
    }

    vec![trimmed.to_string()]
}

fn wrap_with_prefix(raw: &str, prefix: &str, text: &str, max_width: usize) -> Vec<String> {
    let leading = raw.len() - raw.trim_start().len();
    let base_prefix = format!("{}{}", " ".repeat(leading), prefix);
    let available = max_width.saturating_sub(base_prefix.len()).max(10);

    let wrapped = textwrap::wrap(text.trim(), available);
    if wrapped.is_empty() {
        return vec![base_prefix];
    }

    let mut out = Vec::new();
    for (i, w) in wrapped.iter().enumerate() {
        if i == 0 {
            out.push(format!("{}{}", base_prefix, w));
        } else {
            out.push(format!("{:width$}{}", "", w, width = base_prefix.len()));
        }
    }
    out
}

fn wrap_list_like(raw: &str, marker: &str, text: &str, max_width: usize) -> Vec<String> {
    let leading = raw.len() - raw.trim_start().len();
    let base_prefix = format!("{}{}", " ".repeat(leading), marker);
    let available = max_width.saturating_sub(base_prefix.len()).max(10);

    let wrapped = textwrap::wrap(text.trim(), available);
    if wrapped.is_empty() {
        return vec![base_prefix];
    }

    let mut out = Vec::new();
    for (i, w) in wrapped.iter().enumerate() {
        if i == 0 {
            out.push(format!("{}{}", base_prefix, w));
        } else {
            out.push(format!("{:width$}{}", "", w, width = base_prefix.len()));
        }
    }
    out
}

/// Side panel chat - clean, consistent styling
fn render_chat_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    // Consistent background - no change on focus
    let bg_color = BG_BASE;

    // Fill background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, area);

    // Border only changes on focus (subtle indicator)
    let border_color = if app.chat_focused {
        BORDER_DEFAULT
    } else {
        BORDER_DIM
    };

    // Simple title with model name
    let model_name = app
        .chat_session
        .as_ref()
        .map(|s| {
            let model = &s.model;
            model
                .split('/')
                .next_back()
                .unwrap_or(model)
                .split(':')
                .next()
                .unwrap_or(model)
        })
        .unwrap_or("chat");

    let title = format!(" {} ", model_name);
    let title_style = Style::default().fg(TEXT_SECONDARY);

    // Clean block styling
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0))
        .title(Span::styled(title, title_style))
        .style(Style::default().bg(bg_color));

    frame.render_widget(block.clone(), area);

    if app.chat_session.is_none() {
        let tips = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Press 'c' to start",
                Style::default().fg(TEXT_SECONDARY),
            )),
        ];
        let empty = Paragraph::new(tips)
            .style(Style::default().fg(TEXT_DIM).bg(bg_color))
            .alignment(Alignment::Center);
        frame.render_widget(
            empty,
            area.inner(Margin {
                horizontal: 2,
                vertical: 2,
            }),
        );
        return;
    }

    // Tighter padding for better space usage
    let inner = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let wrap_width = inner.width.saturating_sub(1).max(20) as usize;

    // Calculate input height based on text length (with wrapping)
    let input_len = app.chat_input.len() + 3; // +3 for "> " and "_"
    let input_lines = ((input_len / wrap_width.max(1)) + 1).clamp(1, 6) as u16; // 1-6 lines
    let input_height = input_lines + 2; // +2 for border

    // Split: messages | input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(input_height)])
        .split(inner);

    // Render messages with enhanced styling
    if let Some(session) = &app.chat_session {
        let mut all_lines: Vec<Line> = Vec::new();
        let md_renderer = StyledMarkdown::new(wrap_width);

        // Define role-specific colors
        let user_prefix_color = ACCENT_PRIMARY; // Blue for user
        let user_text_color = TEXT_PRIMARY;
        let tool_color = ACCENT_WARNING; // Orange for tools
        let tool_result_color = TEXT_DIM;

        for msg in &session.messages {
            match msg.role {
                crate::chat::Role::System => continue,
                crate::chat::Role::User => {
                    // User message - compact, same line
                    let display_content = if msg.content.contains("[Browser Context]") {
                        msg.content
                            .split("\n\n")
                            .last()
                            .unwrap_or(&msg.content)
                            .to_string()
                    } else {
                        msg.content.clone()
                    };
                    // Truncate long user messages
                    let display = truncate_to_width(&display_content, wrap_width.saturating_sub(3));
                    all_lines.push(Line::from(vec![
                        Span::styled("› ", Style::default().fg(user_prefix_color)),
                        Span::styled(display, Style::default().fg(user_text_color)),
                    ]));
                }
                crate::chat::Role::Assistant => {
                    // Tool calls - compact inline
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            all_lines.push(Line::from(vec![
                                Span::styled("  ⚙ ", Style::default().fg(tool_color)),
                                Span::styled(&tc.name, Style::default().fg(tool_color)),
                            ]));
                        }
                    }
                    // AI message - render inline, no separate indicator line
                    if !msg.content.is_empty() {
                        let content_lines: Vec<String> =
                            msg.content.lines().map(String::from).collect();
                        let styled = md_renderer.render(&content_lines);
                        // Add small AI indicator to first line if present
                        if let Some(first) = styled.first() {
                            let mut first_spans = vec![Span::styled("  ", Style::default())];
                            first_spans.extend(first.spans.iter().cloned());
                            all_lines.push(Line::from(first_spans));
                            all_lines.extend(styled.into_iter().skip(1));
                        }
                    }
                }
                crate::chat::Role::Tool => {
                    // Tool result - very compact
                    let result_preview =
                        truncate_to_width(&msg.content, wrap_width.saturating_sub(6));
                    all_lines.push(Line::from(vec![
                        Span::styled("    ↳ ", Style::default().fg(tool_result_color)),
                        Span::styled(result_preview, Style::default().fg(tool_result_color)),
                    ]));
                }
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

        let msg_para = Paragraph::new(visible).style(Style::default().fg(TOKYO_TEXT).bg(bg_color));
        frame.render_widget(msg_para, chunks[0]);
    }

    // Input area - simple, consistent styling
    let input_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(BORDER_DIM))
        .style(Style::default().bg(BG_BASE));

    // Simple cursor - just show blinking when focused
    let cursor = if app.chat_focused && (app.animation_tick / 8).is_multiple_of(2) {
        "│"
    } else {
        " "
    };

    let input_text = format!(" › {}{}", app.chat_input, cursor);

    // Consistent style - no color change on focus
    let input = Paragraph::new(input_text)
        .block(input_block)
        .style(Style::default().fg(TEXT_PRIMARY).bg(BG_BASE))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[1]);
}

// Keep old function for compatibility (unused but prevents compile errors)
#[allow(dead_code)]
fn render_chat_panel(frame: &mut Frame, area: Rect, app: &App) {
    render_chat_side_panel(frame, area, app);
}

fn render_settings_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 60, area);
    let bg_color = BG_ELEVATED;

    // CRITICAL: Clear the area first to remove any content behind the overlay
    frame.render_widget(Clear, panel_area);

    // Fill with elevated background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(ACCENT_SECONDARY))
        .padding(Padding::new(2, 2, 1, 1))
        .title(" Settings ")
        .title_bottom(" Esc close ")
        .style(Style::default().bg(bg_color));

    let settings_text = vec![
        Line::from(vec![Span::styled(
            "AI Configuration",
            Style::default()
                .fg(ACCENT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )]),
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
                format!("{}...{}", &key[..7], &key[key.len() - 4..])
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
            lines.push(Line::from(vec![Span::styled(
                "  Fallback Models:",
                Style::default().fg(TOKYO_COMMENT),
            )]));
            for m in models.iter().take(5) {
                lines.push(Line::from(vec![
                    Span::raw("    - "),
                    Span::styled(m, Style::default().fg(TOKYO_TEXT)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No AI configuration found",
            Style::default().fg(TOKYO_COMMENT),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Edit ~/.config/azul/config.json to change settings",
        Style::default().fg(TOKYO_COMMENT),
    )]));

    let settings = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

    frame.render_widget(settings, panel_area);
}

fn render_rag_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);
    let bg_color = BG_ELEVATED;

    // CRITICAL: Clear the area first to remove any content behind the overlay
    frame.render_widget(Clear, panel_area);

    // Then fill with solid background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Use the unified mascot indicator
    let indicator = app.mascot.view();

    // Pulsing border for overlay panels (always focused when visible)
    let border_color = pulsing_focus_color(app.focus_pulse_phase);

    let title = format!(" {} RAG ", indicator);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(2, 2, 1, 1))
        .title(Span::styled(
            title,
            Style::default()
                .fg(ACCENT_INFO)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            if app.rag_loading {
                " searching...  Esc close "
            } else {
                " Enter search  j/k scroll  Esc close "
            },
            Style::default().fg(TEXT_DIM),
        ))
        .style(Style::default().bg(bg_color));

    let inner = panel_area.inner(Margin {
        horizontal: 3,
        vertical: 2,
    });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input area
            Constraint::Min(0),    // Results area
        ])
        .split(inner);

    // Input box - clean text, no inline spinners
    let input_border_color = if app.rag_loading {
        ACCENT_SECONDARY
    } else {
        ACCENT_PRIMARY
    };
    let input_text = if app.rag_loading {
        "Searching...".to_string()
    } else {
        format!("> {}_", app.rag_query)
    };
    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(input_border_color))
                .title(Span::styled(" Query ", Style::default().fg(TEXT_DIM)))
                .style(Style::default().bg(bg_color)),
        )
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color));

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
                "Searching knowledge base...",
                Style::default().fg(TOKYO_PURPLE),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "This may take a few seconds.",
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
                "  Neo4j knowledge graph for relationships",
                Style::default().fg(TOKYO_TEXT),
            )]),
            Line::from(vec![Span::styled(
                "  Vector embeddings for semantic search",
                Style::default().fg(TOKYO_TEXT),
            )]),
            Line::from(vec![Span::styled(
                "  LLM (Ollama/OpenRouter) for answer synthesis",
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
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(ACCENT_SECONDARY))
                .title(results_title)
                .style(Style::default().bg(bg_color)),
        )
        .style(Style::default().fg(TOKYO_TEXT).bg(bg_color))
        .wrap(Wrap { trim: false });

    frame.render_widget(results, chunks[1]);
    frame.render_widget(block, panel_area);
}

fn render_memory_panel(frame: &mut Frame, area: Rect, app: &App) {
    let panel_area = centered_rect(60, 70, area);
    let bg_color = BG_ELEVATED;

    // CRITICAL: Clear the area first to remove any content behind the overlay
    frame.render_widget(Clear, panel_area);

    // Then fill with solid background
    let bg_fill = Block::default().style(Style::default().bg(bg_color));
    frame.render_widget(bg_fill, panel_area);

    // Use the unified mascot indicator
    let indicator = app.mascot.view();

    // Pulsing border for overlay panels (always focused when visible)
    let border_color = pulsing_focus_color(app.focus_pulse_phase);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(2, 2, 1, 1))
        .title(Span::styled(
            format!(" {} Memory ", indicator),
            Style::default()
                .fg(ACCENT_INFO)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " r refresh  Esc close ",
            Style::default().fg(TEXT_DIM),
        ))
        .style(Style::default().bg(bg_color));

    let status = if app.memory_client.is_none() {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "mem-layer not available",
                Style::default().fg(ACCENT_ERROR),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Install from: ~/.local/bin/mem-layer",
                Style::default().fg(TEXT_DIM),
            )]),
        ]
    } else if app.memory_nodes.is_empty() {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Memory graph is empty",
                Style::default().fg(TEXT_DIM),
            )]),
        ]
    } else {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                format!("Nodes ({})", app.memory_nodes.len()),
                Style::default()
                    .fg(ACCENT_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        for (i, node) in app.memory_nodes.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(format!(" {}. ", i + 1), Style::default().fg(TEXT_DIM)),
                Span::styled(node, Style::default().fg(TEXT_PRIMARY)),
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
