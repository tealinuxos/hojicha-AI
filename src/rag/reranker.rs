/// Heuristic reranker — boosts entries based on:
/// 1. Exact keyword match
/// 2. Category match bonus from query signals
/// 3. Risk penalty (prefer Safe entries)
/// 4. Phrase overlap boost

use crate::rag::bm25::tokenize;
use crate::rag::kb::{Category, RiskTag};
use crate::rag::retriever::RetrievedEntry;

/// Rerank the retrieved entries and return sorted (best first)
pub fn rerank<'a>(query: &str, mut entries: Vec<RetrievedEntry<'a>>) -> Vec<RetrievedEntry<'a>> {
    let q_lower = query.to_lowercase();
    let q_tokens: std::collections::HashSet<String> = tokenize(&q_lower).into_iter().collect();
    let inferred_category = infer_category(&q_lower);

    for entry in entries.iter_mut() {
        let mut boost = 0.0f32;

        // 1. Exact keyword match bonus
        for kw in &entry.entry.keywords {
            if q_lower.contains(kw) {
                boost += 0.15;
            }
        }

        // 2. Token overlap bonus
        let entry_tokens: std::collections::HashSet<String> = tokenize(&entry.entry.description)
            .into_iter().collect();
        let overlap = q_tokens.intersection(&entry_tokens).count() as f32;
        boost += overlap * 0.05;

        // 3. Category match bonus
        if let Some(cat) = &inferred_category {
            if *cat == entry.entry.category {
                boost += 0.10;
            }
        }

        // 4. Risk penalty — discourage Moderate in general context
        if entry.entry.risk == RiskTag::Moderate {
            boost -= 0.03;
        }
        if entry.entry.risk == RiskTag::Dangerous {
            boost -= 0.20;
        }

        // 5. Command name exact hit
        if q_lower.contains(&entry.entry.command) {
            boost += 0.25;
        }

        entry.score += boost;
    }

    entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    entries
}

fn infer_category(query: &str) -> Option<Category> {
    let mem_kws = ["ram", "memori", "memory", "free", "swap"];
    let cpu_kws = ["cpu", "prosesor", "processor", "beban", "load", "core"];
    let disk_kws = ["disk", "penyimpanan", "storage", "ruang", "partisi", "kapasitas"];
    let net_kws = ["internet", "koneksi", "ping", "network", "ip", "port", "jaringan"];
    let proc_kws = ["proses", "process", "pid", "kill", "running", "berjalan"];
    let pkg_kws = ["install", "instal", "apt", "update", "upgrade", "paket", "software"];
    let file_kws = ["file", "folder", "direktori", "ls", "find", "cari", "lihat"];

    let check = |kws: &[&str]| kws.iter().any(|&k| query.contains(k));

    if check(&mem_kws) { Some(Category::Memory) }
    else if check(&cpu_kws) { Some(Category::Cpu) }
    else if check(&disk_kws) { Some(Category::Disk) }
    else if check(&net_kws) { Some(Category::Network) }
    else if check(&proc_kws) { Some(Category::Process) }
    else if check(&pkg_kws) { Some(Category::Package) }
    else if check(&file_kws) { Some(Category::Files) }
    else { None }
}
