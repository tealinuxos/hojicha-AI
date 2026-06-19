/// Local Model Client: Built-in offline inference using Candle + SmolLM2-135M-Instruct (GGUF).
/// Downloads model from Hugging Face Hub on first run, then works 100% offline.
use anyhow::{Context, Result};
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_llama::ModelWeights;
use colored::Colorize;
use std::path::PathBuf;
use tokenizers::Tokenizer;

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
                "<|im_start|>system\nYou are Hojicha, a Linux assistant. Translate user intent to a Linux command. Output ONLY the raw command. Do not explain. Do not use markdown.<|im_end|>\n\
                 <|im_start|>user\n{}<|im_end|>\n\
                 <|im_start|>assistant\n{}<|im_end|>\n\
                 <|im_start|>user\n{}<|im_end|>\n\
                 <|im_start|>assistant\n",
                example.0, example.1, prompt
            )
        } else if prompt.contains("Jelaskan output di atas") || prompt.contains("Ringkasan") {
            let simplified_system = "You are Hojicha, a Linux assistant. Summarize the terminal output.\n\n\
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
    use hf_hub::{Cache, Repo, RepoType};

    let cache = Cache::default();
    let repo_token = Repo::new("HuggingFaceTB/SmolLM2-135M-Instruct".to_string(), RepoType::Model);
    let repo_model = Repo::new("bartowski/SmolLM2-135M-Instruct-GGUF".to_string(), RepoType::Model);

    let is_cached = cache.repo(repo_token).get("tokenizer.json").is_some()
        && cache.repo(repo_model).get("SmolLM2-135M-Instruct-Q4_K_M.gguf").is_some();

    if !is_cached {
        println!("{}", "📥 Model AI lokal built-in tidak ditemukan.".truecolor(251, 191, 36).bold());
        println!("{}", "   Mengunduh SmolLM2-135M (105MB) dari Hugging Face Hub...".truecolor(251, 191, 36).bold());
        println!("{}", "   (Proses ini hanya sekali, selanjutnya akan berjalan 100% offline)".truecolor(160, 160, 160));
        println!();
    }

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

    if !is_cached {
        println!("{}", "✅ Model dan tokenizer berhasil diunduh!".truecolor(134, 239, 172).bold());
        println!();
    }

    Ok((model_path, tokenizer_path))
}