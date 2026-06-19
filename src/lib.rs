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
use dialoguer::{Select, MultiSelect};
use executor::execute_command;
use rag::{AiClient, LocalModelClient, RagPipeline};
use safety::{check_safety, RiskLevel};
use serde_json::Value;
use std::collections::HashSet;
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
    // Load config to initialize theme first thing
    if let Ok(config) = crate::config::LlmConfig::load_or_create() {
        ui::set_theme(config.theme.as_deref() == Some("light"));
    }

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
                ui::color_primary("Terhubung ke Ollama lokal").bold(),
                format!("(model: {})", ollama.config.model).dimmed()
            );
        }
        AiClient::OpenAi(openai) => {
            println!(
                "  {} {}",
                ui::color_primary("Terhubung ke OpenAI").bold(),
                format!("(model: {})", openai.config.model).dimmed()
            );
        }
        AiClient::Gemini(gemini) => {
            println!(
                "  {} {}",
                ui::color_primary("Terhubung ke Gemini").bold(),
                format!("(model: {})", gemini.config.model).dimmed()
            );
        }
        AiClient::OpenRouter(openrouter) => {
            println!(
                "  {} {}",
                ui::color_primary(&format!("Terhubung ke {}", openrouter.provider_name())).bold(),
                format!("(model: {})", openrouter.config.model).dimmed()
            );
        }
        AiClient::Groq(groq) => {
            println!(
                "  {} {}",
                ui::color_primary(&format!("Terhubung ke {}", groq.provider_name())).bold(),
                format!("(model: {})", groq.config.model).dimmed()
            );
        }
        AiClient::Local(_) => {
            println!(
                "  {}",
                ui::color_primary("Menggunakan model built-in lokal").bold()
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

    let history_file_path = if let Ok(home) = std::env::var("HOME") {
        Some(std::path::PathBuf::from(home)
            .join(".config")
            .join("hojicha")
            .join("history.txt"))
    } else {
        None
    };

    let mut rl = rustyline::DefaultEditor::new()?;
    if let Some(ref path) = history_file_path {
        if path.exists() {
            let should_load = if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        elapsed.as_secs() < 86400 // 1 day in seconds
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };

            if should_load {
                let _ = rl.load_history(path);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    loop {
        let readline = rl.readline(&format!("{} ", ui::color_primary("hojicha ❯").bold()));
        let input = match readline {
            Ok(line) => line,
            Err(rustyline::error::ReadlineError::Interrupted) => {
                // Ctrl-C
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                // Ctrl-D
                ui::print_goodbye();
                break;
            }
            Err(err) => {
                ui::print_error(&format!("Gagal membaca input: {:?}", err));
                break;
            }
        };

        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        let _ = rl.add_history_entry(&input);

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
            "theme" | "/theme" => {
                if let Err(e) = crate::config::run_theme_menu().await {
                    ui::print_error(&format!("Gagal menjalankan konfigurasi tema: {}", e));
                }
                continue;
            }
            "clear" | "/clear" => {
                history.clear();
                ui::print_history_cleared();
                continue;
            }
            "git" | "/git" => {
                match execute_command("git rev-parse --is-inside-work-tree") {
                    Ok(out) if out.success && out.stdout.trim() == "true" => {
                        let branch = execute_command("git branch --show-current")
                            .map(|o| o.stdout.trim().to_string())
                            .unwrap_or_else(|_| "Tidak diketahui".to_string());
                        
                        let remote = execute_command("git remote -v")
                            .map(|o| o.stdout.trim().to_string())
                            .unwrap_or_else(|_| "".to_string());
                        let remote_str = if remote.is_empty() {
                            "Tidak ada remote terkonfigurasi".to_string()
                        } else {
                            remote.lines().next().unwrap_or("").to_string()
                        };

                        let status = execute_command("git status --short")
                            .map(|o| o.stdout.trim().to_string())
                            .unwrap_or_else(|_| "".to_string());
                        let status_str = if status.is_empty() {
                            "Bersih (tidak ada perubahan)".to_string()
                        } else {
                            status
                        };

                        let commit = execute_command("git log -1 --oneline")
                            .map(|o| o.stdout.trim().to_string())
                            .unwrap_or_else(|_| "Belum ada commit".to_string());

                        println!();
                        println!("{}", ui::color_primary("=== INFORMASI REPOSITORY GIT ===").bold());
                        println!("  {} {}", ui::color_light("Branch aktif :"), branch);
                        println!("  {} {}", ui::color_light("Remote URL   :"), remote_str);
                        println!("  {}", ui::color_light("Status File  :"));
                        for line in status_str.lines() {
                            println!("    {}", line);
                        }
                        println!("  {}", ui::color_light("Commit Terakhir:"));
                        println!("    {}", commit);
                        println!();

                        history.push((
                            "/git".to_string(),
                            format!(
                                "Informasi Repository Git saat ini:\n\
                                 Branch: {}\n\
                                 Remote: {}\n\
                                 Status perubahan:\n{}\n\
                                 Commit terakhir: {}",
                                branch, remote_str, status_str, commit
                            )
                        ));
                        println!("  {}", "Data .git otomatis dimuat ke memori percakapan asisten.".italic().dimmed());
                        println!();
                    }
                    _ => {
                        ui::print_error("Direktori saat ini bukan merupakan repository Git. Silakan inisialisasi dengan 'git init'.");
                    }
                }
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
                    // FIXED: Use drain instead of remove(0) to avoid O(n) shift on Vec.
                    // With max 6 elements the impact is minimal, but drain is cleaner.
                    history.drain(..1);
                }
            }
            Ok(None) => {}
            Err(e) => {
                ui::print_error(&e.to_string());
            }
        }
    }

    if let Some(ref path) = history_file_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = rl.save_history(path);
    }

    Ok(())
}

