pub mod gemini;
pub mod groq;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod wizard;

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::PathBuf;
use colored::Colorize;

use self::gemini::GeminiConfig;
use self::groq::GroqConfig;
use self::ollama::OllamaConfig;
use self::openai::OpenAiConfig;
use self::openrouter::OpenRouterConfig;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiProvider {
    Native,
    #[default]
    Ollama,
    Openai,
    Gemini,
    Openrouter,
    Groq,
}

impl ApiProvider {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Native => "Native",
            Self::Ollama => "Ollama",
            Self::Openai => "OpenAI",
            Self::Gemini => "Gemini",
            Self::Openrouter => "OpenRouter",
            Self::Groq => "Groq",
        }
    }
}

impl<'de> Deserialize<'de> for ApiProvider {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Option::<String>::deserialize(deserializer)?.unwrap_or_default();
        let normalized = raw
            .trim()
            .chars()
            .filter(|c| *c != '_' && *c != '-')
            .collect::<String>()
            .to_ascii_lowercase();

        Ok(match normalized.as_str() {
            "native" => Self::Native,
            "ollama" => Self::Ollama,
            "openai" => Self::Openai,
            "gemini" => Self::Gemini,
            "openrouter" => Self::Openrouter,
            "groq" => Self::Groq,
            _ => Self::Ollama,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmConfig {
    #[serde(default)]
    pub active_api_provider: ApiProvider,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
    #[serde(default)]
    pub gemini: GeminiConfig,
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
    #[serde(default)]
    pub groq: GroqConfig,
    #[serde(default)]
    pub theme: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            active_api_provider: ApiProvider::Ollama,
            ollama: OllamaConfig::default(),
            openai: OpenAiConfig::default(),
            gemini: GeminiConfig::default(),
            openrouter: OpenRouterConfig::default(),
            groq: GroqConfig::default(),
            theme: Some("dark".to_string()),
        }
    }
}

impl LlmConfig {
    /// Get the default path for the configuration file: ~/.config/hojicha/config.json
    pub fn default_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("Variable lingkungan HOME tidak ditemukan")?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("hojicha")
            .join("config.json"))
    }

    /// Load config from file, or generate a default one if it doesn't exist
    pub fn load_or_create() -> Result<Self> {
        let path = Self::default_path()?;
        if !path.exists() {
            let default_config = Self::default();
            // Try saving, but don't error out completely if directory creation fails (e.g. read-only env)
            let _ = default_config.save();
            return Ok(default_config);
        }

        let content = fs::read_to_string(&path).context("Gagal membaca file konfigurasi LLM")?;
        let config: LlmConfig = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("{}", "⚠️  Peringatan: File konfigurasi LLM rusak atau tidak kompatibel. Mengatur ulang ke default...".yellow());
                let default_config = Self::default();
                let _ = default_config.save();
                default_config
            }
        };
        Ok(config)
    }

    /// Save the current configuration to the config file.
    /// Sets file permissions to 600 (owner read/write only) since the config
    /// contains sensitive API keys.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Gagal membuat direktori konfigurasi")?;
        }
        let content = serde_json::to_string_pretty(self)
            .context("Gagal mengonversi konfigurasi LLM ke JSON")?;
        fs::write(&path, content).context("Gagal menulis file konfigurasi LLM")?;

        // SECURITY: Set restrictive file permissions (owner read/write only)
        // since the config contains API keys. Without this, other users on
        // shared systems could read the keys.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms)
                .context("Gagal mengatur permission file konfigurasi")?;
        }

        Ok(())
    }
}
