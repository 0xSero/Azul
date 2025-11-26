use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::ai::Summarizer;
use crate::browser::{Browser, Page, RenderMode};
use crate::config::Config;
use crate::search::{self, QueryTarget, SearchManager};
use crate::scrape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Content,
    Sidebar,
    URLBar,
}

impl Focus {
    pub fn next(&self) -> Self {
        match self {
            Focus::Content => Focus::Sidebar,
            Focus::Sidebar => Focus::URLBar,
            Focus::URLBar => Focus::Content,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Focus::Content => Focus::URLBar,
            Focus::Sidebar => Focus::Content,
            Focus::URLBar => Focus::Sidebar,
        }
    }
}

pub enum AppMessage {
    PageLoaded(Page),
    LoadError(String),
    AiSummary(String),
    AiError(String),
}

pub struct App {
    pub config: Config,
    pub current_page: Option<Page>,
    pub url_input: String,
    pub focus: Focus,
    pub should_quit: bool,
    pub animation_tick: usize,
    pub loading: bool,
    pub status_message: String,
    pub scroll_offset: usize,
    pub sidebar_selected: usize,
    pub sidebar_offset: usize,
    pub show_help: bool,
    pub view_mode: ViewMode,
    pub format_text: bool,
    pub ai_summary: Option<String>,
    pub render_mode: RenderMode,
    page_rx: Receiver<AppMessage>,
    page_tx: Sender<AppMessage>,
    summarizer: Option<Arc<Summarizer>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Rendered,
    Raw,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let (page_tx, page_rx) = channel();

