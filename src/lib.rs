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
use rag::{AiClient, RagPipeline};
use safety::{check_safety, RiskLevel};
use std::io::{self, Write};

// ─── CLI Definition ───────────────────────────────────────────────────────────

/// Hojicha - Asisten Terminal Linux untuk Pemula
#[derive(Parser)]
#[command(name = "hojicha")]
#[command(version = "0.1.0")]
#[command(about = "Asisten Terminal Linux bertenaga AI untuk Pemula", long_about = None)]
#[command(after_help = "Contoh: hojicha \"cek ram laptop saya\"")]
pub struct Cli {
    /// Pertanyaan atau perintah dalam bahasa alami (opsional - tanpa argumen masuk mode interaktif)
    pub query: Option<String>,

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

    // 1. Check in the current working directory
    let cwd_path = std::path::Path::new("src/data").join(&filename);
    if cwd_path.exists() {
        return cwd_path;
    }

    // 2. Fallback to ~/.config/hojicha/knowledge_base_<suffix>.json
    if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home)
            .join(".config")
            .join("hojicha")
            .join(&filename)
    } else {
        std::path::PathBuf::from("src/data").join(&filename)
    }
}

// ─── Main run function ────────────────────────────────────────────────────────

pub async fn run(cli: Cli) -> Result<()> {
    let kb_file_path = resolve_kb_path(cli.kb_path.as_deref());
    // Build RAG pipeline (indexes KB in memory, ~1ms)
    let rag_pipeline = RagPipeline::build(&kb_file_path);

    // Load configuration
    let config = crate::config::LlmConfig::load_or_create()?;

    let mut ai_client = crate::config::load_ai_client_from_config(&config).await?;

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
