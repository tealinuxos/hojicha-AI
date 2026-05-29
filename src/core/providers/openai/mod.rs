use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::core::OpenAiConfig;

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: String,
}

pub struct OpenAiClient {
    client: Client,
    pub config: OpenAiConfig,
}

impl OpenAiClient {
    pub fn new(config: OpenAiConfig) -> Self {
        let timeout_secs = config.timeout.unwrap_or(30);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client for OpenAI");

        Self { client, config }
    }

    pub async fn generate_raw(&self, system: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.api_key.as_ref()
            .context("API Key OpenAI tidak ditemukan. Silakan atur lewat menu /model")?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1");

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let request = OpenAiRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        let mut req_builder = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request);

        if let Some(org) = &self.config.organization {
            req_builder = req_builder.header("OpenAI-Organization", org);
        }
        if let Some(proj) = &self.config.project {
            req_builder = req_builder.header("OpenAI-Project", proj);
        }

        let res = req_builder.send().await
            .context("Gagal menghubungi server OpenAI")?;

        let status = res.status();
        let text = res.text().await.context("Gagal membaca respons dari OpenAI")?;

        if !status.is_success() {
            anyhow::bail!("OpenAI API error (status {}): {}", status, text);
        }

        let response: OpenAiResponse = serde_json::from_str(&text)
            .with_context(|| format!("Gagal mengurai respons JSON OpenAI: {}", text))?;

        let content = response.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content.trim().to_string())
    }
}
