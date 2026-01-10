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
    ToolAction(ToolAction),
    RagResponse(Vec<String>),
    RagError(String),
}

/// Actions that can be triggered by AI tool calls
#[derive(Debug, Clone)]
pub enum ToolAction {
    Navigate(String),
    FollowLink(usize),
    Scroll { direction: String, amount: usize },
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
    pub chat_focused: bool,

    // Settings
    pub settings_selected: usize,
    pub settings_model_input: String,
    pub settings_editing_model: bool,

    // RAG & Memory
    pub rag_client: Option<crate::rag::RagClient>,
    pub rag_results: Vec<String>,
    pub rag_query: String,
    pub rag_scroll: usize,
    pub rag_loading: bool,
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
                let provider = config.ai.as_ref().and_then(|ai| ai.provider.as_deref()).unwrap_or("openrouter");
                let default_model = if provider == "minimax" {
                    "MiniMax-M2.1"
                } else {
                    "qwen/qwen3-235b-a22b:free"
                };

                let model = config.get_ai_model().unwrap_or(default_model).to_string();
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
            panel_mode: PanelMode::None,  // Chat is always visible, not a panel mode
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
            chat_focused: false,  // Start in content mode, press 'c' or '3' for chat
            settings_selected: 0,
            settings_model_input: String::new(),
            settings_editing_model: false,
            rag_client,
            rag_results: Vec::new(),
            rag_query: String::new(),
            rag_scroll: 0,
            rag_loading: false,
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
                    // Also show error in chat so it's visible
                    if let Some(session) = &mut self.chat_session {
                        session.add_assistant_message(format!("❌ Error: {}", err));
                        self.chat_scroll = 0;
                    }
                }
                AppMessage::ChatResponse(response) => {
                    if let Some(session) = &mut self.chat_session {
                        session.add_assistant_message(response);
                        self.chat_scroll = 0; // Auto-scroll to bottom (newest messages)
                        self.mascot.set_state(MascotState::Success);
                        self.status_message = "Response received".to_string();
                    }
                }
                AppMessage::ToolAction(action) => {
                    match action {
                        ToolAction::Navigate(url) => {
                            self.url_input = url;
                            self.navigate();
                            self.status_message = "AI navigated to URL".to_string();
                        }
                        ToolAction::FollowLink(idx) => {
                            if let Some(page) = self.current_page() {
                                if let Some(link) = page.links.get(idx) {
                                    self.url_input = link.url.clone();
                                    self.navigate();
                                    self.status_message = format!("AI followed link {}", idx + 1);
                                }
                            }
                        }
                        ToolAction::Scroll { direction, amount } => {
                            if let Some(tab) = self.tabs.active_tab_mut() {
                                match direction.as_str() {
                                    "up" => tab.scroll_offset = tab.scroll_offset.saturating_sub(amount),
                                    "down" => tab.scroll_offset += amount,
                                    "top" => tab.scroll_offset = 0,
                                    "bottom" => tab.scroll_offset = usize::MAX / 2,
                                    _ => {}
                                }
                                self.status_message = format!("AI scrolled {}", direction);
                            }
                        }
                    }
                }
                AppMessage::RagResponse(results) => {
                    self.rag_loading = false;
                    self.rag_results = results;
                    self.rag_scroll = 0;
                    self.mascot.set_state(MascotState::Success);
                    self.status_message = format!("RAG: {} results", self.rag_results.len());
                }
                AppMessage::RagError(err) => {
                    self.rag_loading = false;
                    self.rag_results = vec![format!("Error: {}", err)];
                    self.mascot.set_state(MascotState::Error);
                    self.status_message = format!("RAG error: {}", err);
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
        // Overlay panel handling first (these take full focus)
        match self.panel_mode {
            PanelMode::Bookmarks => return self.handle_bookmarks_keys(key),
            PanelMode::History => return self.handle_history_keys(key),
            PanelMode::Help => {
                // Any key closes help
                self.panel_mode = PanelMode::None;
                return Ok(());
            }
            PanelMode::Settings => return self.handle_settings_keys(key),
            PanelMode::Rag => return self.handle_rag_keys(key),
            PanelMode::Memory => return self.handle_memory_keys(key),
            PanelMode::Chat | PanelMode::None => {}  // Chat is always visible, handled below
        }

        // Chat input handling when chat is focused
        if self.chat_focused {
            match (key.code, key.modifiers) {
                // Esc unfocuses chat
                (KeyCode::Esc, _) => {
                    self.chat_focused = false;
                    self.status_message = "Content mode - 'c' for chat, '/' for URL".to_string();
                    return Ok(());
                }
                // These pass through to global shortcuts
                (KeyCode::Tab, _) | (KeyCode::BackTab, _) => {}
                // Handle chat input
                _ => return self.handle_chat_keys(key),
            }
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
            // Chat focus (same as '3')
            (KeyCode::Char('c'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.chat_focused = true;
                self.status_message = "Chat - type message, Esc to exit".to_string();
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
            // Number keys for panel focus: 1=Links, 2=Content, 3=Chat
            (KeyCode::Char('1'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.focus = Focus::Sidebar;
                self.chat_focused = false;
                self.status_message = "Links - j/k navigate, Enter follow".to_string();
                return Ok(());
            }
            (KeyCode::Char('2'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.focus = Focus::Content;
                self.chat_focused = false;
                self.status_message = "Content - j/k scroll, / search".to_string();
                return Ok(());
            }
            (KeyCode::Char('3'), KeyModifiers::NONE) if self.focus != Focus::URLBar => {
                self.chat_focused = true;
                self.status_message = "Chat - type message, Esc to exit".to_string();
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
            KeyCode::Char('o') => {
                // Open current URL in system browser/viewer
                if let Some(tab) = self.tabs.active_tab() {
                    if !tab.url.is_empty() {
                        let url = tab.url.clone();
                        
                        // Cross-platform open logic
                        let success = if cfg!(target_os = "windows") {
                            // Windows: Try cmd /c start first
                            std::process::Command::new("cmd")
                                .args(&["/c", "start", "", &url])
                                .spawn()
                                .is_ok() || 
                            // Fallback to PowerShell
                            std::process::Command::new("powershell")
                                .args(&["-Command", &format!("Start-Process '{}'", url)])
                                .spawn()
                                .is_ok()
                        } else if cfg!(target_os = "macos") {
                            std::process::Command::new("open").arg(&url).spawn().is_ok()
                        } else {
                            // Linux/Other
                            std::process::Command::new("xdg-open").arg(&url).spawn().is_ok()
                        };

                        if success {
                            self.status_message = format!("Opened in system viewer: {}", url);
                        } else {
                            self.status_message = "Failed to open in system viewer".to_string();
                        }
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
            QueryTarget::MultiSearch { query } => {
                self.status_message = format!("Multi-engine search: {}", query);
                self.mascot.set_state(MascotState::Searching);

                std::thread::spawn(move || {
                    match SearchManager::new() {
                        Ok(manager) => {
                            let response = manager.search_aggregated(&query);
                            let page = response.to_page();
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
            KeyCode::Char('[') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.chat_focused = false;
                self.status_message = "Content mode".to_string();
            }
            KeyCode::PageUp => {
                self.chat_scroll += 5;
            }
            KeyCode::PageDown => {
                self.chat_scroll = self.chat_scroll.saturating_sub(5);
            }
            KeyCode::Up => {
                self.chat_scroll += 1;
            }
            KeyCode::Down => {
                self.chat_scroll = self.chat_scroll.saturating_sub(1);
            }
            KeyCode::Char(c) => {
                self.chat_input.push(c);
            }
            KeyCode::Backspace => {
                self.chat_input.pop();
            }
            KeyCode::Enter => {
                if !self.chat_input.is_empty() {
                    let message = self.chat_input.clone();
                    self.chat_input.clear();
                    self.chat_scroll = 0;

                    let context = self.get_browser_context();

                    if let Some(session) = &mut self.chat_session {
                        session.add_user_message(format!("{}\n\n{}", context, message));

                        let api_key = self.config.get_api_key().unwrap_or("").to_string();
                        
                        let provider = self.config.ai.as_ref().and_then(|ai| ai.provider.as_deref()).unwrap_or("openrouter");
                        let default_url = if provider == "minimax" {
                            "https://api.minimax.io/v1"
                        } else {
                            "https://openrouter.ai/api/v1"
                        };

                        let base_url = self.config.get_ai_base_url()
                            .unwrap_or(default_url)
                            .to_string();
                            
                        let models = session.available_models.clone();
                        let messages = session.messages.clone();
                        let tx = self.page_tx.clone();

                        self.mascot.set_state(crate::mascot::MascotState::Loading);
                        self.status_message = "Thinking...".to_string();

                        std::thread::spawn(move || {
                            match send_chat_message_with_fallback(&api_key, &base_url, &models, &messages, &tx) {
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
            KeyCode::Esc => {
                self.panel_mode = PanelMode::None;
            }
            // Scrolling results with j/k or arrows (scroll = 0 is top)
            KeyCode::Up | KeyCode::Char('k') if !self.rag_results.is_empty() => {
                self.rag_scroll = self.rag_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') if !self.rag_results.is_empty() => {
                self.rag_scroll = self.rag_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.rag_scroll = self.rag_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.rag_scroll = self.rag_scroll.saturating_add(10);
            }
            KeyCode::Char('g') if !self.rag_results.is_empty() => {
                self.rag_scroll = 0; // Top
            }
            KeyCode::Char('G') if !self.rag_results.is_empty() => {
                self.rag_scroll = usize::MAX; // Bottom (will be clamped in render)
            }
            KeyCode::Char(c) => {
                self.rag_query.push(c);
            }
            KeyCode::Backspace => {
                self.rag_query.pop();
            }
            KeyCode::Enter => {
                if !self.rag_query.is_empty() && self.rag_client.is_some() && !self.rag_loading {
                    let query = self.rag_query.clone();
                    self.status_message = format!("Querying RAG: {}...", query);
                    self.rag_loading = true;
                    self.rag_results.clear();
                    self.mascot.set_state(MascotState::Loading);

                    // Clone what we need for the thread
                    let tx = self.page_tx.clone();
                    let rag_url = self.config.rag_base_url.clone().unwrap_or_else(|| "http://localhost:8100".to_string());

                    std::thread::spawn(move || {
                        // Create a new client in the thread
                        match crate::rag::RagClient::new(rag_url) {
                            Ok(rag) => {
                                match rag.query(&query, Some(5)) {
                                    Ok(response) => {
                                        let mut results = vec![
                                            format!("## Answer\n\n{}", response.answer),
                                            String::new(),
                                            "---".to_string(),
                                            String::new(),
                                            format!("## Sources ({})", response.sources.len()),
                                        ];
                                        for (i, s) in response.sources.iter().enumerate() {
                                            let content = s.text.as_ref().unwrap_or(&s.content);
                                            let score = s.similarity.unwrap_or(s.score);
                                            results.push(format!("\n**[{}]** score: {:.2}", i + 1, score));
                                            results.push(content.clone());
                                        }
                                        let _ = tx.send(AppMessage::RagResponse(results));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(AppMessage::RagError(e.to_string()));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::RagError(format!("Failed to connect: {}", e)));
                            }
                        }
                    });
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

    fn get_browser_context(&self) -> String {
        let mut context = String::from("[Browser Context]");

        if let Some(page) = self.current_page() {
            context.push_str(&format!(
                "\nCurrent page: {} ({})\nLinks: {}\nContent preview:\n{}",
                page.title,
                page.url,
                page.links.len(),
                page.content_lines.iter()
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        } else {
            context.push_str("\nNo page currently loaded.");
        }

        context
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
// Chat API Integration with Tool Calling
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIChatMessage>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAIChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize, Clone)]
struct ToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDefinition,
}

#[derive(Serialize, Clone)]
struct FunctionDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Option<Vec<OpenAIChatChoice>>,
    error: Option<OpenAIError>,
}

#[derive(Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIChatMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
}

/// Get browser tool definitions
fn get_browser_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "brave_search".to_string(),
                description: "Search the web using Brave Search API. Use this to find current information, news, or answer questions that require up-to-date web data.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        },
                        "count": {
                            "type": "integer",
                            "description": "Number of results to return (default 5, max 20)"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "navigate".to_string(),
                description: "Navigate to a URL or search query".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to navigate to or search query"
                        }
                    },
                    "required": ["url"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "follow_link".to_string(),
                description: "Follow a link by its number (1-indexed)".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "link_number": {
                            "type": "integer",
                            "description": "Link number to follow (starting from 1)"
                        }
                    },
                    "required": ["link_number"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "scroll".to_string(),
                description: "Scroll the page content".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "direction": {
                            "type": "string",
                            "enum": ["up", "down", "top", "bottom"],
                            "description": "Direction to scroll"
                        },
                        "amount": {
                            "type": "integer",
                            "description": "Number of lines to scroll (default 5)"
                        }
                    },
                    "required": ["direction"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_page_content".to_string(),
                description: "Get the full content of the current page".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_links".to_string(),
                description: "List all links on the current page".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
    ]
}

/// Execute Web Search (Brave or Fallback)
fn brave_search(query: &str, count: usize) -> Result<String> {
    let api_key = std::env::var("BRAVE_API_KEY")
        .unwrap_or_default();

    // Fallback to internal search if no Brave key
    if api_key.is_empty() {
        // Use internal SearchManager (DuckDuckGo)
        let manager = SearchManager::new()?;
        // Use general search (DuckDuckGo)
        let engine = crate::search::EngineType::DuckDuckGo;
        let response = manager.search_with(engine, query)?;
        
        if response.results.is_empty() {
            return Ok("No results found via DuckDuckGo.".to_string());
        }

        let mut formatted = Vec::new();
        for (i, result) in response.results.iter().take(count).enumerate() {
            formatted.push(format!(
                "{}. **{}**\n   {}\n   {}\n",
                i + 1, result.title, result.url, result.description
            ));
        }
        
        return Ok(format!("## Search Results (DDG) for: {}\n\n{}", query, formatted.join("\n")));
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let count = count.min(20).max(1);
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
        urlencoding::encode(query),
        count
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", &api_key)
        .send()?;

    if !response.status().is_success() {
        return Ok(format!("Search failed: {}", response.status()));
    }

    let json: serde_json::Value = response.json()?;

    // Extract web results
    let mut results = Vec::new();
    if let Some(web) = json.get("web").and_then(|w| w.get("results")).and_then(|r| r.as_array()) {
        for (i, result) in web.iter().take(count).enumerate() {
            let title = result.get("title").and_then(|t| t.as_str()).unwrap_or("No title");
            let url = result.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let description = result.get("description").and_then(|d| d.as_str()).unwrap_or("No description");

            results.push(format!(
                "{}. **{}**\n   {}\n   {}\n",
                i + 1, title, url, description
            ));
        }
    }

    if results.is_empty() {
        Ok("No results found".to_string())
    } else {
        Ok(format!("## Search Results for: {}\n\n{}", query, results.join("\n")))
    }
}

/// Response from chat API - can be content or tool calls
enum ChatApiResponse {
    Content(String),
    ToolCalls(Vec<ToolCall>),
}

fn send_chat_message_with_tools(
    api_key: &str,
    model: &str,
    base_url: &str,
    messages: &[OpenAIChatMessage],
    include_tools: bool,
) -> Result<ChatApiResponse> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let tools = if include_tools {
        Some(get_browser_tools())
    } else {
        None
    };

    let request = OpenAIChatRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        max_tokens: 1024,
        temperature: 0.7,
        tools,
    };

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/ser/azul-browse")
        .header("X-Title", "Azul Terminal Browser")
        .json(&request)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("API error ({}): {}", status, error_text);
    }

    let result: OpenAIChatResponse = response.json()?;

    if let Some(error) = result.error {
        anyhow::bail!("API error: {}", error.message);
    }

    if let Some(choices) = result.choices {
        if let Some(choice) = choices.first() {
            // Check if there are tool calls
            if let Some(tool_calls) = &choice.message.tool_calls {
                if !tool_calls.is_empty() {
                    return Ok(ChatApiResponse::ToolCalls(tool_calls.clone()));
                }
            }
            // Otherwise return content
            if let Some(content) = &choice.message.content {
                Ok(ChatApiResponse::Content(content.clone()))
            } else {
                Ok(ChatApiResponse::Content(String::new()))
            }
        } else {
            anyhow::bail!("No choices in API response")
        }
    } else {
        anyhow::bail!("No response from API")
    }
}

/// Execute a tool call - sends action to app and returns result string
fn execute_tool_call(
    tool_call: &ToolCall,
    context: &BrowserContext,
    tx: &Sender<AppMessage>,
) -> String {
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
        .unwrap_or(serde_json::json!({}));

    match tool_call.function.name.as_str() {
        "brave_search" => {
            if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
                let count = args.get("count").and_then(|v| v.as_i64()).unwrap_or(5) as usize;
                match brave_search(query, count) {
                    Ok(results) => results,
                    Err(e) => format!("Search error: {}", e),
                }
            } else {
                "Error: Missing 'query' parameter".to_string()
            }
        }
        "navigate" => {
            if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                let _ = tx.send(AppMessage::ToolAction(ToolAction::Navigate(url.to_string())));
                format!("Navigating to: {}", url)
            } else {
                "Error: Missing 'url' parameter".to_string()
            }
        }
        "follow_link" => {
            if let Some(link_num) = args.get("link_number").and_then(|v| v.as_i64()) {
                let idx = (link_num - 1) as usize;
                if idx < context.links.len() {
                    let _ = tx.send(AppMessage::ToolAction(ToolAction::FollowLink(idx)));
                    format!("Following link {}", link_num)
                } else {
                    format!("Error: Link {} not found. Available links: 1-{}", link_num, context.links.len())
                }
            } else {
                "Error: Missing 'link_number' parameter".to_string()
            }
        }
        "scroll" => {
            let direction = args.get("direction").and_then(|v| v.as_str()).unwrap_or("down").to_string();
            let amount = args.get("amount").and_then(|v| v.as_i64()).unwrap_or(5) as usize;
            let _ = tx.send(AppMessage::ToolAction(ToolAction::Scroll {
                direction: direction.clone(),
                amount
            }));
            format!("Scrolling {} by {} lines", direction, amount)
        }
        "get_page_content" => {
            if context.content.is_empty() {
                "No page currently loaded.".to_string()
            } else {
                format!("Page content:\n{}", context.content)
            }
        }
        "list_links" => {
            if context.links.is_empty() {
                "No links on current page.".to_string()
            } else {
                let links_list: Vec<String> = context.links.iter()
                    .enumerate()
                    .map(|(i, (text, url))| format!("{}. {} ({})", i + 1, text, url))
                    .collect();
                format!("Links on page:\n{}", links_list.join("\n"))
            }
        }
        _ => format!("Unknown tool: {}", tool_call.function.name),
    }
}

/// Browser context for tool execution
struct BrowserContext {
    title: String,
    url: String,
    content: String,
    links: Vec<(String, String)>, // (text, url)
}

fn send_chat_message(
    api_key: &str,
    model: &str,
    base_url: &str,
    messages: &[ChatMessage],
    tx: &Sender<AppMessage>,
) -> Result<String> {
    // Convert our messages to OpenAI format
    let mut api_messages: Vec<OpenAIChatMessage> = messages
        .iter()
        .map(|m| OpenAIChatMessage {
            role: match m.role {
                crate::chat::Role::System => "system".to_string(),
                crate::chat::Role::User => "user".to_string(),
                crate::chat::Role::Assistant => "assistant".to_string(),
                crate::chat::Role::Tool => "tool".to_string(),
            },
            content: Some(m.content.clone()),
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();

    // Extract browser context from system message if available
    let context = extract_browser_context(&api_messages);

    // Try with tools first, then without if it fails
    let response = match send_chat_message_with_tools(api_key, model, base_url, &api_messages, true) {
        Ok(resp) => resp,
        Err(e) => {
            // Log the tool error and retry without tools
            eprintln!("Tools not supported ({}), retrying without tools...", e);
            send_chat_message_with_tools(api_key, model, base_url, &api_messages, false)?
        }
    };

    match response {
        ChatApiResponse::Content(content) => Ok(content),
        ChatApiResponse::ToolCalls(tool_calls) => {
            // Build a summary of tool calls for display
            let tool_summary: Vec<String> = tool_calls.iter()
                .map(|tc| format!("⚙ {}", tc.function.name))
                .collect();

            // Execute tool calls and send results back
            let assistant_msg = OpenAIChatMessage {
                role: "assistant".to_string(),
                content: None,
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
            };
            api_messages.push(assistant_msg);

            // Execute each tool and add results
            let mut tool_results: Vec<String> = Vec::new();
            for tool_call in &tool_calls {
                let result = execute_tool_call(tool_call, &context, tx);
                tool_results.push(format!("  ↳ {}", result));
                api_messages.push(OpenAIChatMessage {
                    role: "tool".to_string(),
                    content: Some(result),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }

            // Continue conversation without tools to get final response
            let final_content = match send_chat_message_with_tools(api_key, model, base_url, &api_messages, false)? {
                ChatApiResponse::Content(content) => content,
                ChatApiResponse::ToolCalls(_) => String::new(),
            };

            // Combine tool summary with final response
            let mut full_response = tool_summary.join("\n");
            if !tool_results.is_empty() {
                full_response.push_str("\n");
                full_response.push_str(&tool_results.join("\n"));
            }
            if !final_content.is_empty() {
                full_response.push_str("\n\n");
                full_response.push_str(&final_content);
            }

            Ok(full_response)
        }
    }
}

/// Extract browser context from messages
fn extract_browser_context(messages: &[OpenAIChatMessage]) -> BrowserContext {
    let mut context = BrowserContext {
        title: String::new(),
        url: String::new(),
        content: String::new(),
        links: Vec::new(),
    };

    // Look for context in user messages
    for msg in messages {
        if msg.role == "user" {
            if let Some(content) = &msg.content {
                if content.contains("[Browser Context]") {
                    // Parse the context
                    for line in content.lines() {
                        if line.starts_with("Current page:") {
                            let parts: Vec<&str> = line.split('(').collect();
                            if parts.len() >= 2 {
                                context.title = parts[0].replace("Current page:", "").trim().to_string();
                                context.url = parts[1].trim_end_matches(')').to_string();
                            }
                        } else if line.starts_with("Content preview:") {
                            // Content follows after this line
                            let idx = content.find("Content preview:").unwrap_or(0);
                            context.content = content[idx..].lines().skip(1).take(10).collect::<Vec<_>>().join("\n");
                        }
                    }
                }
            }
        }
    }

    context
}

fn send_chat_message_with_fallback(
    api_key: &str,
    base_url: &str,
    models: &[String],
    messages: &[ChatMessage],
    tx: &Sender<AppMessage>,
) -> Result<String> {
    let mut errors = Vec::new();

    for (i, model) in models.iter().enumerate() {
        match send_chat_message(api_key, model, base_url, messages, tx) {
            Ok(response) => {
                if i > 0 {
                    // Used a fallback model
                    eprintln!("Fallback: Using model {} after {} failures", model, i);
                }
                return Ok(response);
            }
            Err(e) => {
                let error_msg = e.to_string();
                let is_rate_limit = error_msg.contains("429")
                    || error_msg.contains("rate limit")
                    || error_msg.contains("Rate limit")
                    || error_msg.contains("Too Many Requests");

                errors.push(format!("{}: {}", model, error_msg));

                // If it's a rate limit, try next model
                if is_rate_limit {
                    eprintln!("Rate limit hit on {}, trying next model...", model);
                    continue;
                }

                // If it's not a rate limit, fail immediately
                // (e.g., invalid API key, network error)
                return Err(e);
            }
        }
    }

    // All models failed
    anyhow::bail!("All models failed:\n{}", errors.join("\n"))
}
