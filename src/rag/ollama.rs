use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::OllamaConfig;

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: usize,
    top_k: usize,
    top_p: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat_penalty: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
    thinking: Option<String>,
}

pub struct OllamaClient {
    client: Client,
    pub config: OllamaConfig,
}

impl OllamaClient {
    pub fn new(config: OllamaConfig) -> Self {
        let timeout_secs = config.timeout.unwrap_or(60);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client for Ollama");

        Self { client, config }
    }

    pub async fn ping(&self) -> bool {
        self.client
            .get(&format!(
                "{}/api/tags",
                self.config.base_url.trim_end_matches('/')
            ))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn generate_raw(&self, system: &str, prompt: &str) -> Result<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            system: system.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
                top_k: self.config.top_k.unwrap_or(10),
                top_p: self.config.top_p.unwrap_or(0.9),
                num_ctx: self.config.num_ctx,
                repeat_penalty: self.config.repeat_penalty,
            },
        };

        let url = format!(
            "{}/api/generate",
            self.config.base_url.trim_end_matches('/')
        );

        let res = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Gagal menghubungi Ollama. Pastikan Ollama sudah berjalan.")?;

        let status = res.status();
        let text = res
            .text()
            .await
            .context("Gagal membaca body respons dari Ollama")?;

        if !status.is_success() {
            anyhow::bail!("Ollama error (status {}): {}", status, text);
        }

        let response: OllamaResponse = serde_json::from_str(&text)
            .with_context(|| format!("Gagal mem-parsing JSON Ollama. Raw response: {}", text))?;

        let content = if response.response.trim().is_empty() {
            response.thinking.unwrap_or_default()
        } else {
            response.response
        };

        Ok(content.trim().to_string())
    }
}
