use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Select, Input, Password};
use crate::config::{LlmConfig, LlmType, ApiProvider};
use crate::rag::{AiClient, NativeModelClient, OllamaClient, OpenAiClient, GeminiClient, AnthropicClient};

/// Fetch models list from Ollama server
async fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    
    let resp = client.get(&url).send().await
        .context("Gagal terhubung ke Ollama. Pastikan Ollama sudah berjalan.")?;
    
    #[derive(serde::Deserialize)]
    struct OllamaModel {
        name: String,
    }
    #[derive(serde::Deserialize)]
    struct OllamaTags {
        models: Vec<OllamaModel>,
    }
    
    let tags: OllamaTags = resp.json().await
        .context("Gagal membaca daftar model dari respons Ollama")?;
    
    Ok(tags.models.into_iter().map(|m| m.name).collect())
}

/// Helper function to load AI client based on configuration
pub async fn load_ai_client_from_config(config: &LlmConfig) -> Result<AiClient> {
    match config.active {
        LlmType::Native => {
            let native_client = NativeModelClient::load_with_config(config.native.clone())?;
            Ok(AiClient::Native(native_client))
        }
        LlmType::Api => {
            match config.active_api_provider {
                ApiProvider::Ollama => {
                    let ollama_client = OllamaClient::new(&config.ollama.base_url, &config.ollama.model);
                    Ok(AiClient::Ollama(ollama_client))
                }
                ApiProvider::Openai => {
                    let openai_client = OpenAiClient::new(config.openai.clone());
                    Ok(AiClient::OpenAi(openai_client))
                }
                ApiProvider::Gemini => {
                    let gemini_client = GeminiClient::new(config.gemini.clone());
                    Ok(AiClient::Gemini(gemini_client))
                }
                ApiProvider::Anthropic => {
                    let anthropic_client = AnthropicClient::new(config.anthropic.clone());
                    Ok(AiClient::Anthropic(anthropic_client))
                }
            }
        }
    }
}

