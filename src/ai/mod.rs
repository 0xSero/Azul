use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// AI Provider trait for LLM backends
pub trait Provider: Send + Sync {
    fn complete(&self, prompt: &str) -> Result<String>;
    fn complete_with_system(&self, system: &str, user: &str) -> Result<String>;
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
}

/// OpenRouter provider with fallback model support
pub struct OpenRouterProvider {
    api_key: String,
    models: Vec<String>,
    base_url: String,
    client: reqwest::blocking::Client,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Option<Vec<OpenAIChoice>>,
    error: Option<OpenAIError>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    message: String,
}

/// Default fallback models for OpenRouter (free tier)
pub fn default_fallback_models() -> Vec<String> {
    vec![
        "x-ai/grok-4.1-fast:free".to_string(),
        "kwaipilot/kat-coder-pro:free".to_string(),
        "z-ai/glm-4.5-air:free".to_string(),
        "moonshotai/kimi-k2:free".to_string(),
    ]
}

impl OpenRouterProvider {
    pub fn new(api_key: String, models: Option<Vec<String>>, base_url: Option<String>) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        let models = models.unwrap_or_else(default_fallback_models);
        let base_url = base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());

        Ok(Self {
            api_key,
            models,
            base_url,
            client,
        })
    }

    fn try_model(&self, system: &str, user: &str, model: &str) -> Result<String> {
        let mut messages = vec![];

        if !system.is_empty() {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system.to_string(),
            });
        }

        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: user.to_string(),
        });

        let request = OpenAIRequest {
            model: model.to_string(),
            messages,
            max_tokens: 1024,
            temperature: 0.7,
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/ser/azul-browse")
            .header("X-Title", "Azul Terminal Browser")
            .json(&request)
            .send()
            .context("Failed to send request")?;

        let result: OpenAIResponse = response.json().context("Failed to parse response")?;

        if let Some(error) = result.error {
            anyhow::bail!("API error: {}", error.message);
        }

        let choices = result.choices.context("No choices in response")?;
        let first = choices.first().context("Empty choices array")?;

        Ok(first.message.content.clone())
    }
}

impl Provider for OpenRouterProvider {
    fn complete(&self, prompt: &str) -> Result<String> {
        self.complete_with_system("", prompt)
    }

    fn complete_with_system(&self, system: &str, user: &str) -> Result<String> {
        let mut errors = vec![];

        for model in &self.models {
            match self.try_model(system, user, model) {
                Ok(response) => return Ok(response),
                Err(e) => {
                    errors.push(format!("{}: {}", model, e));
                }
            }
        }

        anyhow::bail!("All models failed: {}", errors.join("; "))
    }

    fn name(&self) -> &str {
        "OpenRouter"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// Summarizer for AI-powered content summarization
pub struct Summarizer {
    provider: Box<dyn Provider>,
}

/// Search result for summarization
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: String,
    pub source: String,
}

impl Summarizer {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self { provider }
    }

    /// Create a summarizer from config
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        let api_key = config.get_api_key()?.to_string();
        if api_key.is_empty() {
            return None;
        }

        // Get model and fallback models from config
        let model = config.get_ai_model().map(String::from);
        let fallback_models = config.get_fallback_models();
        let base_url = config.get_ai_base_url().map(String::from);

        // Build models list: primary model + fallback models
        let mut models = Vec::new();
        if let Some(m) = model {
            models.push(m);
        }
        if let Some(fallback) = fallback_models {
            models.extend(fallback);
        }

        // If no models specified, use the default primary model
        if models.is_empty() {
            models.push("qwen/qwen3-235b-a22b:free".to_string());
        }

