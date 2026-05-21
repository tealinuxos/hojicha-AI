/// AI module: Supports both local Ollama API and built-in offline Candle inference.
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

// ─── Candle Local Inference Imports ──────────────────────────────────────────
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_llama::ModelWeights;
use tokenizers::Tokenizer;

// ─── Ollama API types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
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

// ─── Common response types ──────────────────────────────────────────────────

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

// ─── Client variant ──────────────────────────────────────────────────────────

pub enum AiClient {
    Ollama(OllamaClient),
    Local(LocalModelClient),
}

// ─── Ollama client implementation ────────────────────────────────────────────

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
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            system: system.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: 0.1,
                num_predict: 2048,
                top_k: 10,
                top_p: 0.9,
                stop: vec![],
            },
        };

        let response: OllamaResponse = self
            .client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Gagal menghubungi Ollama. Apakah Ollama sudah dijalankan?")?
            .json()
            .await
            .context("Gagal membaca respons dari Ollama")?;

        let content = if response.response.trim().is_empty() {
            response.thinking.unwrap_or_default()
        } else {
            response.response
        };

        Ok(content.trim().to_string())
    }
}

// ─── Local Model Client Implementation (Candle) ─────────────────────────────

pub struct LocalModelClient {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
}

impl LocalModelClient {
    /// Initialize local model, downloading from Hugging Face if not present
    pub fn load_built_in() -> Result<Self> {
        // We run the model check and download on the calling thread (blocked)
        // using standard hf-hub cache.
        let (model_path, tokenizer_path) = download_built_in_model()?;

        let device = Device::Cpu; // CPU execution is standard and dependency-free

        let mut file = std::fs::File::open(&model_path)
            .context("Gagal membuka file model GGUF")?;

        let content = gguf_file::Content::read(&mut file)
            .context("Gagal membaca metadata GGUF")?;

        let model = ModelWeights::from_gguf(content, &mut file, &device)
            .context("Gagal memuat bobot model GGUF ke memory")?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Gagal memuat tokenizer: {}", e))?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub fn generate_raw(&mut self, system: &str, prompt: &str) -> Result<String> {
        // Clone the model template to get a fresh model state with a clean KV cache.
        // Tensors in Candle are cheaply cloned (shallow reference count copy), so this has negligible overhead.
        let mut active_model = self.model.clone();

        let is_cmd_gen = system.contains("FORMAT RESPONS WAJIB");

        // Format prompt for SmolLM2 chat template.
        // For command generation, we construct a structured multi-turn conversation
        // that dynamically selects the single most relevant few-shot example.
        // This keeps the context extremely short and focused, which prevents the 135M model from losing attention.
        let formatted_prompt = if is_cmd_gen {
            let p_lower = prompt.to_lowercase();
            let example = if p_lower.contains("ram") || p_lower.contains("memori") || p_lower.contains("memory") || p_lower.contains("konsumsi") {
                ("berapa konsumsi ram saya", "free -h")
            } else if p_lower.contains("koneksi") || p_lower.contains("internet") || p_lower.contains("ping") || p_lower.contains("konek") {
                ("cek koneksi internet", "ping -c 4 google.com")
            } else {
                ("lihat file di folder ini", "ls -la")
            };

            format!(
                "<|im_start|>system\nYou are TERA, a Linux assistant. Translate user intent to a Linux command. Output ONLY the raw command. Do not explain. Do not use markdown.<|im_end|>\n\
                 <|im_start|>user\n{}<|im_end|>\n\
                 <|im_start|>assistant\n{}<|im_end|>\n\
                 <|im_start|>user\n{}<|im_end|>\n\
                 <|im_start|>assistant\n",
                example.0, example.1, prompt
            )
        } else if prompt.contains("Jelaskan output di atas") || prompt.contains("Ringkasan") {
            let simplified_system = "You are TERA, a Linux assistant. Summarize the terminal output.\n\n\
                 Example:\n\
                 Assistant: {\"summary\": \"Perintah berhasil dijalankan dan menampilkan daftar file.\", \"key_info\": \"Ada 5 file di direktori saat ini.\", \"next_suggestion\": \"Ketik pwd untuk melihat posisi folder Anda saat ini.\"}\n\n\
                 Respond ONLY with a JSON object in this format:\n\
                 {\"summary\": \"summary in Indonesian\", \"key_info\": \"key info in Indonesian\", \"next_suggestion\": \"suggestion in Indonesian or null\"}";
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{{",
                simplified_system, prompt
            )
        } else {
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{{",
                system, prompt
            )
        };

        let tokens = self.tokenizer.encode(formatted_prompt.clone(), true)
            .map_err(|e| anyhow::anyhow!("Encoding error: {}", e))?;
        let prompt_tokens = tokens.get_ids();

        // println!("DEBUG: Tokenized prompt: {:?}", token_names);

        let mut all_tokens = prompt_tokens.to_vec();
        let mut next_token = 0;
        let mut pos = 0;

        let mut logits_processor = LogitsProcessor::from_sampling(
            299792458, // seed
            Sampling::ArgMax,
        );

        let mut generated_tokens = vec![];

        // Process prompt tokens
        if !prompt_tokens.is_empty() {
            let input = Tensor::new(prompt_tokens, &self.device)?.unsqueeze(0)?;
            let logits = active_model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?;
            next_token = logits_processor.sample(&logits)?;
            pos += prompt_tokens.len();
            all_tokens.push(next_token);
            generated_tokens.push(next_token);
        }

        let eos_token_id = self.tokenizer.token_to_id("<|im_end|>")
            .or_else(|| self.tokenizer.token_to_id("<|endoftext|>"))
            .unwrap_or(0);

        // Generate response loop
        let max_new_tokens = 800;

        for _ in 0..max_new_tokens {
            if next_token == eos_token_id {
                break;
            }

            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = active_model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?;

            // Apply repeat penalty to generated tokens only (avoid penalizing few-shot prompt tokens)
            let logits = if generated_tokens.is_empty() {
                logits
            } else {
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    1.1, // penalty
                    &generated_tokens[..],
                )?
            };

            next_token = logits_processor.sample(&logits)?;
            pos += 1;
            all_tokens.push(next_token);
            generated_tokens.push(next_token);
        }

        let output = self.tokenizer.decode(&generated_tokens, true)
            .map_err(|e| anyhow::anyhow!("Decoding error: {}", e))?;

        // Reconstruct the response output
        let final_output = if is_cmd_gen {
            output.trim().to_string()
        } else {
            format!("{{{}", output.trim())
        };
        Ok(final_output)
    }
}

// ─── Auto-downloader for built-in model ─────────────────────────────────────

fn download_built_in_model() -> Result<(PathBuf, PathBuf)> {
    use hf_hub::api::sync::Api;
    use hf_hub::{Repo, RepoType};

    println!("{}", "📥 Model AI lokal built-in tidak ditemukan.".yellow_or_colored());
    println!("{}", "   Mengunduh SmolLM2-135M (105MB) dari Hugging Face Hub...".yellow_or_colored());
    println!("{}", "   (Proses ini hanya sekali, selanjutnya akan berjalan 100% offline)".dimmed_colored());
    println!();

    let api = Api::new().context("Gagal menginisialisasi Hugging Face API client")?;
    
    // Get tokenizer from main repository
    let repo_info = api.model("HuggingFaceTB/SmolLM2-135M-Instruct".to_string());
    let tokenizer_path = repo_info.get("tokenizer.json")
        .context("Gagal mengunduh tokenizer.json")?;

    // Get model file from GGUF repository
    let model_repo = api.repo(Repo::new(
        "bartowski/SmolLM2-135M-Instruct-GGUF".to_string(),
        RepoType::Model,
    ));
    let model_path = model_repo.get("SmolLM2-135M-Instruct-Q4_K_M.gguf")
        .context("Gagal mengunduh SmolLM2-135M-Instruct-Q4_K_M.gguf")?;

    println!("{}", "✅ Model dan tokenizer berhasil diunduh!".green_or_colored());
    println!();

    Ok((model_path, tokenizer_path))
}

// ─── Helper extension traits for console style ──────────────────────────────

trait ColorExt {
    fn green_or_colored(&self) -> String;
    fn yellow_or_colored(&self) -> String;
    fn dimmed_colored(&self) -> String;
}

impl ColorExt for str {
    fn green_or_colored(&self) -> String {
        use colored::Colorize;
        self.truecolor(134, 239, 172).bold().to_string()
    }
    fn yellow_or_colored(&self) -> String {
        use colored::Colorize;
        self.truecolor(251, 191, 36).bold().to_string()
    }
    fn dimmed_colored(&self) -> String {
        use colored::Colorize;
        self.truecolor(160, 160, 160).to_string()
    }
}

// ─── Orchestrator Methods ────────────────────────────────────────────────────

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
                let res = local.generate_raw(&sys, &usr)?;
                // println!("DEBUG: Raw Local LLM Output: {}", res);
                res
            }
        };

        let json_str = extract_json(&raw);
        match serde_json::from_str::<CommandResponse>(&json_str) {
            Ok(resp) => Ok(resp),
            Err(_) => {
                // Fail-safe fallback: If JSON parsing fails, extract a command from the raw text
                let clean_cmd = clean_raw_command(&raw);
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
        serde_json::from_str::<OutputSummary>(&json_str).with_context(|| {
            format!("Gagal mengurai ringkasan output. Raw:\n{}", &raw[..raw.len().min(300)])
        })
    }
}

