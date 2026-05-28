use anyhow::{Context, Result};
use std::path::PathBuf;

use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_llama::ModelWeights;
use tokenizers::Tokenizer;

use crate::rag::utils::ColorExt;
use crate::core::NativeConfig;

pub struct NativeModelClient {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    config: NativeConfig,
}

impl NativeModelClient {
    /// Initialize native model, downloading from Hugging Face if not present, using default configuration
    pub fn load_built_in() -> Result<Self> {
        let llm_config = crate::core::LlmConfig::load_or_create()?;
        Self::load_with_config(llm_config.native)
    }

    /// Initialize native model with a specific configuration
    pub fn load_with_config(config: NativeConfig) -> Result<Self> {
        let (model_path, tokenizer_path) = download_model_with_config(&config)?;

        let device = Device::Cpu;

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
            config,
        })
    }

    pub fn generate_raw(&mut self, system: &str, prompt: &str) -> Result<String> {
        let mut active_model = self.model.clone();
        let is_cmd_gen = system.contains("FORMAT RESPONS WAJIB") && !system.contains("INFORMASI UMUM ASISTEN");

        let formatted_prompt = if is_cmd_gen {
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{{",
                system, prompt
            )
        } else if prompt.contains("Jelaskan output di atas") || prompt.contains("Ringkasan") {
            let simplified_system = "You are Hojicha, a Linux assistant. Summarize the terminal output.\n\n\
                 Respond ONLY with a JSON object in this format:\n\
                 {\"summary\": \"summary in Indonesian\", \"key_info\": \"key info in Indonesian\", \"next_suggestion\": \"suggestion in Indonesian or null\"}";
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{{",
                simplified_system, prompt
            )
        } else {
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                system, prompt
            )
        };

        let ends_with_brace = formatted_prompt.ends_with('{');

        let tokens = self.tokenizer.encode(formatted_prompt, true)
            .map_err(|e| anyhow::anyhow!("Encoding error: {}", e))?;
        let prompt_tokens = tokens.get_ids();

        let mut all_tokens = prompt_tokens.to_vec();
        let mut next_token = 0;
        let mut pos = 0;

        let mut logits_processor = LogitsProcessor::from_sampling(
            299792458,
            if is_cmd_gen {
                Sampling::ArgMax
            } else {
                Sampling::TopP {
                    p: self.config.top_p as f64,
                    temperature: self.config.temperature as f64,
                }
            },
        );

        let mut generated_tokens = vec![];

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

        let max_new_tokens = self.config.max_tokens;

        for _ in 0..max_new_tokens {
            if next_token == eos_token_id {
                break;
            }

            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = active_model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?;

            let logits = if generated_tokens.is_empty() {
                logits
            } else {
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.config.repeat_penalty.unwrap_or(1.1),
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

        let final_output = if ends_with_brace {
            format!("{{{}", output.trim())
        } else {
            output.trim().to_string()
        };
        Ok(final_output)
    }
}

fn download_model_with_config(config: &NativeConfig) -> Result<(PathBuf, PathBuf)> {
    use hf_hub::api::sync::Api;
    use hf_hub::{Cache, Repo, RepoType};

    let cache = Cache::default();
    let repo_token = Repo::new(config.tokenizer_repo.clone(), RepoType::Model);
    let repo_model = Repo::new(config.repo_id.clone(), RepoType::Model);

    let is_cached = cache.repo(repo_token).get(&config.tokenizer_filename).is_some()
        && cache.repo(repo_model).get(&config.filename).is_some();

    if !is_cached {
        println!("{}", "📥 Model AI native built-in tidak ditemukan.".yellow_or_colored());
        println!(
            "   Mengunduh {}/{} dari Hugging Face Hub...",
            config.repo_id, config.filename
        );
        println!("{}", "   (Proses ini hanya sekali, selanjutnya akan berjalan 100% offline)".dimmed_colored());
        println!();
    }

    let api = Api::new().context("Gagal menginisialisasi Hugging Face API client")?;

    let repo_info = api.model(config.tokenizer_repo.clone());
    let tokenizer_path = repo_info.get(&config.tokenizer_filename)
        .context(format!("Gagal mengunduh {}", config.tokenizer_filename))?;

    let model_repo = api.repo(Repo::new(
        config.repo_id.clone(),
        RepoType::Model,
    ));
    let model_path = model_repo.get(&config.filename)
        .context(format!("Gagal mengunduh {}", config.filename))?;

    if !is_cached {
        println!("{}", "✅ Model dan tokenizer berhasil diunduh!".green_or_colored());
        println!();
    }

    Ok((model_path, tokenizer_path))
}
