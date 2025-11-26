//! AI Chat functionality with browser tool integration

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::browser::Page;
use crate::storage::{Bookmark, HistoryEntry};

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Tool call from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Available browser tools for AI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BrowserTool {
    GetCurrentPage,
    GetLinks,
    GetBookmarks,
    GetHistory,
    Navigate { url: String },
    Search { query: String },
    AddBookmark { url: String, title: String },
}

impl ChatMessage {
    pub fn new_user(content: String) -> Self {
        Self {
            role: Role::User,
            content,
            timestamp: Utc::now(),
            tool_calls: None,
        }
    }

    pub fn new_assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            timestamp: Utc::now(),
            tool_calls: None,
        }
    }

    pub fn new_system(content: String) -> Self {
        Self {
            role: Role::System,
            content,
            timestamp: Utc::now(),
            tool_calls: None,
        }
    }
}

/// Chat session
pub struct ChatSession {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub available_models: Vec<String>,
}

impl ChatSession {
    pub fn new(model: String, available_models: Vec<String>) -> Self {
        let mut session = Self {
            messages: Vec::new(),
            model,
            available_models,
        };

        // Add system message with browser tools
        session.add_system_message();
        session
    }

    fn add_system_message(&mut self) {
        let system_prompt = r#"You are Azul Assistant, an AI helper integrated into the Azul terminal web browser.

You have access to the following browser tools:
- get_current_page: Get the currently displayed page content and metadata
- get_links: Get all links from the current page
- get_bookmarks: Get user's bookmarks
- get_history: Get browsing history
- navigate: Navigate to a URL
- search: Perform a web search
- add_bookmark: Bookmark the current page

When the user asks about the current page, links, or wants to navigate, use these tools.
Be helpful, concise, and focused on assisting with web browsing tasks."#;

        self.messages.push(ChatMessage::new_system(system_prompt.to_string()));
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage::new_user(content));
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(ChatMessage::new_assistant(content));
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.add_system_message();
    }
}

/// Browser context for AI tools
#[derive(Debug, Clone)]
pub struct BrowserContext {
    pub current_page: Option<Page>,
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<HistoryEntry>,
}

impl BrowserContext {
    pub fn new() -> Self {
        Self {
            current_page: None,
            bookmarks: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Convert to a JSON string for AI context
    pub fn to_json(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Context {
            current_page: Option<PageSummary>,
            bookmarks_count: usize,
            history_count: usize,
        }

        #[derive(Serialize)]
        struct PageSummary {
            title: String,
            url: String,
            links_count: usize,
            content_preview: String,
        }

        let current_page = self.current_page.as_ref().map(|p| PageSummary {
            title: p.title.clone(),
            url: p.url.clone(),
            links_count: p.links.len(),
            content_preview: p.content_lines
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
        });

        let ctx = Context {
            current_page,
            bookmarks_count: self.bookmarks.len(),
            history_count: self.history.len(),
        };

        Ok(serde_json::to_string_pretty(&ctx)?)
    }

    /// Get links as formatted text
    pub fn get_links_text(&self) -> String {
        if let Some(page) = &self.current_page {
            page.links
                .iter()
                .take(20)
                .enumerate()
                .map(|(i, link)| format!("{}. {} -> {}", i + 1, link.text, link.url))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "No page loaded".to_string()
        }
    }

    /// Get bookmarks as formatted text
    pub fn get_bookmarks_text(&self) -> String {
        if self.bookmarks.is_empty() {
            "No bookmarks".to_string()
        } else {
            self.bookmarks
                .iter()
                .take(20)
                .map(|b| format!("{} - {}", b.title, b.url))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    /// Get history as formatted text
    pub fn get_history_text(&self) -> String {
        if self.history.is_empty() {
            "No history".to_string()
        } else {
            self.history
                .iter()
                .take(20)
                .map(|h| format!("{} - {}", h.title, h.url))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

impl Default for BrowserContext {
    fn default() -> Self {
        Self::new()
    }
}
