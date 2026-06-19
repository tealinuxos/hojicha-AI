/// UI module: Handles all terminal output formatting with colors and styles.
use colored::Colorize;
use std::io::{self, Write};
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

static CURRENT_THEME: AtomicU8 = AtomicU8::new(0); // 0 = Dark (Green), 1 = Light (Cyan)

pub fn set_theme(is_light: bool) {
    CURRENT_THEME.store(if is_light { 1 } else { 0 }, Ordering::SeqCst);
}

pub fn is_light_theme() -> bool {
    CURRENT_THEME.load(Ordering::SeqCst) == 1
}

pub fn color_primary(s: &str) -> colored::ColoredString {
    if is_light_theme() {
        s.truecolor(0, 180, 216) // Blue Cyan
    } else {
        s.truecolor(72, 187, 120) // Mint Green
    }
}

pub fn color_light(s: &str) -> colored::ColoredString {
    if is_light_theme() {
        s.truecolor(72, 202, 228)
    } else {
        s.truecolor(104, 211, 145)
    }
}

pub fn color_lighter(s: &str) -> colored::ColoredString {
    if is_light_theme() {
        s.truecolor(144, 224, 239)
    } else {
        s.truecolor(154, 230, 180)
    }
}

pub fn color_lightest(s: &str) -> colored::ColoredString {
    if is_light_theme() {
        s.truecolor(202, 240, 248)
    } else {
        s.truecolor(198, 246, 213)
    }
}

// ─── Brand (Shown only at startup) ───────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!("{}", color_primary(r#"   __               _ _      _"#));
    println!("{}", color_primary(r#"  / /_  ____  _____(_) /____/ /_  ____ _"#));
    println!("{}", color_light(r#" / __ \/ __ \/ ___/ / / ___/ __ \/ __ `/"#));
    println!("{}", color_lighter(r#"/ / / / /_/ / /__/ / / /__/ / / / /_/ /"#));
    println!("{}", color_lightest(r#"/_/ /_/\____/\___/_/_/\___/_/ /_/\__,_/"#));
    println!();
}

pub fn print_help() {
    println!("{}", color_primary("CARA PENGGUNAAN:").bold());
    println!();
    println!("  {}", "Ketik pertanyaan dalam bahasa Indonesia atau Inggris:".truecolor(200, 200, 200));
    println!();
    println!("  {} {}", color_light("→"), "cek ram laptop saya".italic().bright_white());
    println!("  {} {}", color_light("→"), "lihat file di folder ini".italic().bright_white());
    println!("  {} {}", color_light("→"), "berapa ukuran folder Downloads?".italic().bright_white());
    println!("  {} {}", color_light("→"), "show running processes".italic().bright_white());
    println!("  {} {}", color_light("→"), "cek koneksi internet".italic().bright_white());
    println!();
    println!("{}", color_primary("PERINTAH KHUSUS:").bold());
    println!();
    println!("  {}  - Keluar dari hojicha", "exit / quit / q".truecolor(72, 187, 120));
    println!("  {}       - Tampilkan bantuan ini", "help / ?".truecolor(72, 187, 120));
    println!("  {}    - Tampilkan model yang digunakan", "model".truecolor(72, 187, 120));
    println!("  {}    - Ganti tema warna (dark/light)", "theme".truecolor(72, 187, 120));
    println!("  {}    - Bersihkan riwayat percakapan", "clear".truecolor(72, 187, 120));
    println!("  {}     - Muat info/status .git ke memori percakapan", "/git".truecolor(72, 187, 120));
    println!("  {} - Cari file/folder di Home (~)", "/find <nama>".truecolor(72, 187, 120));
    println!("  {}   - Cari di seluruh laptop", "/find / <nama>".truecolor(72, 187, 120));
    println!("  {}   - Cari di folder saat ini", "/find . <nama>".truecolor(72, 187, 120));
    println!("  {}  - Cari hanya folder saja di Home", "/find folder <nama>".truecolor(72, 187, 120));
    println!("  {}    - Cari hanya file saja di Home", "/find file <nama>".truecolor(72, 187, 120));
    println!();
    println!("{}", "─".repeat(50).truecolor(60, 80, 68));
    println!();
}

// ─── Prompt ──────────────────────────────────────────────────────────────────

pub fn print_prompt() {
    print!("{} ", color_primary("hojicha ❯").bold());
}

// ─── Minimalist Execution UI ─────────────────────────────────────────────────

pub fn print_thinking() {
    // Intentionally silent to keep terminal output clean while waiting for LLM
}

pub fn print_explanation(explanation: &str, tip: Option<&str>) {
    if !explanation.is_empty() {
        println!("  {} {}", color_light("ℹ"), explanation.truecolor(200, 200, 200));
    }
    if let Some(t) = tip {
        println!("  {} {}", "💡".truecolor(251, 191, 36), t.truecolor(180, 180, 180).italic());
    }
}

pub fn print_confirm_moderate(reason: &str, command: &str) {
    println!("  {}", reason.bold().yellow());
    println!("  {} {}", "Perintah:".bold(), command.green());
    print!("  {} ", "Jalankan? (y/n):".dimmed());
}

pub fn print_blocked_dangerous(reason: &str) {
    println!("  {} - {}", "Perintah Diblokir".bold().red(), reason);
}

pub fn print_no_command(explanation: &str) {
    print!("  \x1b[38;2;200;200;200m");
    let mut stdout = io::stdout();
    for c in explanation.chars() {
        print!("{}", c);
        let _ = stdout.flush();
        thread::sleep(Duration::from_millis(10));
    }
    print!("\x1b[0m");
    println!();
}

pub fn print_executing(command: &str) {
    println!("{} {}", color_primary("▶").bold(), color_primary(command));
}

pub fn print_raw_output(output: &str) {
    let trimmed = output.trim_end();
    if !trimmed.is_empty() {
        println!("{}", trimmed);
    }
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

// ─── Status / info ───────────────────────────────────────────────────────────

pub fn print_model_info(model: &str, url: &str) {
    println!();
    println!("  {} {}", color_light("Model:").bold(), model.truecolor(220, 220, 220));
    println!("  {} {}", color_light("Source:").bold(), url.truecolor(220, 220, 220));
    println!();
}

pub fn print_history_cleared() {
    println!("  {}", "Riwayat percakapan dihapus.".dimmed());
}

pub fn print_goodbye() {
    println!();
    println!("  {}", color_primary("Sampai jumpa! Selamat belajar Linux!"));
    println!();
}

pub fn print_error(msg: &str) {
    println!("  {} {}", "⚠ Error:".red().bold(), msg);
}

pub fn print_search_results(query: &str, results: &[String]) {
    println!();
    if results.is_empty() {
        println!(
            "  {} Tidak ada file/folder bernama '{}' ditemukan.",
            "🔍",
            query.bold()
        );
    } else {
        println!(
            "  {} {} hasil untuk '{}':",
            color_light("🔍"),
            results.len().to_string().bold().truecolor(104, 211, 145), // hit count
            query.bold().white()
        );
        println!();
        for path in results {
            println!("  {} {}", color_light("→"), path.truecolor(220, 220, 220));
        }
    }
    println!();
}