/// Run interactive configuration wizard for LLM models
pub async fn run_model_wizard(ai_client: &mut AiClient) -> Result<()> {
    let mut config = LlmConfig::load_or_create()?;
    let theme = ColorfulTheme::default();
    
    println!();
    println!("⚙️  === WIZARD KONFIGURASI MODEL HOJICHA ===");
    
    loop {
        let active_desc = match config.active {
            LlmType::Native => "Native (Offline GGUF)".to_string(),
            LlmType::Api => match config.active_api_provider {
                ApiProvider::Ollama => format!("Ollama (model: {})", config.ollama.model),
                ApiProvider::Openai => format!("OpenAI (model: {})", config.openai.model),
                ApiProvider::Gemini => format!("Gemini (model: {})", config.gemini.model),
                ApiProvider::Anthropic => format!("Anthropic (model: {})", config.anthropic.model),
            }
        };
        
        println!();
        println!("Provider Aktif Saat Ini: {}", active_desc);
        
        let menu_options = vec![
            "1. Ubah Provider Aktif",
            "2. Konfigurasi Parameter Provider (API Key, Model, URL, dll.)",
            "3. Simpan dan Keluar",
            "4. Keluar Tanpa Menyimpan",
        ];
        
        let selection = Select::with_theme(&theme)
            .with_prompt("Pilih menu:")
            .default(0)
            .items(&menu_options)
            .interact()?;
            
        match selection {
            0 => {
                // Change Active Provider
                let providers = vec![
                    "Native (Model Offline Built-in)",
                    "Ollama (Server Lokal)",
                    "OpenAI (Cloud)",
                    "Gemini (Cloud)",
                    "Anthropic (Cloud)",
                ];
                
                let prov_selection = Select::with_theme(&theme)
                    .with_prompt("Pilih Provider Aktif baru:")
                    .default(0)
                    .items(&providers)
                    .interact()?;
                    
                match prov_selection {
                    0 => {
                        config.active = LlmType::Native;
                    }
                    1 => {
                        config.active = LlmType::Api;
                        config.active_api_provider = ApiProvider::Ollama;
                    }
                    2 => {
                        config.active = LlmType::Api;
                        config.active_api_provider = ApiProvider::Openai;
                    }
                    3 => {
                        config.active = LlmType::Api;
                        config.active_api_provider = ApiProvider::Gemini;
                    }
                    4 => {
                        config.active = LlmType::Api;
                        config.active_api_provider = ApiProvider::Anthropic;
                    }
                    _ => {}
                }
                println!("✅ Provider aktif diubah!");
            }
            1 => {
                // Configure Provider Parameters
                let providers_to_config = vec![
                    "Native (Offline GGUF)",
                    "Ollama",
                    "OpenAI",
                    "Gemini",
                    "Anthropic",
                    "Kembali",
                ];
                
                let config_selection = Select::with_theme(&theme)
                    .with_prompt("Pilih provider yang ingin dikonfigurasi:")
                    .default(0)
                    .items(&providers_to_config)
                    .interact()?;
                    
                match config_selection {
                    0 => {
                        // Native
                        println!("\n🔧 Konfigurasi Native Model:");
                        let mut native_conf = config.native.clone();
                        
                        native_conf.repo_id = Input::with_theme(&theme)
                            .with_prompt("Hugging Face Repo ID")
                            .default(native_conf.repo_id)
                            .interact_text()?;
                            
                        native_conf.filename = Input::with_theme(&theme)
                            .with_prompt("GGUF Filename")
                            .default(native_conf.filename)
                            .interact_text()?;
                            
                        native_conf.tokenizer_repo = Input::with_theme(&theme)
                            .with_prompt("Tokenizer Repo")
                            .default(native_conf.tokenizer_repo)
                            .interact_text()?;
                            
                        native_conf.temperature = Input::with_theme(&theme)
                            .with_prompt("Temperature")
                            .default(native_conf.temperature)
                            .interact_text()?;
                            
                        native_conf.max_tokens = Input::with_theme(&theme)
                            .with_prompt("Max Tokens")
                            .default(native_conf.max_tokens)
                            .interact_text()?;
                            
                        config.native = native_conf;
                        println!("✅ Konfigurasi Native diperbarui di memori!");
                    }
                    1 => {
                        // Ollama
                        println!("\n🔧 Konfigurasi Ollama:");
                        let mut ollama_conf = config.ollama.clone();
                        
                        ollama_conf.base_url = Input::with_theme(&theme)
                            .with_prompt("Base URL")
                            .default(ollama_conf.base_url)
                            .interact_text()?;
                            
                        // Model choice
                        let model_choice_options = vec![
                            "Pilih dari model Ollama lokal yang terinstal",
                            "Masukkan nama model secara manual",
                        ];
                        
                        let choice = Select::with_theme(&theme)
                            .with_prompt("Cara memilih model:")
                            .default(0)
                            .items(&model_choice_options)
                            .interact()?;
                            
                        if choice == 0 {
                            println!("🔍 Menghubungi Ollama untuk mengambil daftar model...");
                            match fetch_ollama_models(&ollama_conf.base_url).await {
                                Ok(models) => {
                                    if models.is_empty() {
                                        println!("⚠️  Tidak ada model terinstal di Ollama Anda.");
                                        ollama_conf.model = Input::with_theme(&theme)
                                            .with_prompt("Masukkan nama model secara manual")
                                            .default(ollama_conf.model)
                                            .interact_text()?;
                                    } else {
                                        let model_select = Select::with_theme(&theme)
                                            .with_prompt("Pilih model Ollama:")
                                            .default(0)
                                            .items(&models)
                                            .interact()?;
                                        ollama_conf.model = models[model_select].clone();
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  {} Masukkan nama secara manual.", e);
                                    ollama_conf.model = Input::with_theme(&theme)
                                        .with_prompt("Masukkan nama model secara manual")
                                        .default(ollama_conf.model)
                                        .interact_text()?;
                                }
                            }
                        } else {
                            ollama_conf.model = Input::with_theme(&theme)
                                .with_prompt("Nama Model")
                                .default(ollama_conf.model)
                                .interact_text()?;
                        }
                        
                        ollama_conf.temperature = Input::with_theme(&theme)
                            .with_prompt("Temperature")
                            .default(ollama_conf.temperature)
                            .interact_text()?;
                            
                        ollama_conf.max_tokens = Input::with_theme(&theme)
                            .with_prompt("Max Tokens")
                            .default(ollama_conf.max_tokens)
                            .interact_text()?;
                            
                        config.ollama = ollama_conf;
                        println!("✅ Konfigurasi Ollama diperbarui di memori!");
                    }
                    2 => {
                        // OpenAI
                        println!("\n🔧 Konfigurasi OpenAI:");
                        let mut openai_conf = config.openai.clone();
                        
                        let current_key_masked = openai_conf.api_key.as_ref()
                            .map(|k| if k.len() > 8 { format!("{}...{}", &k[..4], &k[k.len()-4..]) } else { "****".to_string() })
                            .unwrap_or_else(|| "Belum diatur".to_string());
                            
                        println!("API Key Saat Ini: {}", current_key_masked);
                        let change_key = dialoguer::Confirm::with_theme(&theme)
                            .with_prompt("Ubah API Key?")
                            .default(false)
                            .interact()?;
                            
                        if change_key || openai_conf.api_key.is_none() {
                            let key: String = Password::with_theme(&theme)
                                .with_prompt("API Key OpenAI")
                                .interact()?;
                            openai_conf.api_key = Some(key);
                        }
                        
                        let models = vec!["gpt-4o-mini", "gpt-4o", "Custom"];
                        let model_select = Select::with_theme(&theme)
                            .with_prompt("Pilih Model:")
                            .default(0)
                            .items(&models)
                            .interact()?;
                            
                        if model_select == 2 {
                            openai_conf.model = Input::with_theme(&theme)
                                .with_prompt("Masukkan nama model custom")
                                .default(openai_conf.model)
                                .interact_text()?;
                        } else {
                            openai_conf.model = models[model_select].to_string();
                        }
                        
                        let base_url_input: String = Input::with_theme(&theme)
                            .with_prompt("Base URL OpenAI (Biarkan kosong untuk default)")
                            .default(openai_conf.base_url.clone().unwrap_or_default())
                            .allow_empty(true)
                            .interact_text()?;
                            
                        openai_conf.base_url = if base_url_input.trim().is_empty() {
                            None
                        } else {
                            Some(base_url_input)
                        };
                        
                        config.openai = openai_conf;
                        println!("✅ Konfigurasi OpenAI diperbarui di memori!");
                    }
                    3 => {
                        // Gemini
                        println!("\n🔧 Konfigurasi Gemini:");
                        let mut gemini_conf = config.gemini.clone();
                        
                        let current_key_masked = gemini_conf.api_key.as_ref()
                            .map(|k| if k.len() > 8 { format!("{}...{}", &k[..4], &k[k.len()-4..]) } else { "****".to_string() })
                            .unwrap_or_else(|| "Belum diatur".to_string());
                            
                        println!("API Key Saat Ini: {}", current_key_masked);
                        let change_key = dialoguer::Confirm::with_theme(&theme)
                            .with_prompt("Ubah API Key?")
                            .default(false)
                            .interact()?;
                            
                        if change_key || gemini_conf.api_key.is_none() {
                            let key: String = Password::with_theme(&theme)
                                .with_prompt("API Key Gemini")
                                .interact()?;
                            gemini_conf.api_key = Some(key);
                        }
                        
                        let models = vec!["gemini-1.5-flash", "gemini-1.5-pro", "Custom"];
                        let model_select = Select::with_theme(&theme)
                            .with_prompt("Pilih Model:")
                            .default(0)
                            .items(&models)
                            .interact()?;
                            
                        if model_select == 2 {
                            gemini_conf.model = Input::with_theme(&theme)
                                .with_prompt("Masukkan nama model custom")
                                .default(gemini_conf.model)
                                .interact_text()?;
                        } else {
                            gemini_conf.model = models[model_select].to_string();
                        }
                        
                        let base_url_input: String = Input::with_theme(&theme)
                            .with_prompt("Base URL Gemini (Biarkan kosong untuk default)")
                            .default(gemini_conf.base_url.clone().unwrap_or_default())
                            .allow_empty(true)
                            .interact_text()?;
                            
                        gemini_conf.base_url = if base_url_input.trim().is_empty() {
                            None
                        } else {
                            Some(base_url_input)
                        };
                        
                        config.gemini = gemini_conf;
                        println!("✅ Konfigurasi Gemini diperbarui di memori!");
                    }
                    4 => {
                        // Anthropic
                        println!("\n🔧 Konfigurasi Anthropic:");
                        let mut anthropic_conf = config.anthropic.clone();
                        
                        let current_key_masked = anthropic_conf.api_key.as_ref()
                            .map(|k| if k.len() > 8 { format!("{}...{}", &k[..4], &k[k.len()-4..]) } else { "****".to_string() })
                            .unwrap_or_else(|| "Belum diatur".to_string());
                            
                        println!("API Key Saat Ini: {}", current_key_masked);
                        let change_key = dialoguer::Confirm::with_theme(&theme)
                            .with_prompt("Ubah API Key?")
                            .default(false)
                            .interact()?;
                            
                        if change_key || anthropic_conf.api_key.is_none() {
                            let key: String = Password::with_theme(&theme)
                                .with_prompt("API Key Anthropic")
                                .interact()?;
                            anthropic_conf.api_key = Some(key);
                        }
                        
                        let models = vec!["claude-3-5-sonnet-latest", "claude-3-5-haiku-latest", "Custom"];
                        let model_select = Select::with_theme(&theme)
                            .with_prompt("Pilih Model:")
                            .default(0)
                            .items(&models)
                            .interact()?;
                            
                        if model_select == 2 {
                            anthropic_conf.model = Input::with_theme(&theme)
                                .with_prompt("Masukkan nama model custom")
                                .default(anthropic_conf.model)
                                .interact_text()?;
                        } else {
                            anthropic_conf.model = models[model_select].to_string();
                        }
                        
                        let base_url_input: String = Input::with_theme(&theme)
                            .with_prompt("Base URL Anthropic (Biarkan kosong untuk default)")
                            .default(anthropic_conf.base_url.clone().unwrap_or_default())
                            .allow_empty(true)
                            .interact_text()?;
                            
                        anthropic_conf.base_url = if base_url_input.trim().is_empty() {
                            None
                        } else {
                            Some(base_url_input)
                        };
                        
                        config.anthropic = anthropic_conf;
                        println!("✅ Konfigurasi Anthropic diperbarui di memori!");
                    }
                    _ => {}
                }
            }
            2 => {
                // Save and Exit
                println!("💾 Menyimpan konfigurasi ke file...");
                config.save()?;
                println!("⏳ Memuat ulang model client dengan konfigurasi baru...");
                match load_ai_client_from_config(&config).await {
                    Ok(new_client) => {
                        *ai_client = new_client;
                        println!("✅ Model client berhasil dimuat ulang dan diterapkan!");
                    }
                    Err(e) => {
                        println!("⚠️  Gagal memuat client baru: {}. Tetap menggunakan model lama.", e);
                    }
                }
                break;
            }
            3 => {
                // Exit without saving
                println!("🚪 Keluar dari wizard konfigurasi.");
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}
