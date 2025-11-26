use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::ai::Summarizer;
use crate::browser::{Browser, Page, RenderMode};
use crate::chat::ChatMessage;
use crate::config::Config;
use crate::mascot::{Mascot, MascotState};
use crate::search::{self, QueryTarget, SearchManager};
use crate::scrape;
use crate::storage::{Bookmark, Database, HistoryEntry};
use crate::tabs::TabManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Content,
    Sidebar,
    URLBar,
    TabBar,
    Bookmarks,
    History,
}

impl Focus {
    pub fn next(&self) -> Self {
        match self {
            Focus::Content => Focus::Sidebar,
            Focus::Sidebar => Focus::URLBar,
            Focus::URLBar => Focus::Content,
            Focus::TabBar => Focus::Content,
            Focus::Bookmarks => Focus::Content,
            Focus::History => Focus::Content,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Focus::Content => Focus::URLBar,
            Focus::Sidebar => Focus::Content,
            Focus::URLBar => Focus::Sidebar,
            Focus::TabBar => Focus::Content,
            Focus::Bookmarks => Focus::Content,
            Focus::History => Focus::Content,
        }
    }
}

pub enum AppMessage {
    PageLoaded(usize, Page), // tab_id, page
    LoadError(usize, String),
    AiSummary(String),
    AiError(String),
    ChatResponse(String),
}

/// Panel mode for overlays
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelMode {
    None,
    Bookmarks,
    History,
    Help,
    Chat,
    Settings,
    Rag,
    Memory,
}

pub struct App {
    pub config: Config,
    pub focus: Focus,
    pub should_quit: bool,
    pub animation_tick: usize,
    pub status_message: String,
    pub view_mode: ViewMode,
    pub format_text: bool,
    pub ai_summary: Option<String>,
    pub panel_mode: PanelMode,

    // Tabs
    pub tabs: TabManager,
    pub url_input: String,

    // Mascot
    pub mascot: Mascot,

    // Storage
    pub db: Option<Database>,
    pub bookmarks_list: Vec<Bookmark>,
    pub history_list: Vec<HistoryEntry>,
    pub bookmarks_selected: usize,
    pub history_selected: usize,

    // Chat
    pub chat_session: Option<crate::chat::ChatSession>,
    pub chat_input: String,
    pub chat_selected_model: usize,
    pub chat_scroll: usize,

    // Settings
    pub settings_selected: usize,
    pub settings_model_input: String,
    pub settings_editing_model: bool,

    // RAG & Memory
    pub rag_client: Option<crate::rag::RagClient>,
    pub rag_results: Vec<String>,
    pub rag_query: String,
    pub memory_client: Option<crate::memory::MemoryClient>,
    pub memory_nodes: Vec<String>,

    // Message passing
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

        // Try to open database, but don't fail if it errors
        let db = Database::open().ok();

        // Initialize chat session with available models
        let chat_session = if let Some(api_key) = config.get_api_key() {
            if !api_key.is_empty() {
                let model = config.get_ai_model().unwrap_or("qwen/qwen3-235b-a22b:free").to_string();
                let mut models = vec![model.clone()];
                if let Some(fallback) = config.get_fallback_models() {
                    models.extend(fallback);
                }
                Some(crate::chat::ChatSession::new(model, models))
            } else {
                None
            }
        } else {
            None
        };

        // Initialize RAG and memory clients
        let rag_client = crate::rag::RagClient::from_config(&config);
        let memory_client = crate::memory::MemoryClient::from_config(&config);

        let mut app = Self {
            config,
            focus: Focus::Content,
            should_quit: false,
            animation_tick: 0,
            status_message: "Ready - / search | t new tab | b bookmarks | c chat | r rag | m memory | ? help".to_string(),
            view_mode: ViewMode::Rendered,
            format_text: true,
            ai_summary: None,
            panel_mode: PanelMode::None,
            tabs: TabManager::new(),
            url_input: String::new(),
            mascot: Mascot::new(),
            db,
            bookmarks_list: Vec::new(),
            history_list: Vec::new(),
            bookmarks_selected: 0,
            history_selected: 0,
            chat_session,
            chat_input: String::new(),
            chat_selected_model: 0,
            chat_scroll: 0,
            settings_selected: 0,
            settings_model_input: String::new(),
            settings_editing_model: false,
            rag_client,
            rag_results: Vec::new(),
            rag_query: String::new(),
            memory_client,
            memory_nodes: Vec::new(),
            page_rx,
            page_tx,
            summarizer: Summarizer::from_env().map(Arc::new),
        };

