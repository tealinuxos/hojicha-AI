/// Hojicha-AI: Lightweight AI-powered Linux CLI assistant for beginners.
/// Hybrid RAG architecture: Intent → Rules → RAG → LLM → Safety → Exec
pub mod config;
pub mod executor;
pub mod rag;
pub mod safety;
pub mod ui;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use executor::execute_command;
use rag::{AiClient, LocalModelClient, RagPipeline};
use safety::{check_safety, RiskLevel};
use serde_json::Value;
use std::io::{self, Write};
use std::path::Path;

// ─── CLI Definition ───────────────────────────────────────────────────────────

/// Hojicha - Asisten Terminal Linux untuk Pemula
#[derive(Parser)]
#[command(name = "hojicha")]
#[command(version = "0.1.0")]
#[command(about = "Asisten Terminal Linux bertenaga AI untuk Pemula", long_about = None)]
#[command(after_help = "Contoh:\n  hojicha \"cek ram laptop saya\"\n  hojicha --opencode http://127.0.0.1:20128/v1 \"hello\"\n  hojicha --opencode http://127.0.0.1:20128/v1 model-name \"hello\"")]
pub struct Cli {
    /// Pertanyaan atau perintah dalam bahasa alami (opsional - tanpa argumen masuk mode interaktif)
    pub query: Option<String>,

    /// URL server Ollama
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// Nama model Ollama yang akan digunakan
    #[arg(short, long, default_value = "qwen2.5:1.5b")]
    pub model: String,

    /// Gunakan model built-in offline lokal (tanpa memerlukan Ollama)
    #[arg(short, long, default_value_t = false)]
    pub local: bool,

    /// Gunakan OpenAI-compatible API: --opencode <URL> [MODEL_NAME]
    #[arg(long, num_args = 1..=2)]
    pub opencode: Option<Vec<String>>,

    /// Jangan tampilkan ringkasan output AI (lebih cepat)
    #[arg(long, default_value_t = false)]
    pub no_summary: bool,

    /// Jalankan tanpa konfirmasi (langsung eksekusi)
    #[arg(short = 'y', long, default_value_t = false)]
    pub yes: bool,

    /// Path ke file data JSON untuk RAG knowledge base
    #[arg(long)]
    pub kb_path: Option<String>,
}

fn resolve_kb_path(cli_path: Option<&str>) -> std::path::PathBuf {
    if let Some(path_str) = cli_path {
        return std::path::PathBuf::from(path_str);
    }

    let suffix = if std::env::consts::OS == "macos" {
        "macos"
    } else {
        "linux"
    };
    let filename = format!("knowledge_base_{}.json", suffix);

    // Primary: ~/.config/hojicha/knowledge_base_<os>.json
    // If file not found, kb.rs will auto-create it from embedded data on first run.
    if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home)
            .join(".config")
            .join("hojicha")
            .join(&filename)
    } else {
        // Fallback: path that likely won't exist — kb.rs embedded fallback will handle it
        std::path::PathBuf::from("/tmp").join(&filename)
    }
}

// ─── Main run function ────────────────────────────────────────────────────────

pub async fn run(cli: Cli) -> Result<()> {
    let kb_file_path = resolve_kb_path(cli.kb_path.as_deref());
    // Build RAG pipeline (indexes KB in memory, ~1ms)
    let rag_pipeline = RagPipeline::build(&kb_file_path);

    // Load AI client based on priority: --local > --opencode > config file
    let mut ai_client = if cli.local {
        // Force loading local built-in model
        println!("{}", "Mengaktifkan model built-in offline lokal...".yellow());
        AiClient::Local(Box::new(LocalModelClient::load_built_in()?))
    } else if let Some(opencode_args) = cli.opencode {
        let base_url = opencode_args[0].clone();
        let api_key = load_opencode_api_key().unwrap_or_default();
        let model = if opencode_args.len() > 1 {
            opencode_args[1].clone()
        } else if cli.model != "qwen2.5:1.5b" {
            // Use --model flag if explicitly set
            cli.model.clone()
        } else {
            // Use default model from opencode config if available
            load_opencode_model().unwrap_or_else(|| "gpt-4o-mini".to_string())
        };
        let openai_config = crate::config::OpenAiConfig {
            base_url: Some(base_url),
            api_key: Some(api_key),
            model,
            ..Default::default()
        };
        AiClient::OpenAi(rag::OpenAiClient::new(openai_config))
    } else {
        let config = crate::config::LlmConfig::load_or_create()?;

        // If provider is Ollama, apply --ollama-url and --model overrides
        let mut config = config;
        if config.active_api_provider == crate::config::ApiProvider::Ollama {
            if cli.ollama_url != "http://localhost:11434" {
                config.ollama.base_url = cli.ollama_url.clone();
            }
            if cli.model != "qwen2.5:1.5b" {
                config.ollama.model = cli.model.clone();
            }
        }

        // Auto-fallback: If Ollama is the provider but unreachable, fall back to local model
        if config.active_api_provider == crate::config::ApiProvider::Ollama {
            let ollama = rag::OllamaClient::new(config.ollama.clone());
            if !ollama.ping().await {
                println!("{}", "Ollama tidak terdeteksi.".yellow());
                println!("{}", "Mengaktifkan model built-in offline lokal...".yellow());
                AiClient::Local(Box::new(LocalModelClient::load_built_in()?))
            } else {
                crate::config::load_ai_client_from_config(&config).await?
            }
        } else {
            crate::config::load_ai_client_from_config(&config).await?
        }
    };

    if let Some(query) = &cli.query {
        return run_single_query(
            &mut ai_client,
            &rag_pipeline,
            query,
            cli.no_summary,
            cli.yes,
        )
        .await;
    }

    run_interactive(&mut ai_client, &rag_pipeline, cli.no_summary, cli.yes).await
}

