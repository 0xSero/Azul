use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub ai: Option<AIConfig>,
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exa_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_border")]
    pub border: String,
    #[serde(default = "default_text")]
    pub text: String,
    #[serde(default = "default_bg")]
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn default_accent() -> String {
    "#9f8a60".to_string() // Warm tan (Warm Paper theme)
}

fn default_border() -> String {
    "#33312f".to_string() // Charcoal border (Warm Paper theme)
}

fn default_text() -> String {
    "#f0ece0".to_string() // Warm cream (Warm Paper theme)
}

fn default_bg() -> String {
    "#1b1b1b".to_string() // Rich charcoal (Warm Paper theme)
}

fn default_refresh_rate() -> u64 {
    200
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: default_accent(),
            border: default_border(),
            text: default_text(),
            background: default_bg(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            ai: None,
            refresh_rate_ms: default_refresh_rate(),
            rag_base_url: None,
            memory_scope: None,
            exa_api_key: None,
        }
    }
}

impl Config {
    /// Get the config file path (JSON format, compatible with Go version)
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("azul")
            .join("config.json")
    }

    /// Load config from file and environment variables
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        // Start with default or load from file
        let mut config = if path.exists() {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config from {}", path.display()))?;

            serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse config from {}", path.display()))?
        } else {
            Self::default()
        };

        // Initialize AI config if not present
        if config.ai.is_none() {
            config.ai = Some(AIConfig {
                provider: None,
                api_key: None,
                model: None,
                models: None,
                fallback_models: None,
                base_url: None,
            });
        }

        let ai = config.ai.as_mut().unwrap();
        let mut changed = false;

        // Environment variables override file config (compatible with Go version)

        // Try OPENROUTER_API_KEY first, then AZUL_AI_API_KEY
        if let Some(key) = lookup_env_non_empty(&["OPENROUTER_API_KEY", "AZUL_AI_API_KEY"]) {
            if ai.api_key.as_ref() != Some(&key) {
                ai.api_key = Some(key);
                changed = true;
            }
        }

        // Provider
        if let Some(provider) = std::env::var("AZUL_AI_PROVIDER").ok() {
            if !provider.is_empty() && ai.provider.as_ref() != Some(&provider) {
                ai.provider = Some(provider);
                changed = true;
            }
        }

        // Model (OPENROUTER_MODEL or AZUL_AI_MODEL)
        if let Some(model) = lookup_env_non_empty(&["OPENROUTER_MODEL", "AZUL_AI_MODEL"]) {
            if ai.model.as_ref() != Some(&model) {
                ai.model = Some(model);
                changed = true;
            }
        }

        // Models list
        if let Some(models_env) = std::env::var("AZUL_AI_MODELS").ok() {
            let models = split_list(&models_env);
            if !models.is_empty() {
                ai.models = Some(models);
                changed = true;
            }
        }

        // Fallback models
        if let Some(fallback_env) = std::env::var("AZUL_AI_FALLBACK_MODELS").ok() {
            let models = split_list(&fallback_env);
            if !models.is_empty() {
                ai.fallback_models = Some(models);
                changed = true;
            }
        }

        // Set defaults when API key is present but provider/model are not
        if ai.api_key.is_some() {
            if ai.provider.is_none() {
                ai.provider = Some("openrouter".to_string());
                changed = true;
            }

            if ai.provider.as_deref() == Some("openrouter") && ai.model.is_none() {
                ai.model = Some("qwen/qwen3-235b-a22b:free".to_string());
                changed = true;
            }

            // Set default fallback models if not specified
            if ai.fallback_models.is_none() {
                ai.fallback_models = Some(vec![
                    "x-ai/grok-4.1-fast:free".to_string(),
                    "kwaipilot/kat-coder-pro:free".to_string(),
                    "z-ai/glm-4.5-air:free".to_string(),
                    "moonshotai/kimi-k2:free".to_string(),
                ]);
                changed = true;
            }
        }

        // Save config if changed and we have AI settings
        if changed && ai.api_key.is_some() && ai.provider.is_some() {
            let _ = config.save();
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;

        Ok(())
    }

    /// Get the API key for AI features
    pub fn get_api_key(&self) -> Option<&str> {
        self.ai.as_ref()?.api_key.as_deref()
    }

    /// Get the AI model to use
    pub fn get_ai_model(&self) -> Option<&str> {
        self.ai.as_ref()?.model.as_deref()
    }

    /// Get fallback models
    pub fn get_fallback_models(&self) -> Option<Vec<String>> {
        self.ai.as_ref()?.fallback_models.clone()
    }

    /// Get the AI base URL
    pub fn get_ai_base_url(&self) -> Option<&str> {
        self.ai.as_ref()?.base_url.as_deref()
    }

    /// Get the Exa API key (from config or environment)
    pub fn get_exa_api_key(&self) -> Option<String> {
        // Config takes precedence, then env var
        self.exa_api_key.clone()
            .or_else(|| std::env::var("EXA_API_KEY").ok())
    }
}

/// Look up environment variable from multiple possible keys
fn lookup_env_non_empty(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

/// Split comma/space delimited list into trimmed entries
fn split_list(value: &str) -> Vec<String> {
    value
        .split(|c| c == ',' || c == ' ' || c == '\n' || c == '\t')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}
