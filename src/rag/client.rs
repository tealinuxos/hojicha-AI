use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::rag::local::LocalModelClient;
use crate::rag::ollama::OllamaClient;
use crate::rag::utils::extract_json;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CommandResponse {
    pub command: Option<String>,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub beginner_tip: Option<String>,
    #[serde(default = "default_safe")]
    pub is_safe: bool,
}

fn default_safe() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OutputSummary {
    pub summary: String,
    pub key_info: serde_json::Value,
    pub next_suggestion: Option<String>,
}

pub enum AiClient {
    Ollama(OllamaClient),
    Local(LocalModelClient),
}

impl AiClient {
    /// Generate a command from natural language input
    pub async fn nl_to_command(
        &mut self,
        system_prompt: &str,
        user_input: &str,
    ) -> Result<CommandResponse> {
        let raw = match self {
            Self::Ollama(ollama) => ollama.generate_raw(system_prompt, user_input).await?,
            Self::Local(local) => {
                let sys = system_prompt.to_string();
                let usr = user_input.to_string();
                local.generate_raw(&sys, &usr)?
            }
        };

        let json_str = extract_json(&raw);
        match serde_json::from_str::<CommandResponse>(&json_str) {
            Ok(resp) => Ok(resp),
            Err(_) => {
                // Fail-safe fallback: If JSON parsing fails, extract a command from the raw text
                let clean_cmd = crate::rag::utils::clean_raw_command(&raw);
                Ok(CommandResponse {
                    command: Some(clean_cmd),
                    explanation: String::new(),
                    beginner_tip: None,
                    is_safe: true,
                })
            }
        }
    }

    /// Summarize command output in beginner-friendly language
    pub async fn summarize_output(
        &mut self,
        system_prompt: &str,
        summary_prompt: &str,
    ) -> Result<OutputSummary> {
        let raw = match self {
            Self::Ollama(ollama) => ollama.generate_raw(system_prompt, summary_prompt).await?,
            Self::Local(local) => {
                let sys = system_prompt.to_string();
                let sum = summary_prompt.to_string();
                local.generate_raw(&sys, &sum)?
            }
        };

        let json_str = extract_json(&raw);
        serde_json::from_str::<OutputSummary>(&json_str).map_err(|e| {
            anyhow::anyhow!("Gagal mengurai ringkasan output. Err: {}. Raw:\n{}", e, &raw[..raw.len().min(300)])
        })
    }
}