fn load_opencode_api_key() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let config_path = std::path::PathBuf::from(home)
        .join(".config/opencode/opencode.json");
    let content = std::fs::read_to_string(config_path).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;

    // Try to find apiKey from any provider
    let provider = v["provider"].as_object()?;
    for (_, provider_config) in provider {
        if let Some(api_key) = provider_config["options"]["apiKey"].as_str() {
            return Some(api_key.to_string());
        }
    }
    None
}

fn load_opencode_model() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let config_path = std::path::PathBuf::from(home)
        .join(".config/opencode/opencode.json");
    let content = std::fs::read_to_string(config_path).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    let model = v["model"].as_str()?.to_string();

    // Strip provider prefix: "provider/model-name" -> "model-name"
    if model.contains('/') {
        Some(model[model.find('/').unwrap() + 1..].to_string())
    } else {
        Some(model)
    }
}

// ─── Single Query Mode ────────────────────────────────────────────────────────

async fn run_single_query(
    ai_client: &mut AiClient,
    rag: &RagPipeline,
    query: &str,
    no_summary: bool,
    auto_yes: bool,
) -> Result<()> {
    process_query(ai_client, rag, query, no_summary, auto_yes, &[]).await?;
    Ok(())
}

// ─── Interactive REPL Mode ────────────────────────────────────────────────────

