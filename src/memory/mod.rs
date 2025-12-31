//! Memory integration with mem-layer graph-based memory system

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Memory node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Bookmark,
    Page,
    Search,
    Conversation,
    Concept,
    Note,
}

/// Memory node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub node_type: NodeType,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
}

/// Memory relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelation {
    pub from_id: String,
    pub to_id: String,
    pub rel_type: String,
}

/// Client for mem-layer CLI
pub struct MemoryClient {
    scope: String,
    mem_layer_path: String,
    enabled: bool,
}

impl MemoryClient {
    /// Create a new memory client
    pub fn new(scope: String) -> Self {
        // Check common locations for mem-layer
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        let common_paths = [
            format!("{home}/.local/bin/mem-layer"),
            format!("{home}/bin/mem-layer"),
            "/usr/local/bin/mem-layer".to_string(),
            "/usr/bin/mem-layer".to_string(),
            "mem-layer".to_string(), // Fall back to PATH lookup
        ];

        let mut mem_layer_path = "mem-layer".to_string();
        let mut enabled = false;

        // Try each path until one works
        for path in &common_paths {
            if let Ok(output) = Command::new(path).arg("--version").output() {
                if output.status.success() {
                    mem_layer_path = path.clone();
                    enabled = true;
                    break;
                }
            }
        }

        // Also try which as a fallback
        if !enabled {
            if let Ok(path) = which::which("mem-layer") {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(output) = Command::new(&path_str).arg("--version").output() {
                    if output.status.success() {
                        mem_layer_path = path_str;
                        enabled = true;
                    }
                }
            }
        }

        Self {
            scope,
            mem_layer_path,
            enabled,
        }
    }

    /// Create from config
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        let scope = config
            .memory_scope
            .clone()
            .unwrap_or_else(|| "azul-browse".to_string());

        let client = Self::new(scope);
        if client.is_available() {
            Some(client)
        } else {
            None
        }
    }

    /// Check if mem-layer is available
    pub fn is_available(&self) -> bool {
        self.enabled
    }

    /// Add a note to memory
    pub fn add_note(
        &self,
        content: &str,
        tags: &[String],
        priority: Option<&str>,
    ) -> Result<String> {
        if !self.enabled {
            anyhow::bail!("mem-layer is not available");
        }

        let mut cmd = Command::new(&self.mem_layer_path);
        cmd.arg("add")
            .arg("note")
            .arg(content)
            .arg("--scope")
            .arg(&self.scope);

        if !tags.is_empty() {
            cmd.arg("--tags").arg(tags.join(","));
        }

        if let Some(p) = priority {
            cmd.arg("--priority").arg(p);
        }

        let output = cmd.output().context("Failed to execute mem-layer")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("mem-layer add failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    /// Search memory
    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        if !self.enabled {
            anyhow::bail!("mem-layer is not available");
        }

        let output = Command::new(&self.mem_layer_path)
            .arg("search")
            .arg(query)
            .arg("--scope")
            .arg(&self.scope)
            .output()
            .context("Failed to execute mem-layer")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("mem-layer search failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(results)
    }

    /// List nodes
    pub fn list_nodes(&self, node_type: Option<&str>, limit: Option<usize>) -> Result<Vec<String>> {
        if !self.enabled {
            anyhow::bail!("mem-layer is not available");
        }

        let mut cmd = Command::new(&self.mem_layer_path);
        cmd.arg("list")
            .arg("--scope")
            .arg(&self.scope);

        if let Some(t) = node_type {
            cmd.arg("--type").arg(t);
        }

        if let Some(l) = limit {
            cmd.arg("--limit").arg(l.to_string());
        }

        let output = cmd.output().context("Failed to execute mem-layer")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("mem-layer list failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the table output - skip header lines and decorations
        let nodes: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter(|line| !line.starts_with("┏"))
            .filter(|line| !line.starts_with("┡"))
            .filter(|line| !line.starts_with("└"))
            .filter(|line| !line.starts_with("━"))
            .filter(|line| !line.contains("INFO - "))
            .filter(|line| !line.trim_start().starts_with("ID"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(nodes)
    }

    /// Store a browsing session
    pub fn store_page(&self, url: &str, title: &str, tags: &[String]) -> Result<String> {
        let content = format!("Page: {} ({})", title, url);
        self.add_note(&content, tags, Some("low"))
    }

    /// Store a chat conversation
    pub fn store_conversation(&self, query: &str, response: &str) -> Result<String> {
        let content = format!("Q: {}\nA: {}", query, response);
        self.add_note(&content, &["chat".to_string()], Some("medium"))
    }

    /// Get recent memory context
    pub fn get_recent_context(&self, limit: usize) -> Result<Vec<String>> {
        self.list_nodes(None, Some(limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_client_creation() {
        let client = MemoryClient::new("test-scope".to_string());
        // Should not panic even if mem-layer is not available
        assert!(!client.scope.is_empty());
    }
}
