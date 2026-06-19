use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};

use crate::config::{ApiProvider, GeminiConfig, LlmConfig, OllamaConfig, OpenAiConfig};
use crate::rag::{AiClient, GeminiClient, OllamaClient, OpenAiClient};

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

async fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .send()
        .await
        .context("Gagal terhubung ke Ollama. Pastikan Ollama sudah berjalan.")?;

    #[derive(serde::Deserialize)]
    struct OllamaModel {
        name: String,
    }

    #[derive(serde::Deserialize)]
    struct OllamaTags {
        models: Vec<OllamaModel>,
    }

    let tags: OllamaTags = resp
        .json()
        .await
        .context("Gagal membaca daftar model dari respons Ollama")?;

    Ok(tags.models.into_iter().map(|m| m.name).collect())
}

/// Helper function to load AI client based on configuration.
pub async fn load_ai_client_from_config(config: &LlmConfig) -> Result<AiClient> {
    Ok(match config.active_api_provider {
        ApiProvider::Ollama => AiClient::Ollama(OllamaClient::new(config.ollama.clone())),
        ApiProvider::Openai => AiClient::OpenAi(OpenAiClient::new(config.openai.clone())),
        ApiProvider::Gemini => AiClient::Gemini(GeminiClient::new(config.gemini.clone())),
        ApiProvider::Openrouter => {
            AiClient::OpenRouter(OpenAiClient::new_openrouter(config.openrouter.clone()))
        }
        ApiProvider::Groq => AiClient::Groq(OpenAiClient::new_groq(config.groq.clone())),
    })
}

/// Run interactive configuration wizard for LLM models.
pub async fn run_model_wizard(ai_client: &mut AiClient) -> Result<()> {
    let mut config = LlmConfig::load_or_create()?;
    let theme = ColorfulTheme::default();

    println!();
    println!("⚙️  === WIZARD KONFIGURASI MODEL HOJICHA ===");

    loop {
        println!();
        println!("Provider Aktif Saat Ini: {}", active_provider_desc(&config));

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
                let providers = provider_menu();
                let default = provider_index(config.active_api_provider);
                let prov_selection = Select::with_theme(&theme)
                    .with_prompt("Pilih Provider Aktif baru:")
                    .default(default)
                    .items(&providers)
                    .interact()?;

                config.active_api_provider = provider_from_index(prov_selection);
                println!("✅ Provider aktif diubah!");
            }
            1 => {
                let mut providers_to_config = provider_menu();
                providers_to_config.push("Kembali");

                let config_selection = Select::with_theme(&theme)
                    .with_prompt("Pilih provider yang ingin dikonfigurasi:")
                    .default(provider_index(config.active_api_provider))
                    .items(&providers_to_config)
                    .interact()?;

                match provider_from_index(config_selection) {
                    ApiProvider::Ollama if config_selection < 5 => {
                        configure_ollama(&theme, &mut config.ollama).await?;
                    }
                    ApiProvider::Openai if config_selection < 5 => {
                        configure_openai_compatible(
                            &theme,
                            "OpenAI",
                            &mut config.openai,
                            &["gpt-4o-mini", "gpt-4o"],
                            "https://api.openai.com/v1",
                        )?;
                    }
                    ApiProvider::Gemini if config_selection < 5 => {
                        configure_gemini(&theme, &mut config.gemini)?;
                    }
                    ApiProvider::Openrouter if config_selection < 5 => {
                        configure_openai_compatible(
                            &theme,
                            "OpenRouter",
                            &mut config.openrouter,
                            &[
                                "openai/gpt-4o-mini",
                                "google/gemini-2.5-flash",
                                "anthropic/claude-sonnet-4",
                            ],
                            OPENROUTER_BASE_URL,
                        )?;
                    }
                    ApiProvider::Groq if config_selection < 5 => {
                        configure_openai_compatible(
                            &theme,
                            "Groq",
                            &mut config.groq,
                            &[
                                "llama-3.1-8b-instant",
                                "llama-3.3-70b-versatile",
                                "llama-3.1-70b-versatile",
                            ],
                            GROQ_BASE_URL,
                        )?;
                    }
                    _ => {}
                }
            }
            2 => {
                println!("💾 Menyimpan konfigurasi ke file...");
                config.save()?;
                println!("⏳ Memuat ulang model client dengan konfigurasi baru...");
                match load_ai_client_from_config(&config).await {
                    Ok(new_client) => {
                        *ai_client = new_client;
                        println!("✅ Model client berhasil dimuat ulang dan diterapkan!");
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Gagal memuat client baru: {}. Tetap menggunakan model lama.",
                            e
                        );
                    }
                }
                break;
            }
            3 => {
                println!("🚪 Keluar dari wizard konfigurasi.");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn active_provider_desc(config: &LlmConfig) -> String {
    match config.active_api_provider {
        ApiProvider::Ollama => format!("Ollama Lokal (model: {})", config.ollama.model),
        ApiProvider::Openai => format!("OpenAI (model: {})", config.openai.model),
        ApiProvider::Gemini => format!("Gemini (model: {})", config.gemini.model),
        ApiProvider::Openrouter => format!("OpenRouter (model: {})", config.openrouter.model),
        ApiProvider::Groq => format!("Groq (model: {})", config.groq.model),
    }
}

fn provider_menu() -> Vec<&'static str> {
    vec!["Ollama Lokal", "OpenAI", "Gemini", "OpenRouter", "Groq"]
}