async fn run_interactive(
    ai_client: &mut AiClient,
    rag: &RagPipeline,
    no_summary: bool,
    auto_yes: bool,
) -> Result<()> {
    ui::print_banner();

    match ai_client {
        AiClient::Ollama(ollama) => {
            println!(
                "  {} {}",
                "Terhubung ke Ollama lokal".green().bold(),
                format!("(model: {})", ollama.config.model).dimmed()
            );
        }
        AiClient::OpenAi(openai) => {
            println!(
                "  {} {}",
                "Terhubung ke OpenAI".green().bold(),
                format!("(model: {})", openai.config.model).dimmed()
            );
        }
        AiClient::Gemini(gemini) => {
            println!(
                "  {} {}",
                "Terhubung ke Gemini".green().bold(),
                format!("(model: {})", gemini.config.model).dimmed()
            );
        }
        AiClient::OpenRouter(openrouter) => {
            println!(
                "  {} {}",
                format!("Terhubung ke {}", openrouter.provider_name())
                    .green()
                    .bold(),
                format!("(model: {})", openrouter.config.model).dimmed()
            );
        }
        AiClient::Groq(groq) => {
            println!(
                "  {} {}",
                format!("Terhubung ke {}", groq.provider_name())
                    .green()
                    .bold(),
                format!("(model: {})", groq.config.model).dimmed()
            );
        }
        AiClient::Local(_) => {
            println!(
                "  {}",
                "Menggunakan model built-in lokal".green().bold()
            );
        }
    }
    println!();
    println!(
        "  {}",
        "Ketik /help atau /h untuk melihat bantuan.".truecolor(200, 200, 200)
    );
    println!();
    println!("{}", "─".repeat(50).truecolor(60, 60, 80));
    println!();

    let mut history: Vec<(String, String)> = Vec::new();

    loop {
        ui::print_prompt();
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        match input.to_lowercase().as_str() {
            "exit" | "quit" | "q" | "/exit" | "/q" => {
                ui::print_goodbye();
                break;
            }
            "help" | "?" | "h" | "/help" | "/h" => {
                ui::print_help();
                continue;
            }
            "model" | "/model" => {
                if let Err(e) = crate::config::run_model_wizard(ai_client).await {
                    ui::print_error(&format!("Gagal menjalankan konfigurasi model: {}", e));
                }
                continue;
            }
            "clear" | "/clear" => {
                history.clear();
                ui::print_history_cleared();
                continue;
            }
            _ => {}
        }

        // /find [folder|file] [/|.] <nama> — search tanpa lewat AI
        // Contoh:
        //   /find undip             → cari folder & file bernama undip di Home
        //   /find folder undip      → hanya folder
        //   /find file config.json  → hanya file
        //   /find / undip           → cari di seluruh laptop
        //   /find folder / undip    → hanya folder, seluruh laptop
        if input.starts_with("/find") || (input.starts_with("find ") && !input.contains("=")) {
            let args = input
                .trim_start_matches("/find")
                .trim_start_matches("find")
                .trim()
                .to_string();

            if args.is_empty() {
                println!("{}", "Penggunaan /find:".bold().truecolor(72, 187, 120));
                println!("    /find <nama>               → cari semua di Home (~)");
                println!("    /find folder <nama>        → hanya folder");
                println!("    /find file <nama>          → hanya file");
                println!("    /find / <nama>             → cari di seluruh laptop");
                println!("    /find folder / <nama>      → hanya folder, seluruh laptop");
            } else {
                // ── Parse tipe (folder/file/both) ──
                #[derive(PartialEq)]
                enum FindType { Both, FolderOnly, FileOnly }

                let (type_filter, rest) = if args.starts_with("folder ") {
                    (FindType::FolderOnly, args[7..].trim().to_string())
                } else if args.starts_with("file ") {
                    (FindType::FileOnly, args[5..].trim().to_string())
                } else {
                    (FindType::Both, args.clone())
                };

                // ── Parse scope (/, ., atau default ~) ──
                let (root, query) = if rest.starts_with("/ ") {
                    (std::path::PathBuf::from("/"), rest[2..].trim().to_string())
                } else if rest.starts_with(". ") {
                    (std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")), rest[2..].trim().to_string())
                } else if rest == "/" || rest == "." || rest.is_empty() {
                    ui::print_error("Nama yang dicari tidak boleh kosong.");
                    continue;
                } else {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
                    (std::path::PathBuf::from(home), rest.clone())
                };

                let scope_label = if root == std::path::PathBuf::from("/") {
                    "seluruh laptop".to_string()
                } else if root == std::env::current_dir().unwrap_or_default() {
                    format!("folder saat ini ({})", root.display())
                } else {
                    "Home (~)".to_string()
                };

                let type_label = match type_filter {
                    FindType::FolderOnly => " [folder]",
                    FindType::FileOnly   => " [file]",
                    FindType::Both       => "",
                };

                println!(
                    "  {} Mencari{} '{}' di {} ...",
                    "🔍".truecolor(104, 211, 145),
                    type_label.dimmed(),
                    query.bold(),
                    scope_label.dimmed()
                );

                let only_dirs  = type_filter == FindType::FolderOnly;
                let only_files = type_filter == FindType::FileOnly;
                let results = find_files_typed(&root, &query, 10, 300, only_dirs, only_files);
                ui::print_search_results(&query, &results);
            }
            continue;
        }

        match process_query(ai_client, rag, &input, no_summary, auto_yes, &history).await {
            Ok(Some(response_json)) => {
                history.push((input, response_json));
                if history.len() > 5 {
                    history.remove(0);
                }
            }
            Ok(None) => {}
            Err(e) => {
                ui::print_error(&e.to_string());
            }
        }
    }

    Ok(())
}

// ─── Core Query Processing ────────────────────────────────────────────────────

async fn process_query(
    ai_client: &mut AiClient,
    rag: &RagPipeline,
    user_input: &str,
    _no_summary: bool,
    auto_yes: bool,
    // FIXED: renamed from _history to history — now actually used
    history: &[(String, String)],
) -> Result<Option<String>> {
    // ── Step 1: RAG Pipeline → LLM ──────────────────────────────
    ui::print_thinking();
    // FIXED: Pass conversation history to the RAG pipeline so the LLM
    // has multi-turn context. Previously history was accepted but ignored.
    let cmd_resp = match rag.run(ai_client, user_input, history).await {
        Ok(r) => r,
        Err(e) => {
            ui::print_error(&format!("AI error: {}", e));
            return Ok(None);
        }
    };

    // ── Step 4: Display command info ────────────────────────────────
    let command = match &cmd_resp.command {
        None => {
            ui::print_no_command(&cmd_resp.explanation);
            return Ok(None);
        }
        Some(c) if c.trim().is_empty() => {
            ui::print_no_command(&cmd_resp.explanation);
            return Ok(None);
        }
        Some(c) => c.clone(),
    };

    // ── Step 5: Safety Check ────────────────────────────────────────
    let safety = check_safety(&command);

    match safety.risk {
        RiskLevel::Dangerous => {
            ui::print_blocked_dangerous(&safety.reason);
            return Ok(None);
        }
        RiskLevel::Moderate => {
            if auto_yes {
                // proceed
            } else {
                ui::print_confirm_moderate();
                io::stdout().flush()?;
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                let confirm = confirm.trim().to_lowercase();
                match confirm.as_str() {
                    "y" | "yes" | "ya" => {}
                    _ => {
                        ui::print_cancelled();
                        return Ok(None);
                    }
                }
            }
        }
        RiskLevel::Safe => {
            if !cmd_resp.is_safe {
                ui::print_blocked_dangerous("AI menandai perintah ini sebagai tidak aman.");
                return Ok(None);
            }
        }
    }

    // ── Step 6: Execute ─────────────────────────────────────────────
    ui::print_executing(&command);

    let output = match execute_command(&command) {
        Ok(o) => o,
        Err(e) => {
            ui::print_error(&format!("Gagal menjalankan perintah: {}", e));
            return Ok(None);
        }
    };

    ui::print_raw_output(&output.stdout);
    if !output.success && !output.stderr.is_empty() {
        ui::print_command_failed(&output.stderr, output.exit_code);
    }

    // Show explanation + tip if present
    if !cmd_resp.explanation.is_empty() {
        ui::print_explanation(&cmd_resp.explanation, cmd_resp.beginner_tip.as_deref());
    }

    // FIXED: Store the full response (command + explanation) for history context,
    // not just the JSON-serialized command string. Previously stored
    // `serde_json::to_string(&cmd_resp.command)` which lost all context.
    let history_entry = format!(
        "command: {}, explanation: {}",
        command,
        cmd_resp.explanation
    );
    Ok(Some(history_entry))
}

// ─── File Search ──────────────────────────────────────────────────────────────

/// Cari file/folder dengan filter tipe opsional.
/// - `only_dirs`: hanya kembalikan direktori
/// - `only_files`: hanya kembalikan file
pub fn find_files_typed(
    root: &Path,
    query: &str,
    max_depth: usize,
    max_results: usize,
    only_dirs: bool,
    only_files: bool,
) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();
    find_recursive(root, &query_lower, 0, max_depth, max_results, only_dirs, only_files, &mut results);
    results
}

