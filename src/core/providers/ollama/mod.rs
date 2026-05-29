use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
    top_k: u32,
    top_p: f32,
    stop: Vec<String>,
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
    pub base_url: String,
    pub model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn ping(&self) -> bool {
        self.client
            .get(&format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn generate_raw(&self, system: &str, prompt: &str) -> Result<String> {
        let is_json = system.contains("JSON") || system.contains("command") || system.contains("FORMAT RESPONS");
        let format = if is_json { Some("json".to_string()) } else { None };

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            system: system.to_string(),
            stream: false,
            format,
            options: OllamaOptions {
                temperature: 0.1,
                num_predict: 2048,
                top_k: 10,
                top_p: 0.9,
                stop: vec![],
            },
        };

        let res = self
            .client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Gagal menghubungi Ollama. Apakah Ollama sudah dijalankan?")?;

        let status = res.status();
        let text = res.text().await.context("Gagal membaca body respons dari Ollama")?;

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
