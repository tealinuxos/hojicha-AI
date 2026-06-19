/// Local Model Client: Built-in offline inference using Candle + Qwen2.5-0.5B-Instruct (GGUF).
/// Downloads model from Hugging Face Hub on first run, then works 100% offline.
use anyhow::{Context, Result};
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen2::ModelWeights;
use colored::Colorize;
use std::path::PathBuf;
use tokenizers::Tokenizer;

pub struct LocalModelClient {
    model_path: PathBuf,
    tokenizer: Tokenizer,
    device: Device,
}

// ChatML special tokens constructed via hex escapes to avoid shell/tool interpretation.
// These are the exact strings used by Qwen2.5-0.5B-Instruct's ChatML format.
fn chatml_sys_start() -> String {
    format!("{}system", "<\x7cim_start\x7c>")
}
fn chatml_user_start() -> String {
    format!("{}user", "<\x7cim_start\x7c>")
}
fn chatml_asst_start() -> String {
    format!("{}assistant", "<\x7cim_start\x7c>")
}
fn chatml_end() -> String {
    "<\x7cim_end\x7c>".to_string()
}

impl LocalModelClient {
    /// Initialize local model, downloading from Hugging Face if not present
    pub fn load_built_in() -> Result<Self> {
        // We run the model check and download on the calling thread (blocked)
        // using standard hf-hub cache.
        let (model_path, tokenizer_path) = download_built_in_model()?;
        let device = Device::Cpu; // CPU execution is standard and dependency-free

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Gagal memuat tokenizer: {}", e))?;

        Ok(Self {
            model_path,
            tokenizer,
            device,
        })
    }

    pub fn generate_raw(&mut self, system: &str, prompt: &str) -> Result<String> {
        // Load the model weights dynamically per-query to avoid cloning limitations
        // and reduce background memory usage to 0MB when idle.
        let mut file = std::fs::File::open(&self.model_path)
            .context("Gagal membuka file model GGUF")?;

        let content = gguf_file::Content::read(&mut file)
            .context("Gagal membaca metadata GGUF")?;

        let mut active_model = ModelWeights::from_gguf(content, &mut file, &self.device)
            .context("Gagal memuat bobot model GGUF ke memory")?;

        // Clean ChatML format — no few-shot examples.
        let end = chatml_end();
        let formatted_prompt = format!(
            "{}\n{}{}\n{}\n{}{}\n{}",
            chatml_sys_start(), system, end,
            chatml_user_start(), prompt, end,
            chatml_asst_start(),
        );

        let tokens = self.tokenizer.encode(formatted_prompt, true)
            .map_err(|e| anyhow::anyhow!("Encoding error: {}", e))?;
        let prompt_tokens = tokens.get_ids();

        let mut all_tokens = prompt_tokens.to_vec();
        let mut next_token: u32 = 0;
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

        let eos_token_id = self.tokenizer.token_to_id(&end)
            .or_else(|| self.tokenizer.token_to_id("<|im_end|>"))
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

            // Apply repeat penalty to generated tokens
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

        // Return the raw generated text — the client.rs extract_json handles parsing
        Ok(output.trim().to_string())
    }
}

// ─── Auto-downloader for built-in model ─────────────────────────────────────

fn download_built_in_model() -> Result<(PathBuf, PathBuf)> {
    use hf_hub::api::sync::ApiBuilder;
    use hf_hub::{Cache, Repo, RepoType};

    let cache = Cache::default();
    let repo_token = Repo::new("Qwen/Qwen2.5-0.5B-Instruct".to_string(), RepoType::Model);
    let repo_model = Repo::new("Qwen/Qwen2.5-0.5B-Instruct-GGUF".to_string(), RepoType::Model);

    let is_cached = cache.repo(repo_token).get("tokenizer.json").is_some()
        && cache.repo(repo_model).get("qwen2.5-0.5b-instruct-q4_k_m.gguf").is_some();

    if !is_cached {
        println!("{}", "📥 Model AI lokal built-in tidak ditemukan.".truecolor(251, 191, 36).bold());
        println!("{}", "   Mengunduh Qwen2.5-0.5B (~396MB) dari Hugging Face Hub...".truecolor(251, 191, 36).bold());
        println!("{}", "   (Proses ini hanya sekali, selanjutnya akan berjalan 100% offline)".truecolor(160, 160, 160));
        println!();
    }

    // Build API with retries enabled for large file downloads
    let api = ApiBuilder::new()
        .with_retries(3)
        .build()
        .context("Gagal menginisialisasi Hugging Face API client")?;

    // Get tokenizer from main repository (small file, rarely fails)
    let repo_info = api.model("Qwen/Qwen2.5-0.5B-Instruct".to_string());
    let tokenizer_path = repo_info.get("tokenizer.json")
        .context("Gagal mengunduh tokenizer.json")?;

    // Get model file from GGUF repository (~396MB, may need retries)
    let model_repo = api.repo(Repo::new(
        "Qwen/Qwen2.5-0.5B-Instruct-GGUF".to_string(),
        RepoType::Model,
    ));

    const MAX_ATTEMPTS: u32 = 3;
    let model_path = {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match model_repo.get("qwen2.5-0.5b-instruct-q4_k_m.gguf") {
                Ok(path) => break Ok(path),
                Err(e) => {
                    if attempt < MAX_ATTEMPTS {
                        println!(
                            "{}",
                            format!(
                                "   ⚠️  Percobaan {}/{} gagal: {}. Mengulang...",
                                attempt, MAX_ATTEMPTS, e
                            )
                            .truecolor(251, 191, 36)
                        );
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    } else {
                        break Err(anyhow::anyhow!(
                            "Gagal mengunduh qwen2.5-0.5b-instruct-q4_k_m.gguf setelah {} percobaan: {}",
                            MAX_ATTEMPTS,
                            e
                        ));
                    }
                }
            }
        }
    }?;

    if !is_cached {
        println!("{}", "✅ Model dan tokenizer berhasil diunduh!".truecolor(134, 239, 172).bold());
        println!();
    }

    Ok((model_path, tokenizer_path))
}