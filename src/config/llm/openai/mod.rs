use serde::{Deserialize, Serialize};

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
