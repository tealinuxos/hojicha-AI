use serde::{Deserialize, Serialize};

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
            top_p: Some(0.9),
            top_k: Some(10),
            num_ctx: Some(8192),
            repeat_penalty: None,
            timeout: Some(60),
            max_retries: Some(3),
        }
    }
}
