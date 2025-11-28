//! RAG (Retrieval-Augmented Generation) integration with home-rag

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// RAG query request - matches home-rag API
#[derive(Debug, Serialize)]
struct RagQueryRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_graph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<serde_json::Value>,
}

/// RAG query response - matches home-rag QueryResponse
#[derive(Debug, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<RagSource>,
    pub query: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Source document from RAG - flexible to handle various formats
#[derive(Debug, Clone, Deserialize)]
pub struct RagSource {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub text: Option<String>,  // Alternative field name
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub similarity: Option<f32>,  // Alternative field name
    #[serde(flatten)]
    pub extra: serde_json::Value,  // Capture any extra fields
}

/// RAG client for querying home-rag
pub struct RagClient {
    base_url: String,
    client: reqwest::blocking::Client,
    enabled: bool,
}

impl RagClient {
    /// Create a new RAG client
    pub fn new(base_url: String) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))  // RAG queries can take a while
            .build()
            .context("Failed to create HTTP client")?;

        // Check if RAG service is available (try both /health and root)
        let enabled = client
            .get(format!("{}/health", base_url))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or_else(|_| {
                // Fallback: try root endpoint
                client
                    .get(&base_url)
                    .send()
                    .map(|r| r.status().is_success())
                    .unwrap_or(false)
            });

        Ok(Self {
            base_url,
            client,
            enabled,
        })
    }

    /// Create from config
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        // Only enable if explicitly configured
        let base_url = config.rag_base_url.as_ref()?;
        Self::new(base_url.clone()).ok()
    }

    /// Check if RAG is available
    pub fn is_available(&self) -> bool {
        self.enabled
    }

    /// Query the RAG system
    pub fn query(&self, query: &str, top_k: Option<usize>) -> Result<RagResponse> {
        if !self.enabled {
            anyhow::bail!("RAG service is not available. Check home-rag at {}", self.base_url);
        }

        let request = RagQueryRequest {
            query: query.to_string(),
            top_k,
            use_graph: Some(true),
            filters: None,
        };

        let response = self
            .client
            .post(format!("{}/query", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .context("Failed to send RAG query")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("RAG query failed ({}): {}", status, text);
        }

        // Try to parse as JSON, with fallback for unexpected formats
        let text = response.text().context("Failed to read RAG response")?;

        match serde_json::from_str::<RagResponse>(&text) {
            Ok(result) => Ok(result),
            Err(e) => {
                // Try to extract useful info from the response
                anyhow::bail!("Failed to parse RAG response: {}. Response: {}", e,
                    if text.len() > 300 { &text[..300] } else { &text })
            }
        }
    }

    /// Get document context for a query
    pub fn get_context(&self, query: &str, max_chunks: usize) -> Result<Vec<String>> {
        let response = self.query(query, Some(max_chunks))?;
        // Extract content from sources
        let context: Vec<String> = response.sources
            .iter()
            .map(|s| s.text.as_ref().unwrap_or(&s.content).clone())
            .collect();
        Ok(context)
    }

    /// Check if service is healthy
    pub fn health_check(&mut self) -> bool {
        if let Ok(response) = self.client.get(format!("{}/health", self.base_url)).send() {
            let healthy = response.status().is_success();
            self.enabled = healthy;
            healthy
        } else {
            self.enabled = false;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_client_creation() {
        // Should not panic even if service is unavailable
        let client = RagClient::new("http://localhost:8000".to_string());
        assert!(client.is_ok());
    }
}
