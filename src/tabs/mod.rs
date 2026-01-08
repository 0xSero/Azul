//! Tab management for multiple browser tabs

use crate::browser::{Page, RenderMode};

/// A single browser tab
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: usize,
    pub url: String,
    pub title: String,
    pub page: Option<Page>,
    pub loading: bool,
    pub error: Option<String>,
    pub scroll_offset: usize,
    pub sidebar_selected: usize,
    pub render_mode: RenderMode,

    // Navigation history within tab
    history: Vec<String>,
    history_index: isize,
}

impl Tab {
    /// Create a new tab with welcome content
    pub fn new(id: usize) -> Self {
        use crate::browser::page::Link;

        // Create a welcome page with suggested links
        let welcome_page = Page {
            url: "azul://home".to_string(),
            title: "Welcome to Azul".to_string(),
            content_lines: vec![
                "# Welcome to Azul".to_string(),
                "".to_string(),
                "**Terminal Web Browser**".to_string(),
                "".to_string(),
                "---".to_string(),
                "".to_string(),
                "## Getting Started".to_string(),
                "".to_string(),
                "- Press `/` to search or enter a URL".to_string(),
                "- Press `?` for help".to_string(),
                "- Press `1` for links, `2` for content, `3` for chat".to_string(),
                "".to_string(),
                "## Quick Links".to_string(),
                "".to_string(),
                "Browse the **Links** panel on the left to visit suggested sites.".to_string(),
                "".to_string(),
                "## AI Assistant".to_string(),
                "".to_string(),
                "Chat with the AI (press `c` or `3`) - it can search the web, summarize pages, and help you navigate.".to_string(),
                "".to_string(),
            ],
            raw_content: "Welcome to Azul Browser".to_string(),
            links: vec![
                Link { text: "Ethers Blog".to_string(), url: "https://blog.ethers.club".to_string() },
                Link { text: "Hacker News".to_string(), url: "https://news.ycombinator.com".to_string() },
                Link { text: "Wikipedia".to_string(), url: "https://en.wikipedia.org".to_string() },
                Link { text: "arXiv".to_string(), url: "https://arxiv.org".to_string() },
                Link { text: "GitHub".to_string(), url: "https://github.com".to_string() },
                Link { text: "Lobsters".to_string(), url: "https://lobste.rs".to_string() },
                Link { text: "DuckDuckGo".to_string(), url: "https://duckduckgo.com".to_string() },
            ],
        };

        Self {
            id,
            url: "azul://home".to_string(),
            title: "Welcome to Azul".to_string(),
            page: Some(welcome_page),
            loading: false,
            error: None,
            scroll_offset: 0,
            sidebar_selected: 0,
            render_mode: RenderMode::Auto,
            history: Vec::new(),
            history_index: -1,
        }
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: &str) {
        // Trim history after current position
        if self.history_index >= 0
            && (self.history_index as usize) < self.history.len().saturating_sub(1)
        {
            self.history.truncate((self.history_index + 1) as usize);
        }

        self.history.push(url.to_string());
        self.history_index = self.history.len() as isize - 1;
        self.url = url.to_string();
        self.loading = true;
        self.error = None;
    }

    /// Set the loaded page content
    pub fn set_page(&mut self, page: Page) {
        self.title = page.title.clone();
        self.url = page.url.clone();
        self.page = Some(page);
        self.loading = false;
        self.error = None;
        self.scroll_offset = 0;
        self.sidebar_selected = 0;
    }

    /// Set an error state
    pub fn set_error(&mut self, error: String) {
        self.loading = false;
        self.error = Some(error);
    }

    /// Check if we can go back in history
    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    /// Check if we can go forward in history
    pub fn can_go_forward(&self) -> bool {
        self.history_index >= 0
            && (self.history_index as usize) < self.history.len().saturating_sub(1)
    }

    /// Go back in history, returns the URL to navigate to
    pub fn go_back(&mut self) -> Option<String> {
        if !self.can_go_back() {
            return None;
        }
        self.history_index -= 1;
        self.url = self.history[self.history_index as usize].clone();
        Some(self.url.clone())
    }

