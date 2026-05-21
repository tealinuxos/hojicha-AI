/// UI module: Handles all terminal output formatting with colors and styles.
use colored::Colorize;

// ─── Brand (Shown only at startup) ───────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!("{}", "  ████████╗███████╗██████╗  █████╗ ".truecolor(99, 179, 237));
    println!("{}", "     ██╔══╝██╔════╝██╔══██╗██╔══██╗".truecolor(99, 179, 237));
    println!("{}", "     ██║   █████╗  ██████╔╝███████║".truecolor(129, 199, 247));
    println!("{}", "     ██║   ██╔══╝  ██╔══██╗██╔══██║".truecolor(159, 219, 255));
    println!("{}", "     ██║   ███████╗██║  ██║██║  ██║".truecolor(189, 224, 255));
    println!("{}", "     ╚═╝   ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝".truecolor(189, 224, 255));
    println!();
    println!("  {}  {}", "🤖".bright_white(), "Asisten Terminal Linux untuk Pemula".truecolor(200, 200, 200));
    println!("  {}  {}", "🔒".bright_white(), "Aman · Lokal · Offline".truecolor(134, 239, 172));
    println!();
    println!("{}", "─".repeat(50).truecolor(60, 60, 80));
    println!();
}

pub fn print_help() {
    println!("{}", "📌 CARA PENGGUNAAN:".bold().truecolor(250, 204, 21));
    println!();
    println!("  {}", "Ketik pertanyaan dalam bahasa Indonesia atau Inggris:".truecolor(200, 200, 200));
    println!();
    println!("  {} {}", "→".truecolor(99, 179, 237), "cek ram laptop saya".italic().bright_white());
    println!("  {} {}", "→".truecolor(99, 179, 237), "lihat file di folder ini".italic().bright_white());
    println!("  {} {}", "→".truecolor(99, 179, 237), "berapa ukuran folder Downloads?".italic().bright_white());
    println!("  {} {}", "→".truecolor(99, 179, 237), "show running processes".italic().bright_white());
    println!("  {} {}", "→".truecolor(99, 179, 237), "cek koneksi internet".italic().bright_white());
    println!();
    println!("{}", "📟 PERINTAH KHUSUS:".bold().truecolor(250, 204, 21));
    println!();
    println!("  {}  - Keluar dari TERA", "exit / quit / q".truecolor(99, 179, 237));
    println!("  {}       - Tampilkan bantuan ini", "help / ?".truecolor(99, 179, 237));
    println!("  {}    - Tampilkan model yang digunakan", "model".truecolor(99, 179, 237));
    println!("  {}    - Bersihkan riwayat percakapan", "clear".truecolor(99, 179, 237));
    println!();
    println!("{}", "─".repeat(50).truecolor(60, 60, 80));
    println!();
}

// ─── Prompt ──────────────────────────────────────────────────────────────────

pub fn print_prompt() {
    print!("{} ", "tera ❯".truecolor(99, 179, 237).bold());
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

pub fn print_explanation(_explanation: &str) {
    // Do nothing
}

pub fn print_tip(_tip: &str) {
    // Do nothing
}

pub fn print_confirm_moderate() {
    print!("  {} {}\n  {} ", "⚠️".yellow(), "Perintah ini memerlukan konfirmasi.".bold(), "Jalankan? (y/n):".dimmed());
}

pub fn print_blocked_dangerous(reason: &str) {
    println!("  {} {} - {}", "🚫".red(), "Perintah Diblokir".bold().red(), reason);
}

pub fn print_no_command(explanation: &str) {
    println!("  {} {}", "ℹ️".blue(), explanation);
}

pub fn print_executing(command: &str) {
    println!("{} {}", "▶".green().bold(), command.cyan());
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
    println!("  {} {} (exit code: {})", "✗".red(), "Perintah gagal".red(), exit_code);
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
    println!("  {} {}", "🤖 Model:".bold().truecolor(147, 197, 253), model.truecolor(220, 220, 220));
    println!("  {} {}", "🌐 Source:".bold().truecolor(147, 197, 253), url.truecolor(220, 220, 220));
    println!();
}

pub fn print_history_cleared() {
    println!("  {} {}", "🧹", "Riwayat percakapan dihapus.".dimmed());
}

pub fn print_goodbye() {
    println!();
    println!("  {} {}", "👋", "Sampai jumpa! Selamat belajar Linux!".green());
    println!();
}

pub fn print_error(msg: &str) {
    println!("  {} {}", "⚠ Error:".red().bold(), msg);
}
