use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ─── Provider Config Structs ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NativeConfig {
    pub repo_id: String,
    pub filename: String,
    pub tokenizer_repo: String,
    pub tokenizer_filename: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,

    #[serde(default)]
    pub repeat_penalty: Option<f32>,
    #[serde(default)]
    pub repeat_last_n: Option<usize>,
    #[serde(default)]
    pub cpu_threads: Option<usize>,
}

impl Default for NativeConfig {
    fn default() -> Self {
        Self {
            repo_id: "bartowski/SmolLM2-135M-Instruct-GGUF".to_string(),
            filename: "SmolLM2-135M-Instruct-Q4_K_M.gguf".to_string(),
            tokenizer_repo: "HuggingFaceTB/SmolLM2-135M-Instruct".to_string(),
            tokenizer_filename: "tokenizer.json".to_string(),
            temperature: 0.3,
            top_p: 0.95,
            max_tokens: 800,
            repeat_penalty: Some(1.1),
            repeat_last_n: Some(64),
            cpu_threads: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,

    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub num_ctx: Option<usize>,
    #[serde(default)]
    pub repeat_penalty: Option<f32>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub max_retries: Option<usize>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "qwen2.5:1.5b".to_string(),
            temperature: 0.1,
            max_tokens: 2048,
            top_p: None,
            top_k: None,
            num_ctx: Some(8192),
            repeat_penalty: None,
            timeout: Some(30),
            max_retries: Some(3),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAiConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub organization: Option<String>,

    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub max_retries: Option<usize>,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.2,
            max_tokens: 2048,
            organization: None,
            project: None,
            top_p: None,
            presence_penalty: None,
            frequency_penalty: None,
            timeout: Some(30),
            max_retries: Some(3),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,

    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub max_retries: Option<usize>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gemini-1.5-flash".to_string(),
            temperature: 0.2,
            max_tokens: 2048,
            base_url: None,
            top_p: None,
            top_k: None,
            timeout: Some(30),
            max_retries: Some(3),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,

    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub max_retries: Option<usize>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "claude-3-5-sonnet-latest".to_string(),
            temperature: 0.2,
            max_tokens: 2048,
            base_url: None,
            top_p: None,
            top_k: None,
            timeout: Some(30),
            max_retries: Some(3),
        }
    }
}

// ─── LLM Type & Provider Enums ────────────────────────────────────────────────

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

// ─── Main LLM Config ──────────────────────────────────────────────────────────

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
