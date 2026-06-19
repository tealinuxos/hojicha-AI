use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::config::GeminiConfig;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    system_instruction: SystemInstruction,
    contents: Vec<ContentItem>,
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct ContentItem {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(default)]
    content: ContentResponse,
}

#[derive(Debug, Deserialize, Default)]
struct ContentResponse {
    #[serde(default)]
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}

pub struct GeminiClient {
    client: Client,
    pub config: GeminiConfig,
}

impl GeminiClient {
    pub fn new(config: GeminiConfig) -> Self {
        let timeout_secs = config.timeout.unwrap_or(30);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client for Gemini");

        Self { client, config }
    }

    pub async fn generate_raw(&self, system: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.api_key.as_ref()
            .context("API Key Gemini tidak ditemukan. Silakan atur lewat menu /model")?;

        // Standard url, or support custom baseUrl
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com");

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            base_url.trim_end_matches('/'),
            self.config.model,
            api_key
        );

        let request = GeminiRequest {
            system_instruction: SystemInstruction {
                parts: vec![Part { text: system.to_string() }],
            },
            contents: vec![ContentItem {
                role: "user".to_string(),
                parts: vec![Part { text: prompt.to_string() }],
            }],
            generation_config: GenerationConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
                top_p: self.config.top_p,
                top_k: self.config.top_k,
            },
        };

        let res = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Gagal menghubungi server Gemini")?;

        let status = res.status();
        let text = res.text().await.context("Gagal membaca respons dari Gemini")?;

        if !status.is_success() {
            anyhow::bail!("Gemini API error (status {}): {}", status, text);
        }

        let response: GeminiResponse = serde_json::from_str(&text)
            .with_context(|| format!("Gagal mengurai respons JSON Gemini: {}", text))?;

        let content = response.candidates.first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(content.trim().to_string())
    }
}
