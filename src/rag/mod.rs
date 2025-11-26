//! RAG (Retrieval-Augmented Generation) integration with home-rag

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// RAG query request
#[derive(Debug, Serialize)]
struct RagQueryRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<serde_json::Value>,
}

/// RAG query response
#[derive(Debug, Deserialize)]
pub struct RagResponse {
    pub answer: Option<String>,
    pub sources: Vec<RagSource>,
    pub context: Vec<String>,
}

/// Source document from RAG
#[derive(Debug, Clone, Deserialize)]
pub struct RagSource {
    pub content: String,
    pub metadata: RagMetadata,
    pub score: f32,
}

/// Metadata for RAG source
#[derive(Debug, Clone, Deserialize)]
pub struct RagMetadata {
    pub file_path: Option<String>,
    pub document_type: Option<String>,
    pub chunk_index: Option<usize>,
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
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Check if RAG service is available
        let enabled = client
            .get(format!("{}/health", base_url))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        Ok(Self {
            base_url,
            client,
            enabled,
        })
    }

    /// Create from config
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        let base_url = config
            .rag_base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:8000".to_string());

        Self::new(base_url).ok()
    }

    /// Check if RAG is available
    pub fn is_available(&self) -> bool {
        self.enabled
    }

    /// Query the RAG system
    pub fn query(&self, query: &str, top_k: Option<usize>) -> Result<RagResponse> {
        if !self.enabled {
            anyhow::bail!("RAG service is not available");
        }

        let request = RagQueryRequest {
            query: query.to_string(),
            top_k,
            filters: None,
        };

        let response = self
            .client
            .post(format!("{}/query", self.base_url))
            .json(&request)
            .send()
            .context("Failed to send RAG query")?;

        if !response.status().is_success() {
            anyhow::bail!("RAG query failed with status: {}", response.status());
        }

        let result: RagResponse = response.json().context("Failed to parse RAG response")?;
        Ok(result)
    }

    /// Get document context for a query
    pub fn get_context(&self, query: &str, max_chunks: usize) -> Result<Vec<String>> {
        let response = self.query(query, Some(max_chunks))?;
        Ok(response.context)
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
