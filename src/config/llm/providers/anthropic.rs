use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::config::AnthropicConfig;

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    system: String,
    messages: Vec<Message>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentResponse>,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    text: String,
}

pub struct AnthropicClient {
    client: Client,
    pub config: AnthropicConfig,
}

impl AnthropicClient {
    pub fn new(config: AnthropicConfig) -> Self {
        let timeout_secs = config.timeout.unwrap_or(30);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client for Anthropic");

        Self { client, config }
    }

    pub async fn generate_raw(&self, system: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.api_key.as_ref()
            .context("API Key Anthropic tidak ditemukan. Silakan atur lewat menu /model")?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.anthropic.com");

        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            system: system.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.config.temperature,
            top_p: self.config.top_p,
            top_k: self.config.top_k,
        };

        let res = self.client.post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Gagal menghubungi server Anthropic")?;

        let status = res.status();
        let text = res.text().await.context("Gagal membaca respons dari Anthropic")?;

        if !status.is_success() {
            anyhow::bail!("Anthropic API error (status {}): {}", status, text);
        }

        let response: AnthropicResponse = serde_json::from_str(&text)
            .with_context(|| format!("Gagal mengurai respons JSON Anthropic: {}", text))?;

        let content = response.content.first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(content.trim().to_string())
    }
}