fn provider_index(provider: ApiProvider) -> usize {
    match provider {
        ApiProvider::Ollama => 0,
        ApiProvider::Openai => 1,
        ApiProvider::Gemini => 2,
        ApiProvider::Openrouter => 3,
        ApiProvider::Groq => 4,
    }
}

fn provider_from_index(index: usize) -> ApiProvider {
    match index {
        1 => ApiProvider::Openai,
        2 => ApiProvider::Gemini,
        3 => ApiProvider::Openrouter,
        4 => ApiProvider::Groq,
        _ => ApiProvider::Ollama,
    }
}

fn mask_key(key: Option<&String>) -> String {
    key.map(|k| {
        if k.len() > 8 {
            format!("{}...{}", &k[..4], &k[k.len() - 4..])
        } else {
            "****".to_string()
        }
    })
    .unwrap_or_else(|| "Belum diatur".to_string())
}

fn choose_model(
    theme: &ColorfulTheme,
    provider_name: &str,
    current_model: &str,
    models: &[&str],
) -> Result<String> {
    let mut model_options = models.to_vec();
    model_options.push("Custom");
    let default = models
        .iter()
        .position(|model| *model == current_model)
        .unwrap_or(model_options.len() - 1);

    let model_select = Select::with_theme(theme)
        .with_prompt(format!("Pilih Model {}:", provider_name))
        .default(default)
        .items(&model_options)
        .interact()?;

    if model_select == model_options.len() - 1 {
        Ok(Input::with_theme(theme)
            .with_prompt("Masukkan nama model custom")
            .default(current_model.to_string())
            .interact_text()?)
    } else {
        Ok(model_options[model_select].to_string())
    }
}

async fn configure_ollama(theme: &ColorfulTheme, conf: &mut OllamaConfig) -> Result<()> {
    println!("\n🔧 Konfigurasi Ollama Lokal:");

    conf.base_url = Input::with_theme(theme)
        .with_prompt("Base URL")
        .default(conf.base_url.clone())
        .interact_text()?;

    let model_choice_options = vec![
        "Pilih dari model Ollama lokal yang terinstal",
        "Masukkan nama model secara manual",
    ];

    let choice = Select::with_theme(theme)
        .with_prompt("Cara memilih model:")
        .default(0)
        .items(&model_choice_options)
        .interact()?;

    if choice == 0 {
        println!("🔍 Menghubungi Ollama untuk mengambil daftar model...");
        match fetch_ollama_models(&conf.base_url).await {
            Ok(models) if !models.is_empty() => {
                let default = models
                    .iter()
                    .position(|model| model == &conf.model)
                    .unwrap_or(0);
                let model_select = Select::with_theme(theme)
                    .with_prompt("Pilih model Ollama:")
                    .default(default)
                    .items(&models)
                    .interact()?;
                conf.model = models[model_select].clone();
            }
            Ok(_) => {
                println!("⚠️  Tidak ada model terinstal di Ollama Anda.");
                conf.model = Input::with_theme(theme)
                    .with_prompt("Masukkan nama model secara manual")
                    .default(conf.model.clone())
                    .interact_text()?;
            }
            Err(e) => {
                println!("⚠️  {} Masukkan nama secara manual.", e);
                conf.model = Input::with_theme(theme)
                    .with_prompt("Masukkan nama model secara manual")
                    .default(conf.model.clone())
                    .interact_text()?;
            }
        }
    } else {
        conf.model = Input::with_theme(theme)
            .with_prompt("Nama Model")
            .default(conf.model.clone())
            .interact_text()?;
    }

    conf.temperature = Input::with_theme(theme)
        .with_prompt("Temperature")
        .default(conf.temperature)
        .interact_text()?;

    conf.max_tokens = Input::with_theme(theme)
        .with_prompt("Max Tokens")
        .default(conf.max_tokens)
        .interact_text()?;

    println!("✅ Konfigurasi Ollama diperbarui di memori!");
    Ok(())
}