        // Load bookmarks and history
        app.refresh_bookmarks();
        app.refresh_history();

        Ok(app)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONVENIENCE ACCESSORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get current page from active tab
    pub fn current_page(&self) -> Option<&Page> {
        self.tabs.active_tab().and_then(|t| t.page.as_ref())
    }

    /// Get current scroll offset from active tab
    pub fn scroll_offset(&self) -> usize {
        self.tabs.active_tab().map(|t| t.scroll_offset).unwrap_or(0)
    }

    /// Set scroll offset on active tab
    fn set_scroll_offset(&mut self, offset: usize) {
        if let Some(tab) = self.tabs.active_tab_mut() {
            tab.scroll_offset = offset;
        }
    }

    /// Get sidebar selected from active tab
    pub fn sidebar_selected(&self) -> usize {
        self.tabs.active_tab().map(|t| t.sidebar_selected).unwrap_or(0)
    }

    /// Set sidebar selected on active tab
    fn set_sidebar_selected(&mut self, idx: usize) {
        if let Some(tab) = self.tabs.active_tab_mut() {
            tab.sidebar_selected = idx;
        }
    }

    /// Check if current tab is loading
    pub fn loading(&self) -> bool {
        self.tabs.active_tab().map(|t| t.loading).unwrap_or(false)
    }

