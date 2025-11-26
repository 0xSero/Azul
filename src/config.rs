use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub ai: AIConfig,
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
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
    pub api_key: Option<String>,
    #[serde(default = "default_provider")]
    pub provider: String,
    pub fallback_models: Option<Vec<String>>,
}

fn default_accent() -> String {
    "#00BFFF".to_string() // Deep sky blue (Azul theme)
}

fn default_border() -> String {
    "#7aa2f7".to_string() // Tokyo Night blue
}

fn default_text() -> String {
    "#c0caf5".to_string() // Tokyo Night foreground
}

fn default_bg() -> String {
    "#1a1b26".to_string() // Tokyo Night background
}

fn default_provider() -> String {
    "openrouter".to_string()
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

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            provider: default_provider(),
            fallback_models: Some(vec![
                "x-ai/grok-4.1-fast:free".to_string(),
                "kwaipilot/kat-coder-pro:free".to_string(),
                "z-ai/glm-4.5-air:free".to_string(),
                "moonshotai/kimi-k2:free".to_string(),
            ]),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            ai: AIConfig::default(),
            refresh_rate_ms: default_refresh_rate(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("azul")
            .join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;

        Ok(())
    }
}
