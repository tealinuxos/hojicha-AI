use serde::{Deserialize, Serialize};

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
