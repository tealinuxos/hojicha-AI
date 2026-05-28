pub mod native;
pub mod ollama;
pub mod openai;
pub mod gemini;
pub mod anthropic;
pub mod wizard;
pub mod providers;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use self::native::NativeConfig;
use self::ollama::OllamaConfig;
use self::openai::OpenAiConfig;
use self::gemini::GeminiConfig;
use self::anthropic::AnthropicConfig;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmType {
    #[serde(alias = "local")]
    Native,
    Api,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiProvider {
    Ollama,
    Openai,
    Gemini,
    Anthropic,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmConfig {
    pub active: LlmType,
    pub active_api_provider: ApiProvider,
    #[serde(alias = "local")]
    pub native: NativeConfig,
    pub ollama: OllamaConfig,
    pub openai: OpenAiConfig,
    pub gemini: GeminiConfig,
    pub anthropic: AnthropicConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            active: LlmType::Api,
            active_api_provider: ApiProvider::Ollama,
            native: NativeConfig::default(),
            ollama: OllamaConfig::default(),
            openai: OpenAiConfig::default(),
            gemini: GeminiConfig::default(),
            anthropic: AnthropicConfig::default(),
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

        let content = fs::read_to_string(&path)
            .context("Gagal membaca file konfigurasi LLM")?;
        let config: LlmConfig = serde_json::from_str(&content)
            .context("Gagal mengurai file konfigurasi LLM (JSON tidak valid)")?;
        Ok(config)
    }

    /// Save the current configuration to the config file
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Gagal membuat direktori konfigurasi")?;
        }
        let content = serde_json::to_string_pretty(self)
            .context("Gagal mengonversi konfigurasi LLM ke JSON")?;
        fs::write(&path, content)
            .context("Gagal menulis file konfigurasi LLM")?;
        Ok(())
    }
}