/// Extract the first JSON object from a string (handles markdown code blocks and DeepSeek thinking models)
fn extract_json(text: &str) -> String {
    // Strip <think>...</think> blocks (DeepSeek-R1, QwQ reasoning models)
    let stripped = if let Some(think_end) = text.find("</think>") {
        text[think_end + 8..].trim()
    } else {
        text.trim()
    };

    // Try to find JSON between ```json ... ``` or ``` ... ```
    let mut raw_json = if let Some(start) = stripped.find("```json") {
        if let Some(end) = stripped[start + 7..].find("```") {
            stripped[start + 7..start + 7 + end].trim().to_string()
        } else {
            stripped[start + 7..].trim().to_string()
        }
    } else if let Some(start) = stripped.find("```") {
        if let Some(end) = stripped[start + 3..].find("```") {
            stripped[start + 3..start + 3 + end].trim().to_string()
        } else {
            stripped[start + 3..].trim().to_string()
        }
    } else {
        stripped.to_string()
    };

    // Try to find raw JSON object {}
    if let Some(start) = raw_json.find('{') {
        if let Some(end) = raw_json.rfind('}') {
            if end > start {
                raw_json = raw_json[start..=end].to_string();
            }
        }
    }

    clean_json_newlines(&raw_json)
}

/// Helper to sanitize raw newlines inside double-quoted string values in a raw JSON string
fn clean_json_newlines(json_str: &str) -> String {
    let mut in_quotes = false;
    let mut escaped = false;
    let mut result = String::new();

    for c in json_str.chars() {
        if c == '\\' {
            escaped = !escaped;
            result.push(c);
        } else if c == '"' {
            if !escaped {
                in_quotes = !in_quotes;
            }
            escaped = false;
            result.push(c);
        } else {
            escaped = false;
            if c == '\n' || c == '\r' {
                if in_quotes {
                    result.push_str("\\n");
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }
    }
    result
}

/// Robust fallback helper to extract a clean Linux command from raw text
fn clean_raw_command(raw: &str) -> String {
    let trimmed = raw.trim();
    // If it contains "command": "...", extract the value
    if let Some(start) = trimmed.find("\"command\":") {
        let rest = &trimmed[start + 10..];
        if let Some(val_start) = rest.find('"') {
            let val_rest = &rest[val_start + 1..];
            if let Some(val_end) = val_rest.find('"') {
                return val_rest[..val_end].to_string();
            }
        }
    }
    // Otherwise, just return the first non-empty line
    for line in trimmed.lines() {
        let line_trimmed = line.trim();
        if !line_trimmed.is_empty() {
            let mut cleaned = line_trimmed.to_string();
            // Strip leading/trailing quotes if model wrapped command in quotes
            if cleaned.starts_with('"') && cleaned.ends_with('"') {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            if cleaned.starts_with('\'') && cleaned.ends_with('\'') {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            if cleaned.starts_with('`') && cleaned.ends_with('`') {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            return cleaned;
        }
    }
    trimmed.to_string()
}
