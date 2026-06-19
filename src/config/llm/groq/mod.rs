use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use super::openai::OpenAiConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroqConfig {
    #[serde(flatten)]
    inner: OpenAiConfig,
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            inner: OpenAiConfig {
                base_url: Some("https://api.groq.com/openai/v1".to_string()),
                model: "llama-3.1-8b-instant".to_string(),
                ..Default::default()
            },
        }
    }
}

impl From<GroqConfig> for OpenAiConfig {
    fn from(config: GroqConfig) -> Self {
        config.inner
    }
}

impl Deref for GroqConfig {
    type Target = OpenAiConfig;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GroqConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