    /// Go forward in history, returns the URL to navigate to
    pub fn go_forward(&mut self) -> Option<String> {
        if !self.can_go_forward() {
            return None;
        }
        self.history_index += 1;
        self.url = self.history[self.history_index as usize].clone();
        Some(self.url.clone())
    }

    /// Get display title (truncated if too long)
    pub fn display_title(&self, max_len: usize) -> String {
        let title = if self.loading {
            "Loading...".to_string()
        } else if self.title.is_empty() {
            "New Tab".to_string()
        } else {
            self.title.clone()
        };

        if title.len() > max_len {
            format!("{}...", &title[..max_len.saturating_sub(3)])
        } else {
            title
        }
    }
}

/// Manages multiple browser tabs
pub struct TabManager {
    tabs: Vec<Tab>,
    active: usize,
    next_id: usize,
    max_tabs: usize,
}

impl TabManager {
    /// Create a new tab manager
    pub fn new() -> Self {
        let mut manager = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 0,
            max_tabs: 9,
        };

        // Start with one empty tab
        manager.new_tab(None);
        manager
    }

    /// Create a new tab and return its index
    pub fn new_tab(&mut self, url: Option<&str>) -> Option<usize> {
        if self.tabs.len() >= self.max_tabs {
            return None;
        }

        let mut tab = Tab::new(self.next_id);
        self.next_id += 1;

        if let Some(url) = url {
            tab.navigate(url);
        }

        self.tabs.push(tab);
        let idx = self.tabs.len() - 1;
        self.active = idx;

        Some(idx)
    }

    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || self.tabs.len() <= 1 {
            // Don't close last tab
            return false;
        }

        self.tabs.remove(index);

        // Update active index
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        } else if index < self.active {
            self.active = self.active.saturating_sub(1);
        }

        true
    }

    /// Close current tab
    pub fn close_current_tab(&mut self) -> bool {
        self.close_tab(self.active)
    }

    /// Get the active tab
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    /// Get the active tab mutably
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    /// Get active tab index
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Set active tab by index
    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            if self.active == 0 {
                self.active = self.tabs.len() - 1;
            } else {
                self.active -= 1;
            }
        }
    }

    /// Go to tab by number (1-9)
    pub fn go_to_tab(&mut self, num: usize) {
        if num > 0 && num <= self.tabs.len() {
            self.active = num - 1;
        }
    }

    /// Get number of tabs
    pub fn count(&self) -> usize {
        self.tabs.len()
    }

    /// Get all tabs
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Get a tab by index
    pub fn get_tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    /// Get a tab by index mutably
    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(index)
    }

    /// Check if at max tabs
    pub fn is_full(&self) -> bool {
        self.tabs.len() >= self.max_tabs
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tab_manager() {
        let manager = TabManager::new();
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_add_and_close_tabs() {
        let mut manager = TabManager::new();

        // Add a second tab
        manager.new_tab(Some("https://example.com"));
        assert_eq!(manager.count(), 2);
        assert_eq!(manager.active_index(), 1);

        // Close it
        manager.close_current_tab();
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_tab_navigation() {
        let mut manager = TabManager::new();
        manager.new_tab(None);
        manager.new_tab(None);

        assert_eq!(manager.count(), 3);
        assert_eq!(manager.active_index(), 2);

        manager.prev_tab();
        assert_eq!(manager.active_index(), 1);

        manager.next_tab();
        assert_eq!(manager.active_index(), 2);

        manager.go_to_tab(1);
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_tab_history() {
        let mut tab = Tab::new(0);

        tab.navigate("https://example.com");
        tab.navigate("https://example.org");

        assert!(tab.can_go_back());
        assert!(!tab.can_go_forward());

        let url = tab.go_back();
        assert_eq!(url, Some("https://example.com".to_string()));
        assert!(tab.can_go_forward());

        let url = tab.go_forward();
        assert_eq!(url, Some("https://example.org".to_string()));
    }
}
