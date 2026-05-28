use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NativeConfig {
    pub repo_id: String,
    pub filename: String,
    pub tokenizer_repo: String,
    pub tokenizer_filename: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,
    
    #[serde(default)]
    pub repeat_penalty: Option<f32>,
    #[serde(default)]
    pub repeat_last_n: Option<usize>,
    #[serde(default)]
    pub cpu_threads: Option<usize>,
}

impl Default for NativeConfig {
    fn default() -> Self {
        Self {
            repo_id: "bartowski/SmolLM2-135M-Instruct-GGUF".to_string(),
            filename: "SmolLM2-135M-Instruct-Q4_K_M.gguf".to_string(),
            tokenizer_repo: "HuggingFaceTB/SmolLM2-135M-Instruct".to_string(),
            tokenizer_filename: "tokenizer.json".to_string(),
            temperature: 0.3,
            top_p: 0.95,
            max_tokens: 800,
            repeat_penalty: Some(1.1),
            repeat_last_n: Some(64),
            cpu_threads: None,
        }
    }
}