fn configure_openai_compatible(
    theme: &ColorfulTheme,
    provider_name: &str,
    conf: &mut OpenAiConfig,
    models: &[&str],
    default_base_url: &str,
) -> Result<()> {
    println!("\n🔧 Konfigurasi {}:", provider_name);
    println!("API Key Saat Ini: {}", mask_key(conf.api_key.as_ref()));

    let change_key = dialoguer::Confirm::with_theme(theme)
        .with_prompt("Ubah API Key?")
        .default(false)
        .interact()?;

    if change_key || conf.api_key.is_none() {
        let key: String = Password::with_theme(theme)
            .with_prompt(format!("API Key {}", provider_name))
            .interact()?;
        conf.api_key = Some(key);
    }

    conf.model = choose_model(theme, provider_name, &conf.model, models)?;

    let base_url_input: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Base URL {} (kosong = {})",
            provider_name, default_base_url
        ))
        .default(conf.base_url.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    conf.base_url = if base_url_input.trim().is_empty() {
        None
    } else {
        Some(base_url_input)
    };

    conf.temperature = Input::with_theme(theme)
        .with_prompt("Temperature")
        .default(conf.temperature)
        .interact_text()?;

    conf.max_tokens = Input::with_theme(theme)
        .with_prompt("Max Tokens")
        .default(conf.max_tokens)
        .interact_text()?;

    println!("✅ Konfigurasi {} diperbarui di memori!", provider_name);
    Ok(())
}

fn configure_gemini(theme: &ColorfulTheme, conf: &mut GeminiConfig) -> Result<()> {
    println!("\n🔧 Konfigurasi Gemini:");
    println!("API Key Saat Ini: {}", mask_key(conf.api_key.as_ref()));

    let change_key = dialoguer::Confirm::with_theme(theme)
        .with_prompt("Ubah API Key?")
        .default(false)
        .interact()?;

    if change_key || conf.api_key.is_none() {
        let key: String = Password::with_theme(theme)
            .with_prompt("API Key Gemini")
            .interact()?;
        conf.api_key = Some(key);
    }

    conf.model = choose_model(
        theme,
        "Gemini",
        &conf.model,
        &[
            "gemini-3.5-flash-lite-latest",
            "gemini-3.5-flash",
            "gemini-flash-latest",
            "gemini-2.5-flash",
            "gemini-2.5-pro",
        ],
    )?;

    let base_url_input: String = Input::with_theme(theme)
        .with_prompt("Base URL Gemini (kosong = https://generativelanguage.googleapis.com)")
        .default(conf.base_url.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    conf.base_url = if base_url_input.trim().is_empty() {
        None
    } else {
        Some(base_url_input)
    };

    conf.temperature = Input::with_theme(theme)
        .with_prompt("Temperature")
        .default(conf.temperature)
        .interact_text()?;

    conf.max_tokens = Input::with_theme(theme)
        .with_prompt("Max Tokens")
        .default(conf.max_tokens)
        .interact_text()?;

    println!("✅ Konfigurasi Gemini diperbarui di memori!");
    Ok(())
}
