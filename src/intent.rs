/// Intent classifier — rule-based keyword matching.
/// Runs before RAG/LLM to identify structured intents that
/// can be handled by the rule engine (fast path).

#[derive(Debug, Clone, PartialEq)]
pub enum SystemMetric {
    Ram,
    Cpu,
    Disk,
    Network,
    Temperature,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessAction {
    List,
    Kill(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// Monitoring resource usage
    CheckSystem(SystemMetric),
    /// Listing files in optional path
    ListFiles(Option<String>),
    /// Finding a file by name/pattern
    FindFile(String),
    /// Process management
    ManageProcess(ProcessAction),
    /// Installing software
    InstallPackage(String),
    /// Show current directory
    WhereAmI,
    /// Show system info (distro, kernel)
    SystemInfo,
    /// Show command history
    ShowHistory,
    /// Greeting, help, or info queries
    Greeting,
    /// General — fallback to RAG + LLM
    General,
}

/// Classify user input into a structured intent.
pub fn classify(input: &str) -> Intent {
    let s = input.to_lowercase();

    // ── Greetings & Info ─────────────────────────────────────────────
    if contains_any(&s, &["halo", "hello", "helo", "hai", "hi", "siapa kamu", "kamu siapa", "bisa apa", "bantuan", "help", "apa kabar", "apa yang bisa kamu lakukan", "apa itu hojicha", "tentang hojicha", "tentang kamu", "siapa dirimu", "kemampuan", "fitur", "kelebihan", "cara pakai", "cara menggunakan", "panduan"]) {
        return Intent::Greeting;
    }

    // ── Process management ───────────────────────────────────────────
    if contains_any(&s, &["kill", "hentikan proses", "matikan proses", "stop proses"]) {
        // Try to extract target name
        let target = extract_after(&s, &["kill", "hentikan", "matikan", "stop"]);
        if let Some(t) = target {
            return Intent::ManageProcess(ProcessAction::Kill(t));
        }
    }
    if contains_any(&s, &["daftar proses", "lihat proses", "ps aux", "proses apa yang berjalan"]) {
        return Intent::ManageProcess(ProcessAction::List);
    }

    // ── System metric ────────────────────────────────────────────────
    if contains_any(&s, &["ram", "memori", "memory", "konsumsi memori", "sisa ram", "penggunaan ram", "cek ram", "ram saya"]) {
        return Intent::CheckSystem(SystemMetric::Ram);
    }
    if contains_any(&s, &["cpu", "prosesor", "processor", "beban cpu", "penggunaan cpu", "load cpu"]) {
        return Intent::CheckSystem(SystemMetric::Cpu);
    }
    if contains_any(&s, &["disk", "penyimpanan", "ruang disk", "storage", "kapasitas disk", "sisa disk", "cek disk"]) {
        return Intent::CheckSystem(SystemMetric::Disk);
    }
    if contains_any(&s, &["internet", "koneksi", "ping", "cek internet", "konek", "jaringan", "network"]) {
        return Intent::CheckSystem(SystemMetric::Network);
    }
    if contains_any(&s, &["suhu", "temperature", "panas", "thermal"]) {
        return Intent::CheckSystem(SystemMetric::Temperature);
    }

    // ── Files ────────────────────────────────────────────────────────
    if contains_any(&s, &["di mana saya", "posisi saya", "folder sekarang", "lokasi saya", "pwd", "current directory", "dimana saya", "dimana"]) {
        return Intent::WhereAmI;
    }
    if contains_any(&s, &["cari file", "temukan file", "find file", "mencari file"]) {
        let pattern = extract_after(&s, &["cari file", "temukan file", "find file"]);
        return Intent::FindFile(pattern.unwrap_or_else(|| "*".to_string()));
    }
    if contains_any(&s, &[
        "lihat file", "list file", "isi folder", "tampilkan file", "ls", "daftar file",
        "apa aja file", "apa saja file", "file apa saja", "file apa aja", "isi direktori",
        "ada file apa"
    ]) {
        let path = extract_path(&s);
        return Intent::ListFiles(path);
    }

    // ── System info ──────────────────────────────────────────────────
    if contains_any(&s, &["versi linux", "distro", "sistem operasi", "os version", "uname", "info sistem"]) {
        return Intent::SystemInfo;
    }

    // ── History ─────────────────────────────────────────────────────
    if contains_any(&s, &["history", "riwayat", "perintah sebelumnya", "histori terminal"]) {
        return Intent::ShowHistory;
    }

    // ── Package install ──────────────────────────────────────────────
    if contains_any(&s, &["install", "instal", "pasang"]) {
        let pkg = extract_after(&s, &["install", "instal", "pasang"]);
        if let Some(p) = pkg {
            return Intent::InstallPackage(p);
        }
    }

    Intent::General
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn extract_after(s: &str, triggers: &[&str]) -> Option<String> {
    for trigger in triggers {
        if let Some(pos) = s.find(trigger) {
            let rest = s[pos + trigger.len()..].trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn extract_path(s: &str) -> Option<String> {
    if s.contains("di path ini") || s.contains("di sini") || s.contains("di folder ini") || s.contains("di direktori ini") {
        return None;
    }
    for word in s.split_whitespace() {
        if word.starts_with('/') || word.starts_with("./") || word.starts_with("../") || word.contains('/') {
            let clean = word.trim_matches(|c| c == '?' || c == '!' || c == '.' || c == ',');
            if !clean.is_empty() {
                return Some(clean.to_string());
            }
        }
    }
    None
}