        let provider = OpenRouterProvider::new(api_key, Some(models), base_url).ok()?;
        Some(Self::new(Box::new(provider)))
    }

    /// Create a summarizer from environment config (deprecated, use from_config)
    pub fn from_env() -> Option<Self> {
        let config = crate::config::Config::load().ok()?;
        Self::from_config(&config)
    }

    /// Summarize search results
    pub fn summarize_search(&self, query: &str, results: &[SearchResult]) -> Result<String> {
        if results.is_empty() {
            anyhow::bail!("No results to summarize");
        }

        let mut context = String::new();
        for (i, r) in results.iter().take(10).enumerate() {
            context.push_str(&format!("Result {}:\n", i + 1));
            context.push_str(&format!("Title: {}\n", r.title));
            context.push_str(&format!("Source: {}\n", r.source));
            if !r.description.is_empty() {
                context.push_str(&format!("Description: {}\n", r.description));
            }
            context.push('\n');
        }

        let system_prompt = r#"You are a helpful research assistant. Analyze the search results and provide a concise summary.
Your summary should:
1. Identify the main themes or findings across results
2. Highlight the most relevant information for the query
3. Note any conflicting information if present
4. Be 2-4 sentences long
5. Be objective and factual

Do NOT include phrases like "Based on the search results" or "The results show". Just provide the summary directly."#;

        let user_prompt = format!(
            "Query: {}\n\nSearch Results:\n{}\n\nProvide a brief summary of the key information from these results.",
            query, context
        );

        self.provider.complete_with_system(system_prompt, &user_prompt)
    }

    /// Summarize a webpage
    pub fn summarize_page(&self, title: &str, url: &str, content: &str) -> Result<String> {
        let truncated = if content.len() > 4000 {
            format!("{}...", &content[..4000])
        } else {
            content.to_string()
        };

        let system_prompt = r#"You are a helpful assistant that summarizes web pages. Provide a clear, concise summary that captures the main points.
Your summary should:
1. Be 3-5 sentences long
2. Focus on the key information and main points
3. Be objective and factual
4. Not include any meta-commentary about the summarization process"#;

        let user_prompt = format!(
            "Title: {}\nURL: {}\n\nContent:\n{}\n\nSummarize this page.",
            title, url, truncated
        );

        self.provider.complete_with_system(system_prompt, &user_prompt)
    }

    /// Extract key points from content
    pub fn extract_key_points(&self, content: &str, count: usize) -> Result<Vec<String>> {
        let count = if count == 0 { 5 } else { count };
        let truncated = if content.len() > 4000 {
            format!("{}...", &content[..4000])
        } else {
            content.to_string()
        };

        let system_prompt = format!(
            r#"Extract exactly {} key points from the given content.
Format your response as a numbered list, one point per line.
Each point should be a single, clear sentence.
Example format:
1. First key point
2. Second key point
..."#,
            count
        );

        let response = self.provider.complete_with_system(&system_prompt, &truncated)?;

        // Parse numbered list
        let points: Vec<String> = response
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| {
                // Remove number prefix
                let mut result = line.to_string();
                for i in 0..10 {
                    let prefix1 = format!("{}. ", i);
                    let prefix2 = format!("{}) ", i);
                    if result.starts_with(&prefix1) {
                        result = result[prefix1.len()..].to_string();
                        break;
                    }
                    if result.starts_with(&prefix2) {
                        result = result[prefix2.len()..].to_string();
                        break;
                    }
                }
                result
            })
            .take(count)
            .collect();

        Ok(points)
    }

    /// Answer a question about content
    pub fn answer_question(&self, content: &str, question: &str) -> Result<String> {
        let truncated = if content.len() > 4000 {
            format!("{}...", &content[..4000])
        } else {
            content.to_string()
        };

        let system_prompt = r#"You are a helpful assistant that answers questions based on provided content.
Answer the question using only information from the given content.
If the answer cannot be found in the content, say so clearly.
Keep your answer concise and direct."#;

        let user_prompt = format!("Content:\n{}\n\nQuestion: {}", truncated, question);

        self.provider.complete_with_system(system_prompt, &user_prompt)
    }

    /// Check if summarization is available
    pub fn is_available(&self) -> bool {
        self.provider.is_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fallback_models() {
        let models = default_fallback_models();
        assert!(!models.is_empty());
        assert!(models.iter().all(|m| m.contains(":free")));
    }

    #[test]
    fn test_summarizer_from_env_without_key() {
        // Clear the env var for this test
        std::env::remove_var("OPENROUTER_API_KEY");
        let summarizer = Summarizer::from_env();
        assert!(summarizer.is_none());
    }
}