// ─── Interactive Git Helpers ──────────────────────────────────────────────────

fn get_git_changes() -> Result<Vec<(String, String)>> {
    let out = execute_command("git status --porcelain")?;
    let mut changes = Vec::new();
    for line in out.stdout.lines() {
        if line.len() > 3 {
            let status = line[..2].to_string();
            let file = line[2..].trim().to_string();
            changes.push((status, file));
        }
    }
    Ok(changes)
}

fn run_interactive_staging() -> Result<Option<Vec<String>>> {
    let changes = get_git_changes()?;
    if changes.is_empty() {
        println!("Tidak ada file yang berubah atau belum dilacak.");
        return Ok(None);
    }

    println!("\nFile yang berubah/belum dilacak:");
    let items: Vec<String> = changes.iter().map(|(status, file)| {
        format!("{} - {}", status, file)
    }).collect();

    // Add option to select all
    let mut options = vec!["[PILIH SEMUA]".to_string()];
    options.extend(items.clone());

    let selection = MultiSelect::new()
        .with_prompt("Pilih file yang ingin di-stage (git add). Tekan Space untuk memilih, Enter untuk selesai")
        .items(&options)
        .defaults(&vec![false; options.len()])
        .interact()?;

    if selection.is_empty() {
        println!("Tidak ada file yang dipilih untuk di-stage.");
        return Ok(Some(Vec::new()));
    }

    let mut files_to_add = Vec::new();
    if selection.contains(&0) {
        // Select all files
        for (_, file) in changes {
            files_to_add.push(file);
        }
    } else {
        for idx in selection {
            if idx > 0 {
                files_to_add.push(changes[idx - 1].1.clone());
            }
        }
    }

    Ok(Some(files_to_add))
}

