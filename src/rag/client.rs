use crate::rag::gemini::GeminiClient;
use crate::rag::local::LocalModelClient;
use crate::rag::ollama::OllamaClient;
use crate::rag::openai::OpenAiClient;
use crate::rag::utils::extract_json;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CommandResponse {
    pub command: Option<String>,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub beginner_tip: Option<String>,
    // FIXED: Fail-closed default. If the LLM omits `is_safe`, we assume unsafe
    // rather than blindly trusting it. Previously defaulted to `true` which
    // meant a malformed LLM response would bypass safety checks.
    #[serde(default = "default_safe")]
    pub is_safe: bool,
}

fn default_safe() -> bool {
    false
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OutputSummary {
    pub summary: String,
    pub key_info: serde_json::Value,
    pub next_suggestion: Option<String>,
}

pub enum AiClient {
    Ollama(OllamaClient),
    OpenAi(OpenAiClient),
    Gemini(GeminiClient),
    OpenRouter(OpenAiClient),
    Groq(OpenAiClient),
    Local(Box<LocalModelClient>),
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
            Self::OpenAi(openai) => openai.generate_raw(system_prompt, user_input).await?,
            Self::Gemini(gemini) => gemini.generate_raw(system_prompt, user_input).await?,
            Self::OpenRouter(openrouter) => {
                openrouter.generate_raw(system_prompt, user_input).await?
            }
            Self::Groq(groq) => groq.generate_raw(system_prompt, user_input).await?,
            Self::Local(local) => local.generate_raw(system_prompt, user_input)?,
        };

        let json_str = extract_json(&raw);
        match serde_json::from_str::<CommandResponse>(&json_str) {
            Ok(resp) => Ok(resp),
            Err(_) => {
                let raw_trimmed = raw.trim();
                let is_conversational = !raw_trimmed.starts_with('{')
                    && (raw_trimmed.to_lowercase().starts_with("hello")
                        || raw_trimmed.to_lowercase().starts_with("hi")
                        || raw_trimmed.to_lowercase().starts_with("halo")
                        || raw_trimmed.to_lowercase().starts_with("hai")
                        || raw_trimmed.to_lowercase().starts_with("i'm")
                        || raw_trimmed.to_lowercase().starts_with("i am")
                        || raw_trimmed.to_lowercase().starts_with("sure")
                        || raw_trimmed.to_lowercase().starts_with("tentu")
                        || raw_trimmed.to_lowercase().starts_with("saya")
                        || raw_trimmed.to_lowercase().starts_with("kamu")
                        || raw_trimmed.to_lowercase().starts_with("maaf")
                        || raw_trimmed.to_lowercase().starts_with("tidak")
                        || raw_trimmed.contains("help you")
                        || raw_trimmed.contains("bantu"));

                if is_conversational {
                    Ok(CommandResponse {
                        command: None,
                        explanation: raw_trimmed.to_string(),
                        beginner_tip: None,
                        is_safe: true,
                    })
                } else {
                    let clean_cmd = crate::rag::utils::clean_raw_command(&raw);
                    if clean_cmd.is_empty() || clean_cmd.split_whitespace().count() > 10 {
                        Ok(CommandResponse {
                            command: None,
                            explanation: raw_trimmed.to_string(),
                            beginner_tip: None,
                            is_safe: true,
                        })
                    } else {
                        // SECURITY: Raw command fallback — is_safe is set to true because
                        // the authoritative safety check is in the safety module (check_safety),
                        // which ALWAYS runs before execution. The is_safe field is a secondary
                        // check from the LLM's judgment, which is unavailable when JSON
                        // parsing fails. The safety module provides the real security gate.
                        Ok(CommandResponse {
                            command: Some(clean_cmd),
                            explanation: String::new(),
                            beginner_tip: None,
                            is_safe: true,
                        })
                    }
                }
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
            Self::OpenAi(openai) => openai.generate_raw(system_prompt, summary_prompt).await?,
            Self::Gemini(gemini) => gemini.generate_raw(system_prompt, summary_prompt).await?,
            Self::OpenRouter(openrouter) => {
                openrouter
                    .generate_raw(system_prompt, summary_prompt)
                    .await?
            }
            Self::Groq(groq) => groq.generate_raw(system_prompt, summary_prompt).await?,
            Self::Local(local) => local.generate_raw(system_prompt, summary_prompt)?,
        };

        let json_str = extract_json(&raw);
        serde_json::from_str::<OutputSummary>(&json_str).map_err(|e| {
            anyhow::anyhow!(
                "Gagal mengurai ringkasan output. Err: {}. Raw:\n{}",
                e,
                &raw[..raw.len().min(300)]
            )
        })
    }
}
