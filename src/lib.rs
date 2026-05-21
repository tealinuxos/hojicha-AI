/// Tera-AI: Lightweight AI-powered Linux CLI assistant for beginners.
/// Accepts natural language in Indonesian/English, generates safe Linux commands,
/// executes them, and explains output in beginner-friendly language.

pub mod executor;
pub mod prompt;
pub mod rag;
pub mod safety;
pub mod ui;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use executor::execute_command;
use prompt::system_prompt;
use rag::{AiClient, LocalModelClient, OllamaClient};
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

    /// URL server Ollama
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// Nama model Ollama yang akan digunakan
    #[arg(short, long, default_value = "qwen2.5:1.5b")]
    pub model: String,

    /// Gunakan model built-in offline lokal (tanpa memerlukan Ollama)
    #[arg(short, long, default_value_t = false)]
    pub local: bool,

    /// Jangan tampilkan ringkasan output AI (lebih cepat)
    #[arg(long, default_value_t = false)]
    pub no_summary: bool,

    /// Jalankan tanpa konfirmasi (langsung eksekusi)
    #[arg(short = 'y', long, default_value_t = false)]
    pub yes: bool,
}

// ─── Main run function ────────────────────────────────────────────────────────

pub async fn run(cli: Cli) -> Result<()> {
    let mut ai_client = if cli.local {
        // Force loading local built-in model
        AiClient::Local(LocalModelClient::load_built_in()?)
    } else {
        // Attempt to connect to Ollama. If it fails, fallback to local model.
        let ollama = OllamaClient::new(&cli.ollama_url, &cli.model);
        if ollama.ping().await {
            AiClient::Ollama(ollama)
        } else {
            println!(
                "⚠️  {} {}",
                "Tidak bisa terhubung ke Ollama.".yellow().bold(),
                "Mengaktifkan model built-in offline lokal...".yellow()
            );
            println!();
            AiClient::Local(LocalModelClient::load_built_in()?)
        }
    };

    // Single-query mode (non-interactive)
    if let Some(query) = &cli.query {
        return run_single_query(&mut ai_client, query, cli.no_summary, cli.yes).await;
    }

    // Interactive REPL mode
    run_interactive(&mut ai_client, cli.no_summary, cli.yes).await
}

// ─── Single Query Mode ────────────────────────────────────────────────────────

async fn run_single_query(
    ai_client: &mut AiClient,
    query: &str,
    no_summary: bool,
    auto_yes: bool,
) -> Result<()> {
    process_query(ai_client, query, no_summary, auto_yes, &[]).await?;
    Ok(())
}

// ─── Interactive REPL Mode ────────────────────────────────────────────────────

async fn run_interactive(
    ai_client: &mut AiClient,
    no_summary: bool,
    auto_yes: bool,
) -> Result<()> {
    ui::print_banner();

    match ai_client {
        AiClient::Ollama(ollama) => {
            println!(
                "  {} {}",
                "Terhubung ke Ollama".green().bold(),
                format!("(model: {})", ollama.model).dimmed()
            );
        }
        AiClient::Local(_) => {
            println!(
                "  {} {}",
                "model built-in lokal".green().bold(),
                "(SmolLM2-135M · Offline)".dimmed()
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

    // Conversation history: (user_input, ai_response_json)
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
                match ai_client {
                    AiClient::Ollama(ollama) => {
                        ui::print_model_info(&ollama.model, &ollama.base_url);
                    }
                    AiClient::Local(_) => {
                        ui::print_model_info("SmolLM2-135M-Instruct (Quantized)", "Embedded (Candle)");
                    }
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

        match process_query(ai_client, &input, no_summary, auto_yes, &history).await {
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
    user_input: &str,
    _no_summary: bool,
    auto_yes: bool,
    _history: &[(String, String)],
) -> Result<Option<String>> {
    ui::print_thinking();

    let sys = system_prompt();
    let ai_response = ai_client.nl_to_command(&sys, user_input).await;

    let cmd_resp = match ai_response {
        Ok(r) => r,
        Err(e) => {
            ui::print_error(&format!("AI error: {}", e));
            return Ok(None);
        }
    };

    // Case 1: No command suggested
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

    // Case 3: Safety check
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
                    "y" | "yes" | "ya" => {} // proceed
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

    // Case 4: Execute the command
    ui::print_executing(&command);

    let output = match execute_command(&command) {
        Ok(o) => o,
        Err(e) => {
            ui::print_error(&format!("Gagal menjalankan perintah: {}", e));
            return Ok(None);
        }
    };

    // Show raw output directly (unboxed)
    ui::print_raw_output(&output.stdout);
    if !output.success && !output.stderr.is_empty() {
        ui::print_command_failed(&output.stderr, output.exit_code);
    }

    let history_entry = serde_json::to_string(&cmd_resp.command).unwrap_or_default();
    Ok(Some(history_entry))
}
