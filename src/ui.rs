/// UI module: Handles all terminal output formatting with colors and styles.
use colored::Colorize;

// ─── Brand (Shown only at startup) ───────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!("{}", r#"   __               _ _      _"#.truecolor(72, 187, 120));
    println!("{}", r#"  / /_  ____  _____(_) /____/ /_  ____ _"#.truecolor(72, 187, 120));
    println!("{}", r#" / __ \/ __ \/ ___/ / / ___/ __ \/ __ `/"#.truecolor(104, 211, 145));
    println!("{}", r#"/ / / / /_/ / /__/ / / /__/ / / / /_/ /"#.truecolor(154, 230, 180));
    println!("{}", r#"/_/ /_/\____/\___/_/_/\___/_/ /_/\__,_/"#.truecolor(198, 246, 213));
    println!();
}

pub fn print_help() {
    println!("{}", "CARA PENGGUNAAN:".bold().truecolor(72, 187, 120));
    println!();
    println!("  {}", "Ketik pertanyaan dalam bahasa Indonesia atau Inggris:".truecolor(200, 200, 200));
    println!();
    println!("  {} {}", "→".truecolor(104, 211, 145), "cek ram laptop saya".italic().bright_white());
    println!("  {} {}", "→".truecolor(104, 211, 145), "lihat file di folder ini".italic().bright_white());
    println!("  {} {}", "→".truecolor(104, 211, 145), "berapa ukuran folder Downloads?".italic().bright_white());
    println!("  {} {}", "→".truecolor(104, 211, 145), "show running processes".italic().bright_white());
    println!("  {} {}", "→".truecolor(104, 211, 145), "cek koneksi internet".italic().bright_white());
    println!();
    println!("{}", "PERINTAH KHUSUS:".bold().truecolor(72, 187, 120));
    println!();
    println!("  {}  - Keluar dari hojicha", "exit / quit / q".truecolor(72, 187, 120));
    println!("  {}       - Tampilkan bantuan ini", "help / ?".truecolor(72, 187, 120));
    println!("  {}    - Tampilkan model yang digunakan", "model".truecolor(72, 187, 120));
    println!("  {}    - Bersihkan riwayat percakapan", "clear".truecolor(72, 187, 120));
    println!();
    println!("{}", "─".repeat(50).truecolor(60, 80, 68));
    println!();
}

// ─── Prompt ──────────────────────────────────────────────────────────────────

pub fn print_prompt() {
    print!("{} ", "hojicha ❯".truecolor(72, 187, 120).bold());
}

// ─── Minimalist Execution UI ─────────────────────────────────────────────────

pub fn print_thinking() {
    // Print nothing to keep the terminal output clean and silent while waiting
}

pub fn print_section_divider() {
    // Do nothing to keep layout compact and clean
}

pub fn print_command_proposal(_command: &str) {
    // Do nothing (handled directly by print_executing)
}

pub fn print_explanation(explanation: &str, tip: Option<&str>) {
    if !explanation.is_empty() {
        println!("  {} {}", "ℹ".truecolor(104, 211, 145), explanation.truecolor(200, 200, 200));
    }
    if let Some(t) = tip {
        println!("  {} {}", "💡".truecolor(251, 191, 36), t.truecolor(180, 180, 180).italic());
    }
}

pub fn print_tip(_tip: &str) {
    // Do nothing
}

pub fn print_confirm_moderate() {
    print!("  {}\n  {} ", "Perintah ini memerlukan konfirmasi.".bold().yellow(), "Jalankan? (y/n):".dimmed());
}

pub fn print_blocked_dangerous(reason: &str) {
    println!("  {} - {}", "Perintah Diblokir".bold().red(), reason);
}

pub fn print_no_command(explanation: &str) {
    println!("  {}", explanation);
}

pub fn print_executing(command: &str) {
    println!("{} {}", "▶".green().bold(), command.green());
}

pub fn print_raw_output(output: &str) {
    let trimmed = output.trim_end();
    if !trimmed.is_empty() {
        println!("{}", trimmed);
    }
}

pub fn print_output_summary(_summary: &str, _key_info: &str, _next: Option<&str>) {
    // Do nothing to maintain minimal terminal layout
}

pub fn print_command_failed(stderr: &str, exit_code: i32) {
    println!("  {} (exit code: {})", "Perintah gagal".red(), exit_code);
    if !stderr.is_empty() {
        println!("  {}", stderr.trim().red());
    }
}

pub fn print_cancelled() {
    println!("  {}", "Dibatalkan.".dimmed());
}

pub fn print_skipped() {
    println!("  {}", "Perintah tidak dijalankan.".dimmed());
}

// ─── Status / info ───────────────────────────────────────────────────────────

pub fn print_model_info(model: &str, url: &str) {
    println!();
    println!("  {} {}", "Model:".bold().truecolor(104, 211, 145), model.truecolor(220, 220, 220));
    println!("  {} {}", "Source:".bold().truecolor(104, 211, 145), url.truecolor(220, 220, 220));
    println!();
}

pub fn print_history_cleared() {
    println!("  {}", "Riwayat percakapan dihapus.".dimmed());
}

pub fn print_goodbye() {
    println!();
    println!("  {}", "Sampai jumpa! Selamat belajar Linux!".green());
    println!();
}

pub fn print_error(msg: &str) {
    println!("  {} {}", "⚠ Error:".red().bold(), msg);
}
