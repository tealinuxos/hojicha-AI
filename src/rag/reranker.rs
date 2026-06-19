//! Heuristic reranker — boosts entries based on:
//! 1. Exact keyword match (substring)
//! 2. Tokenized keyword overlap (token-level matching)
//! 3. Category match bonus from query signals
//! 4. Risk penalty (prefer Safe entries)
//! 5. Phrase overlap boost
//! 6. Command name exact hit

use crate::rag::bm25::tokenize;
use crate::rag::kb::{Category, RiskTag};
use crate::rag::retriever::RetrievedEntry;

/// Rerank the retrieved entries and return sorted (best first)
pub fn rerank<'a>(query: &str, mut entries: Vec<RetrievedEntry<'a>>) -> Vec<RetrievedEntry<'a>> {
    let q_lower = query.to_lowercase();
    let q_tokens: std::collections::HashSet<String> = tokenize(&q_lower).into_iter().collect();
    let inferred_category = infer_category(&q_tokens);

    for entry in entries.iter_mut() {
        let mut boost = 0.0f32;

        // 1. Exact keyword substring match bonus
        for kw in &entry.entry.keywords {
            if q_lower.contains(kw) {
                boost += 0.15;
            }
        }

        // 2. Tokenized keyword overlap bonus
        // Tokenize each keyword phrase and check overlap with query tokens.
        // This catches cases like "cek ram" matching query tokens even if
        // the exact phrase "cek ram" isn't a substring of the query.
        let mut kw_token_matches = 0usize;
        let mut kw_token_total = 0usize;
        for kw in &entry.entry.keywords {
            let kw_tokens = tokenize(kw);
            kw_token_total += kw_tokens.len();
            for kt in &kw_tokens {
                if q_tokens.contains(kt) {
                    kw_token_matches += 1;
                }
            }
        }
        if kw_token_total > 0 {
            let overlap_ratio = kw_token_matches as f32 / kw_token_total as f32;
            boost += overlap_ratio * 0.20;
        }

        // 3. Token overlap bonus (description tokens)
        let entry_tokens: std::collections::HashSet<String> = tokenize(&entry.entry.description)
            .into_iter().collect();
        let overlap = q_tokens.intersection(&entry_tokens).count() as f32;
        boost += overlap * 0.05;

        // 4. Category match bonus
        if let Some(cat) = &inferred_category {
            if *cat == entry.entry.category {
                boost += 0.10;
            }
        }

        // 5. Risk penalty — discourage Moderate/Dangerous in general context
        if entry.entry.risk == RiskTag::Moderate {
            boost -= 0.03;
        }
        if entry.entry.risk == RiskTag::Dangerous {
            boost -= 0.20;
        }

        // 6. Command name exact hit
        if !entry.entry.command.is_empty() && q_lower.contains(&entry.entry.command) {
            boost += 0.25;
        }

        entry.score += boost;
    }

    entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    entries
}

/// FIXED: Use token-level matching instead of substring matching to avoid false
/// positives. Previously, "ip" in "tip" would trigger Network, "load" in
/// "download" would trigger CPU, and "free" in "freeware" would trigger Memory.
fn infer_category(query_tokens: &std::collections::HashSet<String>) -> Option<Category> {
    let mem_kws = ["ram", "memori", "memory", "free", "swap"];
    let cpu_kws = ["cpu", "prosesor", "processor", "beban", "load", "core", "suhu", "temp"];
    let disk_kws = ["disk", "penyimpanan", "storage", "ruang", "partisi", "kapasitas"];
    let net_kws = ["internet", "koneksi", "ping", "network", "ip", "port", "jaringan"];
    let proc_kws = ["proses", "process", "pid", "kill", "running", "berjalan"];
    let pkg_kws = ["install", "instal", "apt", "update", "upgrade", "paket", "software"];
    let file_kws = ["file", "folder", "direktori", "ls", "find", "cari", "lihat"];

    // Check if any keyword appears as a whole token in the query
    let check = |kws: &[&str]| kws.iter().any(|&k| query_tokens.contains(k));

    if check(&mem_kws) { Some(Category::Memory) }
    else if check(&cpu_kws) { Some(Category::Cpu) }
    else if check(&disk_kws) { Some(Category::Disk) }
    else if check(&net_kws) { Some(Category::Network) }
    else if check(&proc_kws) { Some(Category::Process) }
    else if check(&pkg_kws) { Some(Category::Package) }
    else if check(&file_kws) { Some(Category::Files) }
    else { None }
}