async fn generate_commit_message_ai(ai_client: &mut AiClient) -> Result<String> {
    let mut diff_out = execute_command("git diff --staged")?;
    let mut diff = diff_out.stdout;
    if diff.trim().is_empty() {
        diff_out = execute_command("git diff")?;
        diff = diff_out.stdout;
    }

    if diff.trim().is_empty() {
        return Err(anyhow::anyhow!("Tidak ada perubahan kode yang terdeteksi untuk membuat commit message."));
    }

    let system_prompt = crate::rag::prompt::commit_generator_prompt(&diff);
    println!("Sedang membuat pesan commit menggunakan AI...");
    let response = ai_client.nl_to_command(&system_prompt, "Generate a concise conventional commit message based on the diff.").await?;
    
    let msg = if !response.explanation.is_empty() {
        response.explanation.trim().to_string()
    } else if let Some(ref cmd) = response.command {
        cmd.trim().to_string()
    } else {
        "feat: update files".to_string()
    };

    Ok(msg)
}

fn confirm_command(command: &str) -> bool {
    print!("  Jalankan perintah ini? `{}` (y/n): ", command);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        trimmed == "y" || trimmed == "yes" || trimmed == "ya"
    } else {
        false
    }
}

// FIXED: Separate confirm helper for yes/no questions (not commands).
// Previously, confirm_command was misused for questions like "Apakah Anda ingin..."
// which displayed them as "Jalankan perintah ini? `Apakah Anda ingin...`" — confusing.
fn confirm_yes_no(question: &str) -> bool {
    print!("  {} (y/n): ", question);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        trimmed == "y" || trimmed == "yes" || trimmed == "ya"
    } else {
        false
    }
}