    /// Get current render mode from active tab
    pub fn render_mode(&self) -> RenderMode {
        self.tabs.active_tab().map(|t| t.render_mode).unwrap_or(RenderMode::Auto)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // UPDATE LOOP
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn update(&mut self) {
        self.animation_tick = self.animation_tick.wrapping_add(1);
        self.mascot.tick();

        // Check for loaded pages
        while let Ok(msg) = self.page_rx.try_recv() {
            match msg {
                AppMessage::PageLoaded(tab_id, page) => {
                    // Find and update the tab
                    for i in 0..self.tabs.count() {
                        if let Some(tab) = self.tabs.get_tab_mut(i) {
                            if tab.id == tab_id {
                                // Add to history
                                if let Some(db) = &self.db {
                                    let _ = db.add_history(&page.url, &page.title);
                                }
                                tab.set_page(page);
                                self.mascot.set_state(MascotState::Success);
                                self.status_message = format!("Loaded: {}", tab.url);
                                break;
                            }
                        }
                    }
                    self.ai_summary = None;
                }
                AppMessage::LoadError(tab_id, err) => {
                    for i in 0..self.tabs.count() {
                        if let Some(tab) = self.tabs.get_tab_mut(i) {
                            if tab.id == tab_id {
                                tab.set_error(err.clone());
                                self.mascot.set_state(MascotState::Error);
                                self.status_message = format!("Error: {}", err);
                                break;
                            }
                        }
                    }
                }
                AppMessage::AiSummary(summary) => {
                    self.ai_summary = Some(summary);
                    self.status_message = "AI summary ready".to_string();
                    self.mascot.set_state(MascotState::Success);
                }
                AppMessage::AiError(err) => {
                    self.status_message = format!("AI error: {}", err);
                    self.mascot.set_state(MascotState::Error);
                }
                AppMessage::ChatResponse(response) => {
                    if let Some(session) = &mut self.chat_session {
                        session.add_assistant_message(response);
                        self.mascot.set_state(MascotState::Success);
                        self.status_message = "Response received".to_string();
                    }
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

    // ═══════════════════════════════════════════════════════════════════════════
    // KEY HANDLING
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Panel-specific handling first
        match self.panel_mode {
            PanelMode::Bookmarks => return self.handle_bookmarks_keys(key),
            PanelMode::History => return self.handle_history_keys(key),
            PanelMode::Help => {
                // Any key closes help
                self.panel_mode = PanelMode::None;
                return Ok(());
            }
            PanelMode::Chat => return self.handle_chat_keys(key),
            PanelMode::Settings => return self.handle_settings_keys(key),
            PanelMode::Rag => return self.handle_rag_keys(key),
            PanelMode::Memory => return self.handle_memory_keys(key),
            PanelMode::None => {}
        }

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
                self.panel_mode = PanelMode::Help;
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
            // Tab management
            (KeyCode::Char('t'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                if self.tabs.new_tab(None).is_some() {
                    self.status_message = format!("New tab {} created", self.tabs.count());
                } else {
                    self.status_message = "Max tabs reached (9)".to_string();
                }
                return Ok(());
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                if self.tabs.count() > 1 {
                    self.tabs.close_current_tab();
                    self.status_message = "Tab closed".to_string();
                } else {
                    self.status_message = "Can't close last tab".to_string();
                }
                return Ok(());
            }
            (KeyCode::Char(n), KeyModifiers::ALT) if n.is_ascii_digit() => {
                let num = n.to_digit(10).unwrap_or(0) as usize;
                if num > 0 {
                    self.tabs.go_to_tab(num);
                    self.status_message = format!("Switched to tab {}", num);
                }
                return Ok(());
            }
            (KeyCode::Char('['), KeyModifiers::CONTROL) => {
                self.tabs.prev_tab();
                self.status_message = format!("Tab {}/{}", self.tabs.active_index() + 1, self.tabs.count());
                return Ok(());
            }
            (KeyCode::Char(']'), KeyModifiers::CONTROL) => {
                self.tabs.next_tab();
                self.status_message = format!("Tab {}/{}", self.tabs.active_index() + 1, self.tabs.count());
                return Ok(());
            }
            // Bookmarks & History
            (KeyCode::Char('b'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.panel_mode = PanelMode::Bookmarks;
                self.refresh_bookmarks();
                self.bookmarks_selected = 0;
                return Ok(());
            }
            (KeyCode::Char('H'), KeyModifiers::NONE) => {
                self.panel_mode = PanelMode::History;
                self.refresh_history();
                self.history_selected = 0;
                return Ok(());
            }
            (KeyCode::Char('B'), KeyModifiers::NONE) => {
                self.toggle_bookmark();
                return Ok(());
            }
            // Chat & Settings
            (KeyCode::Char('c'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.panel_mode = PanelMode::Chat;
                return Ok(());
            }
            (KeyCode::Char(','), KeyModifiers::NONE) => {
                self.panel_mode = PanelMode::Settings;
                return Ok(());
            }
            // RAG & Memory
            (KeyCode::Char('r'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.panel_mode = PanelMode::Rag;
                self.status_message = "RAG Panel - Enter query, Esc to close".to_string();
                return Ok(());
            }
            (KeyCode::Char('m'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.panel_mode = PanelMode::Memory;
                self.refresh_memory();
                self.status_message = "Memory Panel - Viewing mem-layer graph".to_string();
                return Ok(());
            }
            _ => {}
        }

        // Focus-specific handling
        match self.focus {
            Focus::Content => self.handle_content_keys(key),
            Focus::Sidebar => self.handle_sidebar_keys(key),
            Focus::URLBar => self.handle_urlbar_keys(key)?,
            Focus::TabBar => self.handle_tab_bar_keys(key),
            Focus::Bookmarks => {}
            Focus::History => {}
        }

        Ok(())
    }

    fn handle_content_keys(&mut self, key: KeyEvent) {
        let scroll = self.scroll_offset();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if scroll > 0 {
                    self.set_scroll_offset(scroll - 1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.set_scroll_offset(scroll + 1);
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
                self.set_scroll_offset(0);
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
                self.set_scroll_offset(0);
            }
            KeyCode::Char('G') => {
                if let Some(page) = self.current_page() {
                    self.set_scroll_offset(page.content_lines.len().saturating_sub(1));
                }
            }
            KeyCode::Char('J') => {
                // Toggle JavaScript rendering mode
                if let Some(tab) = self.tabs.active_tab_mut() {
                    tab.render_mode = match tab.render_mode {
                        RenderMode::Static => RenderMode::Auto,
                        RenderMode::Auto => RenderMode::JavaScript,
                        RenderMode::JavaScript => RenderMode::Static,
                    };
                    self.status_message = match tab.render_mode {
                        RenderMode::Static => "JS Mode: OFF (fast HTTP only)".to_string(),
                        RenderMode::Auto => "JS Mode: AUTO (detect & render if needed)".to_string(),
                        RenderMode::JavaScript => "JS Mode: ON (always use headless Chrome)".to_string(),
                    };
                }
            }
            KeyCode::Char('r') => {
                // Reload current page
                if let Some(tab) = self.tabs.active_tab() {
                    if !tab.url.is_empty() {
                        self.url_input = tab.url.clone();
                        self.navigate();
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('p') => {
                // Go back in history
                if let Some(tab) = self.tabs.active_tab_mut() {
                    if let Some(url) = tab.go_back() {
                        self.url_input = url;
                        self.navigate_internal(false);
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('n') => {
                // Go forward in history
                if let Some(tab) = self.tabs.active_tab_mut() {
                    if let Some(url) = tab.go_forward() {
                        self.url_input = url;
                        self.navigate_internal(false);
                    }
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
        let selected = self.sidebar_selected();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if selected > 0 {
                    self.set_sidebar_selected(selected - 1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(page) = self.current_page() {
                    if selected < page.links.len().saturating_sub(1) {
                        self.set_sidebar_selected(selected + 1);
                    }
                }
            }
            KeyCode::Char('g') => {
                self.set_sidebar_selected(0);
            }
            KeyCode::Char('G') => {
                if let Some(page) = self.current_page() {
                    if !page.links.is_empty() {
                        self.set_sidebar_selected(page.links.len().saturating_sub(1));
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(page) = self.current_page() {
                    if let Some(link) = page.links.get(selected) {
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

    fn handle_tab_bar_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.tabs.prev_tab();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.tabs.next_tab();
            }
            KeyCode::Enter => {
                self.focus = Focus::Content;
            }
            KeyCode::Char('x') => {
                if self.tabs.count() > 1 {
                    self.tabs.close_current_tab();
                }
            }
            _ => {}
        }
    }

    fn handle_bookmarks_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('b') | KeyCode::Char('q') => {
                self.panel_mode = PanelMode::None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.bookmarks_selected > 0 {
                    self.bookmarks_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.bookmarks_selected < self.bookmarks_list.len().saturating_sub(1) {
                    self.bookmarks_selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(bookmark) = self.bookmarks_list.get(self.bookmarks_selected) {
                    self.url_input = bookmark.url.clone();
                    self.panel_mode = PanelMode::None;
                    self.navigate();
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(bookmark) = self.bookmarks_list.get(self.bookmarks_selected) {
                    if let Some(db) = &self.db {
                        let _ = db.remove_bookmark(&bookmark.url);
                        self.refresh_bookmarks();
                        if self.bookmarks_selected > 0 {
                            self.bookmarks_selected -= 1;
                        }
                        self.status_message = "Bookmark removed".to_string();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_history_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('H') | KeyCode::Char('q') => {
                self.panel_mode = PanelMode::None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.history_selected > 0 {
                    self.history_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.history_selected < self.history_list.len().saturating_sub(1) {
                    self.history_selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(entry) = self.history_list.get(self.history_selected) {
                    self.url_input = entry.url.clone();
                    self.panel_mode = PanelMode::None;
                    self.navigate();
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(db) = &self.db {
                    let _ = db.clear_history();
                    self.refresh_history();
                    self.status_message = "History cleared".to_string();
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NAVIGATION
    // ═══════════════════════════════════════════════════════════════════════════

    fn navigate(&mut self) {
        self.navigate_internal(true);
    }

    fn navigate_internal(&mut self, add_to_history: bool) {
        if self.url_input.is_empty() {
            return;
        }

        let input = self.url_input.trim().to_string();
        self.ai_summary = None;
        self.mascot.set_state(MascotState::Loading);

        // Get current tab info
        let tab_id = self.tabs.active_tab().map(|t| t.id).unwrap_or(0);
        let render_mode = self.render_mode();

        // Update tab state
        if add_to_history {
            if let Some(tab) = self.tabs.active_tab_mut() {
                tab.navigate(&input);
            }
        } else {
            if let Some(tab) = self.tabs.active_tab_mut() {
                tab.loading = true;
                tab.error = None;
            }
        }

        let tx = self.page_tx.clone();

        match search::classify_query(&input) {
            QueryTarget::Url(url) => {
                let mode_str = match render_mode {
                    RenderMode::Static => "",
                    RenderMode::Auto => " [auto-JS]",
                    RenderMode::JavaScript => " [JS]",
                };
                self.status_message = format!("Loading{}: {}", mode_str, url);
                self.mascot.set_state(MascotState::Loading);

                std::thread::spawn(move || {
                    let browser = match Browser::new() {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = tx.send(AppMessage::LoadError(tab_id, format!("Browser init error: {}", e)));
                            return;
                        }
                    };

                    match browser.fetch_with_mode(&url, render_mode) {
                        Ok(page) => {
                            let _ = tx.send(AppMessage::PageLoaded(tab_id, page));
                        }
                        Err(err) => {
                            let _ = tx.send(AppMessage::LoadError(tab_id, format!("Fetch error: {}", err)));
                        }
                    }
                });
            }
            QueryTarget::Search { engine, query } => {
                self.status_message = format!("Searching {}: {}", engine.name(), query);
                self.mascot.set_state(MascotState::Searching);

                std::thread::spawn(move || {
                    let search_result =
                        SearchManager::new().and_then(|manager| manager.search_with(engine, &query));

                    match search_result {
                        Ok(response) => {
                            let page = search::results_to_page(response);
                            let _ = tx.send(AppMessage::PageLoaded(tab_id, page));
                        }
                        Err(err) => {
                            let _ = tx.send(AppMessage::LoadError(tab_id, format!("Search error: {}", err)));
                        }
                    }
                });
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BOOKMARKS & HISTORY
    // ═══════════════════════════════════════════════════════════════════════════

    fn toggle_bookmark(&mut self) {
        let Some(db) = &self.db else {
            self.status_message = "Database not available".to_string();
            return;
        };

        let Some(page) = self.current_page() else {
            self.status_message = "No page to bookmark".to_string();
            return;
        };

        let url = page.url.clone();
        let title = page.title.clone();

        match db.is_bookmarked(&url) {
            Ok(true) => {
                if db.remove_bookmark(&url).is_ok() {
                    self.status_message = "Bookmark removed".to_string();
                }
            }
            Ok(false) => {
                if db.add_bookmark(&url, &title, &[]).is_ok() {
                    self.status_message = "Bookmarked!".to_string();
                }
            }
            Err(_) => {
                self.status_message = "Bookmark error".to_string();
            }
        }
        self.refresh_bookmarks();
    }

    fn refresh_bookmarks(&mut self) {
        if let Some(db) = &self.db {
            self.bookmarks_list = db.list_bookmarks(None, 100).unwrap_or_default();
        }
    }

    fn refresh_history(&mut self) {
        if let Some(db) = &self.db {
            self.history_list = db.list_history(None, 100).unwrap_or_default();
        }
    }

    /// Check if current URL is bookmarked
    pub fn is_current_bookmarked(&self) -> bool {
        let Some(db) = &self.db else { return false };
        let Some(page) = self.current_page() else { return false };
        db.is_bookmarked(&page.url).unwrap_or(false)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AI & SCRAPING
    // ═══════════════════════════════════════════════════════════════════════════

    fn request_ai_summary(&mut self) -> Result<()> {
        let Some(summarizer) = &self.summarizer else {
            self.status_message = "AI not configured (set OPENROUTER_API_KEY)".to_string();
            return Ok(());
        };

        let Some(page) = self.current_page() else {
            self.status_message = "No page to summarize".to_string();
            return Ok(());
        };

        let content_text = page.content_lines.join("\n");
        let title = page.title.clone();
        let url = page.url.clone();
        let tx = self.page_tx.clone();
        let summarizer = Arc::clone(summarizer);

        self.mascot.set_state(MascotState::Loading);
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
        let Some(page) = self.current_page() else {
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

    fn handle_chat_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.panel_mode = PanelMode::None;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Cycle to previous model
                if let Some(session) = &mut self.chat_session {
                    if !session.available_models.is_empty() {
                        self.chat_selected_model = if self.chat_selected_model == 0 {
                            session.available_models.len() - 1
                        } else {
                            self.chat_selected_model - 1
                        };
                        session.set_model(session.available_models[self.chat_selected_model].clone());
                        self.status_message = format!("Model: {}", session.model);
                    }
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Cycle to next model
                if let Some(session) = &mut self.chat_session {
                    if !session.available_models.is_empty() {
                        self.chat_selected_model = (self.chat_selected_model + 1) % session.available_models.len();
                        session.set_model(session.available_models[self.chat_selected_model].clone());
                        self.status_message = format!("Model: {}", session.model);
                    }
                }
            }
            KeyCode::Char(c) if c != 'c' => {
                self.chat_input.push(c);
            }
            KeyCode::Backspace => {
                self.chat_input.pop();
            }
            KeyCode::Enter => {
                if !self.chat_input.is_empty() {
                    let message = self.chat_input.clone();
                    self.chat_input.clear();

                    if let Some(session) = &mut self.chat_session {
                        session.add_user_message(message.clone());

                        // Send to AI in background
                        let api_key = self.config.get_api_key().unwrap_or("").to_string();
                        let model = session.model.clone();
                        let messages = session.messages.clone();
                        let tx = self.page_tx.clone();

                        self.mascot.set_state(crate::mascot::MascotState::Loading);
                        self.status_message = "Thinking...".to_string();

                        std::thread::spawn(move || {
                            match send_chat_message(&api_key, &model, &messages) {
                                Ok(response) => {
                                    let _ = tx.send(AppMessage::ChatResponse(response));
                                }
                                Err(e) => {
                                    let _ = tx.send(AppMessage::AiError(format!("Chat error: {}", e)));
                                }
                            }
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_settings_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.panel_mode = PanelMode::None;
                self.settings_editing_model = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_rag_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.panel_mode = PanelMode::None;
            }
            KeyCode::Char(c) if c != 'r' && c != 'q' => {
                self.rag_query.push(c);
            }
            KeyCode::Backspace => {
                self.rag_query.pop();
            }
            KeyCode::Enter => {
                if !self.rag_query.is_empty() && self.rag_client.is_some() {
                    let query = self.rag_query.clone();
                    self.status_message = format!("Querying RAG: {}", query);

                    if let Some(rag) = &self.rag_client {
                        match rag.query(&query, Some(5)) {
                            Ok(response) => {
                                self.rag_results = response.sources
                                    .iter()
                                    .map(|s| format!("[{:.2}] {}", s.score, s.content))
                                    .collect();
                                self.status_message = format!("Found {} results", self.rag_results.len());
                            }
                            Err(e) => {
                                self.status_message = format!("RAG error: {}", e);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_memory_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.panel_mode = PanelMode::None;
            }
            KeyCode::Char('r') => {
                // Refresh memory
                self.refresh_memory();
            }
            _ => {}
        }
        Ok(())
    }

    fn refresh_memory(&mut self) {
        if let Some(mem) = &self.memory_client {
            match mem.list_nodes(None, Some(20)) {
                Ok(nodes) => {
                    self.memory_nodes = nodes;
                    self.status_message = format!("Memory: {} nodes", self.memory_nodes.len());
                }
                Err(e) => {
                    self.status_message = format!("Memory error: {}", e);
                    self.memory_nodes.clear();
                }
            }
        } else {
            self.status_message = "mem-layer not available".to_string();
            self.memory_nodes.clear();
        }
    }

    fn update_status(&mut self) {
        self.status_message = match self.focus {
            Focus::Content => "Content - j/k scroll | / search | 2 sidebar | ? help".to_string(),
            Focus::Sidebar => "Sidebar - j/k navigate | Enter open | 1 content".to_string(),
            Focus::URLBar => "URL Bar - Enter go | Esc cancel".to_string(),
            Focus::TabBar => "Tab Bar - h/l switch | x close | Enter select".to_string(),
            Focus::Bookmarks => "Bookmarks - j/k nav | Enter open | d delete | Esc close".to_string(),
            Focus::History => "History - j/k nav | Enter open | Ctrl+C clear | Esc close".to_string(),
        };
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Chat API Integration
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct OpenAIChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChatChoice>,
}

#[derive(Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIChatMessage,
}

fn send_chat_message(api_key: &str, model: &str, messages: &[ChatMessage]) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    // Convert our messages to OpenAI format
    let api_messages: Vec<OpenAIChatMessage> = messages
        .iter()
        .map(|m| OpenAIChatMessage {
            role: match m.role {
                crate::chat::Role::System => "system".to_string(),
                crate::chat::Role::User => "user".to_string(),
                crate::chat::Role::Assistant => "assistant".to_string(),
                crate::chat::Role::Tool => "tool".to_string(),
            },
            content: m.content.clone(),
        })
        .collect();

    let request = OpenAIChatRequest {
        model: model.to_string(),
        messages: api_messages,
        max_tokens: 1024,
        temperature: 0.7,
    };

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/ser/azul-browse")
        .header("X-Title", "Azul Terminal Browser")
        .json(&request)
        .send()?;

    let result: OpenAIChatResponse = response.json()?;

    if let Some(choice) = result.choices.first() {
        Ok(choice.message.content.clone())
    } else {
        anyhow::bail!("No response from API")
    }
}
