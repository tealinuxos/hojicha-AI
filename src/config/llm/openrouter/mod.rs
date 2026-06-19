use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use super::openai::OpenAiConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenRouterConfig {
    #[serde(flatten)]
    inner: OpenAiConfig,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        let mut inner = OpenAiConfig::default();
        inner.base_url = Some("https://openrouter.ai/api/v1".to_string());
        inner.model = "openai/gpt-4o-mini".to_string();
        Self { inner }
    }
}

impl From<OpenRouterConfig> for OpenAiConfig {
    fn from(config: OpenRouterConfig) -> Self {
        config.inner
    }
}

impl Deref for OpenRouterConfig {
    type Target = OpenAiConfig;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for OpenRouterConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