async fn handle_interactive_git_flow(
    ai_client: &mut AiClient,
    command: &str,
    user_input: &str,
) -> Result<Option<String>> {
    let command_lower = command.to_lowercase();
    let is_commit = command_lower.contains("git commit");
    let is_push = command_lower.contains("git push");
    let is_add = command_lower.contains("git add");

    // 1. Staging flow
    if is_commit {
        let changes = get_git_changes()?;
        if !changes.is_empty() {
            println!("\nAda {} file yang belum di-stage atau belum dilacak.", changes.len());
            let choices = &["Stage semua file (git add .)", "Pilih file untuk di-stage secara interaktif", "Lewati (commit file yang sudah di-stage saja)", "Batalkan"];
            let selection = Select::new()
                .with_prompt("Pilih tindakan staging")
                .items(choices)
                .default(0)
                .interact()?;

            match selection {
                0 => {
                    if confirm_command("git add .") {
                        execute_command("git add .")?;
                        println!("Semua file telah di-stage.");
                    } else {
                        println!("Staging dibatalkan.");
                    }
                }
                1 => {
                    if let Ok(Some(files)) = run_interactive_staging() {
                        if !files.is_empty() {
                            // SECURITY: Escape single quotes in filenames to prevent
                            // command injection. A file named "'; rm -rf /; '" would
                            // otherwise break out of the quoting.
                            let add_cmd = format!("git add {}", files.iter().map(|f| format!("'{}'", f.replace("'", "'\\''"))).collect::<Vec<_>>().join(" "));
                            if confirm_command(&add_cmd) {
                                execute_command(&add_cmd)?;
                                println!("File terpilih telah di-stage.");
                            } else {
                                println!("Staging dibatalkan.");
                            }
                        }
                    }
                }
                2 => {}
                _ => {
                    ui::print_cancelled();
                    return Ok(None);
                }
            }
        }
    }

    // 2. Add flow
    if is_add && (command.trim() == "git add" || command.trim() == "git add .") {
        let staged_something = if command.trim() == "git add" {
            if let Ok(Some(files)) = run_interactive_staging() {
                if !files.is_empty() {
                    // SECURITY: Escape single quotes in filenames
                    let add_cmd = format!("git add {}", files.iter().map(|f| format!("'{}'", f.replace("'", "'\\''"))).collect::<Vec<_>>().join(" "));
                    if confirm_command(&add_cmd) {
                        execute_command(&add_cmd)?;
                        println!("File terpilih telah di-stage.");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            if confirm_command(command) {
                let out = execute_command(command)?;
                ui::print_raw_output(&out.stdout);
                true
            } else {
                false
            }
        };

        if staged_something {
            if confirm_yes_no("Apakah Anda ingin langsung membuat commit untuk perubahan ini?") {
                let commit_res = Box::pin(handle_interactive_git_flow(ai_client, "git commit", user_input)).await?;
                return Ok(commit_res);
            } else {
                return Ok(Some(command.to_string()));
            }
        }
        ui::print_cancelled();
        return Ok(None);
    }

    // 3. Commit flow
    if is_commit {
        let has_message = (command.contains(" -m") || command.contains(" --message"))
            && (user_input.contains(" -m") || user_input.contains(" --message"));

        if !has_message {
            let choices = &["Buatkan dengan AI", "Ketik sendiri", "Batalkan"];
            let selection = Select::new()
                .with_prompt("Pilih cara pengisian pesan commit")
                .items(choices)
                .default(0)
                .interact()?;

            let commit_msg = match selection {
                0 => {
                    match generate_commit_message_ai(ai_client).await {
                        Ok(msg) => {
                            println!("\nPesan commit yang dibuat oleh AI:\n  \"{}\"", msg);
                            if confirm_yes_no("Gunakan pesan commit ini?") {
                                msg
                            } else {
                                println!("Pembuatan pesan commit dibatalkan.");
                                return Ok(None);
                            }
                        }
                        Err(e) => {
                            ui::print_error(&format!("Gagal membuat pesan commit dengan AI: {}", e));
                            println!("Silakan ketik pesan commit secara manual:");
                            let mut msg = String::new();
                            io::stdin().read_line(&mut msg)?;
                            let msg = msg.trim().to_string();
                            if !msg.is_empty() && confirm_yes_no(&format!("Gunakan pesan: \"{}\"?", msg)) {
                                msg
                            } else {
                                return Ok(None);
                            }
                        }
                    }
                }
                1 => {
                    println!("\nSilakan ketik pesan commit Anda:");
                    let mut msg = String::new();
                    io::stdin().read_line(&mut msg)?;
                    let msg = msg.trim().to_string();
                    if !msg.is_empty() && confirm_yes_no(&format!("Gunakan pesan: \"{}\"?", msg)) {
                        msg
                    } else {
                        println!("Commit dibatalkan.");
                        return Ok(None);
                    }
                }
                _ => {
                    ui::print_cancelled();
                    return Ok(None);
                }
            };

            if !commit_msg.is_empty() {
                let commit_cmd = format!("git commit -m '{}'", commit_msg.replace("'", "'\\''"));
                if confirm_command(&commit_cmd) {
                    let out = execute_command(&commit_cmd)?;
                    ui::print_raw_output(&out.stdout);
                    if !out.success {
                        ui::print_command_failed(&out.stderr, out.exit_code);
                    }
                    return Ok(Some(commit_cmd));
                }
            }
        } else {
            if confirm_command(command) {
                let out = execute_command(command)?;
                ui::print_raw_output(&out.stdout);
                if !out.success {
                    ui::print_command_failed(&out.stderr, out.exit_code);
                }
                return Ok(Some(command.to_string()));
            }
        }
        ui::print_cancelled();
        return Ok(None);
    }

    // 4. Push flow
    if is_push {
        let remotes_out = execute_command("git remote")?;
        let remotes: Vec<String> = remotes_out.stdout.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let remote = if remotes.is_empty() {
            "origin".to_string()
        } else if remotes.len() == 1 {
            remotes[0].clone()
        } else {
            let selection = Select::new()
                .with_prompt("Pilih remote repository tujuan")
                .items(&remotes)
                .default(0)
                .interact()?;
            remotes[selection].clone()
        };

        let branches_out = execute_command("git branch")?;
        let mut branches: Vec<String> = branches_out.stdout.lines().map(|s| s.trim().trim_start_matches('*').trim().to_string()).filter(|s| !s.is_empty()).collect();
        
        let current_branch = execute_command("git branch --show-current")
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_else(|_| "main".to_string());

        if let Some(pos) = branches.iter().position(|b| b == &current_branch) {
            branches.remove(pos);
        }
        let mut branch_options = vec![current_branch.clone()];
        branch_options.extend(branches);
        branch_options.push("[Ketik branch manual...]".to_string());

        let selection = Select::new()
            .with_prompt("Pilih branch tujuan push")
            .items(&branch_options)
            .default(0)
            .interact()?;

        let branch = if selection == branch_options.len() - 1 {
            print!("Masukkan nama branch tujuan: ");
            let _ = io::stdout().flush();
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        } else {
            branch_options[selection].clone()
        };

        if branch.is_empty() {
            println!("Nama branch tidak boleh kosong.");
            ui::print_cancelled();
            return Ok(None);
        }

        let push_cmd = format!("git push {} {}", remote, branch);
        if confirm_command(&push_cmd) {
            let out = execute_command(&push_cmd)?;
            ui::print_raw_output(&out.stdout);
            if !out.success {
                ui::print_command_failed(&out.stderr, out.exit_code);
            }
            return Ok(Some(push_cmd));
        }
        ui::print_cancelled();
        return Ok(None);
    }

    // Default fallback for any moderate git command
    if confirm_command(command) {
        let out = execute_command(command)?;
        ui::print_raw_output(&out.stdout);
        if !out.success {
            ui::print_command_failed(&out.stderr, out.exit_code);
        }
        return Ok(Some(command.to_string()));
    }

    ui::print_cancelled();
    Ok(None)
}

// ─── Core Query Processing ────────────────────────────────────────────────────

async fn process_query(
    ai_client: &mut AiClient,
    rag: &RagPipeline,
    user_input: &str,
    no_summary: bool,
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
            ui::print_error(&format!("AI error: {:#}", e));
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

    // Intercept Git commands for interactive workflow
    let trimmed_cmd = command.trim();
    let is_git_cmd = trimmed_cmd.starts_with("git ") || trimmed_cmd == "git";
    
    if is_git_cmd && !auto_yes {
        let safety = check_safety(&command);
        if safety.risk == RiskLevel::Dangerous {
            ui::print_blocked_dangerous(&safety.reason);
            return Ok(None);
        }

        if safety.risk == RiskLevel::Moderate || trimmed_cmd.starts_with("git add") || trimmed_cmd.starts_with("git commit") || trimmed_cmd.starts_with("git push") {
            let res = handle_interactive_git_flow(ai_client, trimmed_cmd, user_input).await?;
            return Ok(res);
        }
    }

    // ── Step 5: Safety Check ────────────────────────────────────────
    let safety = check_safety(&command);

    match safety.risk {
        RiskLevel::Dangerous => {
            // SECURITY: Dangerous commands are ALWAYS blocked, even with --yes.
            // The --yes flag only auto-accepts Moderate commands, never Dangerous.
            ui::print_blocked_dangerous(&safety.reason);
            return Ok(None);
        }
        RiskLevel::Moderate => {
            ui::print_confirm_moderate(&safety.reason, &command);
            if auto_yes {
                // --yes flag: auto-accept Moderate but still show what was run
                println!("  (--yes) Otomatis menyetujui perintah Moderate.");
            } else {
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

    // Show explanation + tip if present (skip when --no-summary flag is set for faster output)
    if !no_summary && !cmd_resp.explanation.is_empty() {
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
    let mut visited = HashSet::new();
    find_recursive(root, &query_lower, 0, max_depth, max_results, only_dirs, only_files, &mut results, &mut visited);
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
    visited: &mut HashSet<std::path::PathBuf>,
) {
    if depth > max_depth || results.len() >= max_results {
        return;
    }

    // FIXED: Symlink loop detection — canonicalize the directory path and skip
    // if we've already visited it. Previously, symlinks pointing to parent
    // directories caused infinite recursion and stack overflow.
    let canonical = match std::fs::canonicalize(dir) {
        Ok(p) => p,
        Err(_) => return,
    };
    if !visited.insert(canonical) {
        return; // Already visited this directory (symlink loop)
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
            find_recursive(&path, query, depth + 1, max_depth, max_results, only_dirs, only_files, results, visited);
        }
    }
}