/// Backward-compatible wrapper (cari semua tipe)
pub fn find_files(root: &Path, query: &str, max_depth: usize, max_results: usize) -> Vec<String> {
    find_files_typed(root, query, max_depth, max_results, false, false)
}

fn find_recursive(
    dir: &Path,
    query: &str,
    depth: usize,
    max_depth: usize,
    max_results: usize,
    only_dirs: bool,
    only_files: bool,
    results: &mut Vec<String>,
) {
    if depth > max_depth || results.len() >= max_results {
        return;
    }

    let skip_dirs = [
        // Build artifacts & Rust
        "target", ".git",
        // JS package managers & caches
        "node_modules", ".bun", ".npm", ".yarn", ".pnpm-store",
        // Python
        "__pycache__", ".venv", "venv", ".virtualenv",
        // Rust
        ".cargo",
        // General cache & temp
        ".cache", ".tmp", "tmp",
        // Build output
        "dist", "build", ".next", ".nuxt", ".svelte-kit", "out",
        // macOS system
        "Library",
        // PHP
        "vendor",
    ];

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let is_dir  = path.is_dir();
        let is_file = path.is_file();

        // Skip direktori berat
        if is_dir && skip_dirs.contains(&name.as_str()) {
            continue;
        }

        // Filter tipe
        let should_match = if only_dirs  { is_dir  }
                           else if only_files { is_file }
                           else { true };

        // Cocokkan nama (case-insensitive, partial match)
        if should_match && name.to_lowercase().contains(query) {
            // Untuk folder: tampilkan dengan trailing /
            let display = if is_dir {
                format!("{}/", path.display())
            } else {
                path.display().to_string()
            };
            results.push(display);
        }

        // Rekursi ke subdirektori
        if is_dir {
            find_recursive(&path, query, depth + 1, max_depth, max_results, only_dirs, only_files, results);
        }
    }
}