        Ok(Self {
            config,
            current_page: None,
            url_input: String::new(),
            focus: Focus::Content,
            should_quit: false,
            animation_tick: 0,
            loading: false,
            status_message: "Ready - Press / to enter URL or search | J to toggle JS mode".to_string(),
            scroll_offset: 0,
            sidebar_selected: 0,
            sidebar_offset: 0,
            show_help: false,
            view_mode: ViewMode::Rendered,
            format_text: true,
            ai_summary: None,
            render_mode: RenderMode::Auto, // Auto-detect JS needs
            page_rx,
            page_tx,
            summarizer: Summarizer::from_env().map(Arc::new),
        })
    }

    pub fn update(&mut self) {
        self.animation_tick = self.animation_tick.wrapping_add(1);

        // Check for loaded pages
        while let Ok(msg) = self.page_rx.try_recv() {
            match msg {
                AppMessage::PageLoaded(page) => {
                    self.loading = false;
                    self.status_message = format!("Loaded: {}", page.url);
                    self.current_page = Some(page);
                    self.scroll_offset = 0;
                    self.sidebar_selected = 0;
                    self.ai_summary = None;
                }
                AppMessage::LoadError(err) => {
                    self.loading = false;
                    self.status_message = format!("Error: {}", err);
                }
                AppMessage::AiSummary(summary) => {
                    self.loading = false;
                    self.ai_summary = Some(summary);
                    self.status_message = "AI summary ready".to_string();
                }
                AppMessage::AiError(err) => {
                    self.loading = false;
                    self.status_message = format!("AI error: {}", err);
                }
            }
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
        self.update_status();
    }

    pub fn focus_previous(&mut self) {
        self.focus = self.focus.previous();
        self.update_status();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global shortcuts
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('Q'), KeyModifiers::NONE) => {
                if self.focus != Focus::URLBar {
                    self.quit();
                    return Ok(());
                }
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.quit();
                return Ok(());
            }
            (KeyCode::Char('?'), KeyModifiers::NONE) => {
                self.show_help = !self.show_help;
                return Ok(());
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.focus_next();
                return Ok(());
            }
            (KeyCode::BackTab, _) => {
                self.focus_previous();
                return Ok(());
            }
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                if self.focus != Focus::URLBar {
                    self.focus = Focus::URLBar;
                    self.url_input.clear();
                    self.update_status();
                    return Ok(());
                }
            }
            _ => {}
        }

        // Focus-specific handling
        match self.focus {
            Focus::Content => self.handle_content_keys(key),
            Focus::Sidebar => self.handle_sidebar_keys(key),
            Focus::URLBar => self.handle_urlbar_keys(key)?,
        }

        Ok(())
    }

    fn handle_content_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset += 1;
            }
            KeyCode::Char('f') => {
                self.format_text = !self.format_text;
                self.status_message = if self.format_text {
                    "Text formatting ON".to_string()
                } else {
                    "Text formatting OFF".to_string()
                };
            }
            KeyCode::Char('v') => {
                self.view_mode = match self.view_mode {
                    ViewMode::Rendered => ViewMode::Raw,
                    ViewMode::Raw => ViewMode::Rendered,
                };
                self.status_message = match self.view_mode {
                    ViewMode::Rendered => "Viewing formatted content".to_string(),
                    ViewMode::Raw => "Viewing raw page source".to_string(),
                };
                self.scroll_offset = 0;
            }
            KeyCode::Char('s') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Err(err) = self.scrape_current_page() {
                        self.status_message = format!("Scrape error: {}", err);
                    }
                } else if let Err(err) = self.request_ai_summary() {
                    self.status_message = format!("AI error: {}", err);
                }
            }
            KeyCode::Char('g') => {
                self.scroll_offset = 0;
            }
            KeyCode::Char('G') => {
                if let Some(page) = &self.current_page {
                    self.scroll_offset = page.content_lines.len().saturating_sub(1);
                }
            }
            KeyCode::Char('J') => {
                // Toggle JavaScript rendering mode
                self.render_mode = match self.render_mode {
                    RenderMode::Static => RenderMode::Auto,
                    RenderMode::Auto => RenderMode::JavaScript,
                    RenderMode::JavaScript => RenderMode::Static,
                };
                self.status_message = match self.render_mode {
                    RenderMode::Static => "JS Mode: OFF (fast HTTP only)".to_string(),
                    RenderMode::Auto => "JS Mode: AUTO (detect & render if needed)".to_string(),
                    RenderMode::JavaScript => "JS Mode: ON (always use headless Chrome)".to_string(),
                };
            }
            KeyCode::Char('r') => {
                // Reload current page
                if let Some(page) = &self.current_page {
                    self.url_input = page.url.clone();
                    self.navigate();
                }
            }
            KeyCode::Char('2') | KeyCode::F(2) => {
                self.focus = Focus::Sidebar;
                self.update_status();
            }
            _ => {}
        }
    }

    fn handle_sidebar_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sidebar_selected > 0 {
                    self.sidebar_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(page) = &self.current_page {
                    if self.sidebar_selected < page.links.len().saturating_sub(1) {
                        self.sidebar_selected += 1;
                    }
                }
            }
            KeyCode::Char('g') => {
                self.sidebar_selected = 0;
            }
            KeyCode::Char('G') => {
                if let Some(page) = &self.current_page {
                    if !page.links.is_empty() {
                        self.sidebar_selected = page.links.len().saturating_sub(1);
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(page) = &self.current_page {
                    if let Some(link) = page.links.get(self.sidebar_selected) {
                        self.url_input = link.url.clone();
                        self.navigate();
                    }
                }
            }
            KeyCode::Char('1') | KeyCode::F(1) => {
                self.focus = Focus::Content;
                self.update_status();
            }
            _ => {}
        }
    }

    fn handle_urlbar_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.url_input.push(c);
            }
            KeyCode::Backspace => {
                self.url_input.pop();
            }
            KeyCode::Enter => {
                self.navigate();
                self.focus = Focus::Content;
                self.update_status();
            }
            KeyCode::Esc => {
                self.focus = Focus::Content;
                self.update_status();
            }
            _ => {}
        }
        Ok(())
    }

    fn navigate(&mut self) {
        if self.url_input.is_empty() {
            return;
        }

        self.loading = true;
        let input = self.url_input.trim().to_string();
        self.scroll_offset = 0;
        self.sidebar_selected = 0;
        self.ai_summary = None;

        let tx = self.page_tx.clone();

        match search::classify_query(&input) {
            QueryTarget::Url(url) => {
                let mode_str = match self.render_mode {
                    RenderMode::Static => "",
                    RenderMode::Auto => " [auto-JS]",
                    RenderMode::JavaScript => " [JS]",
                };
                self.status_message = format!("Loading{}: {}", mode_str, url);

                let render_mode = self.render_mode;

                // Spawn background thread to fetch page
                std::thread::spawn(move || {
                    let browser = match Browser::new() {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = tx.send(AppMessage::LoadError(format!("Browser init error: {}", e)));
                            return;
                        }
                    };

                    match browser.fetch_with_mode(&url, render_mode) {
                        Ok(page) => {
                            let _ = tx.send(AppMessage::PageLoaded(page));
                        }
                        Err(err) => {
                            let _ = tx.send(AppMessage::LoadError(format!("Fetch error: {}", err)));
                        }
                    }
                });
            }
            QueryTarget::Search { engine, query } => {
                self.status_message = format!("Searching {}: {}", engine.name(), query);

                std::thread::spawn(move || {
                    let search_result =
                        SearchManager::new().and_then(|manager| manager.search_with(engine, &query));

                    match search_result {
                        Ok(response) => {
                            let page = search::results_to_page(response);
                            let _ = tx.send(AppMessage::PageLoaded(page));
                        }
                        Err(err) => {
                            let _ = tx.send(AppMessage::LoadError(format!("Search error: {}", err)));
                        }
                    }
                });
            }
        }

        self.sidebar_offset = 0;
    }

    fn request_ai_summary(&mut self) -> Result<()> {
        let Some(summarizer) = &self.summarizer else {
            self.status_message = "AI not configured (set OPENROUTER_API_KEY)".to_string();
            return Ok(());
        };

        let Some(page) = &self.current_page else {
            self.status_message = "No page to summarize".to_string();
            return Ok(());
        };

        let content_text = page.content_lines.join("\n");
        let title = page.title.clone();
        let url = page.url.clone();
        let tx = self.page_tx.clone();
        let summarizer = Arc::clone(summarizer);

        self.loading = true;
        self.status_message = "Summarizing page with AI...".to_string();

        std::thread::spawn(move || {
            match summarizer.summarize_page(&title, &url, &content_text) {
                Ok(summary) => {
                    let _ = tx.send(AppMessage::AiSummary(summary));
                }
                Err(err) => {
                    let _ = tx.send(AppMessage::AiError(format!("{}", err)));
                }
            }
        });

        Ok(())
    }

    fn scrape_current_page(&mut self) -> Result<()> {
        let Some(page) = &self.current_page else {
            self.status_message = "No page to scrape".to_string();
            return Ok(());
        };

        match scrape::save_scrape(page) {
            Ok(path) => {
                self.status_message = format!("Scraped to {}", path.display());
            }
            Err(err) => {
                self.status_message = format!("Scrape error: {}", err);
            }
        }

        Ok(())
    }

    fn update_status(&mut self) {
        self.status_message = match self.focus {
            Focus::Content => "Content - j/k scroll | / search | 2 sidebar | ? help".to_string(),
            Focus::Sidebar => "Sidebar - j/k navigate | ⏎ open | 1 content".to_string(),
            Focus::URLBar => "URL Bar - ⏎ go | Esc cancel".to_string(),
        };
    }
}
