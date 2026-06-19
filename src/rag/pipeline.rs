//! Full RAG pipeline orchestrator.
//! Flow: rewrite (keyword-aware) → retrieve → rerank → build context with keywords → LLM

use anyhow::Result;
use std::collections::HashMap;

use crate::rag::client::{AiClient, CommandResponse};
use crate::rag::kb::{KnowledgeBase, KbEntry, RiskTag};
use crate::rag::reranker::rerank;
use crate::rag::retriever::HybridRetriever;

const GREETING_TOKENS: &[&str] = &[
    "halo", "hai", "hello", "hi", "hey",
    "selamat pagi", "selamat siang", "selamat malam", "selamat sore",
    "apa kabar", "gimana kabar", "siapa kamu", "kamu siapa",
    "bisa apa", "kamu bisa apa", "kamu apaan", "hojicha",
];

fn is_greeting(input: &str) -> bool {
    let lower = input.to_lowercase();
    let trimmed = lower.trim();
    GREETING_TOKENS.iter().any(|&g| trimmed == g || trimmed.starts_with(g))
}

fn make_greeting_response() -> CommandResponse {
    let info: serde_json::Value = serde_json::from_str(
        include_str!("../data/general_info.json")
    ).unwrap_or_default();
    
    let kemampuan = info["kemampuan"]
        .as_array()
        .map(|arr| arr.iter()
            .filter_map(|v| v.as_str())
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join(" "))
        .unwrap_or_default();

    CommandResponse {
        command: None,
        explanation: format!(
            "Halo! Saya {}, {}. Berikut yang bisa saya bantu: {}",
            info["nama_asisten"].as_str().unwrap_or("Hojicha"),
            info["peran"].as_str().unwrap_or(""),
            kemampuan
        ),
        beginner_tip: Some(
            info["saran_penggunaan"].as_str().unwrap_or("").to_string()
        ),
        is_safe: true,
    }
}

fn is_dynamic_command(command: &str) -> bool {
    let cmd = command.trim();
    cmd == "cd" ||
    cmd == "cat" ||
    cmd == "rm" ||
    cmd == "mkdir" ||
    cmd == "cp" ||
    cmd == "mv" ||
    cmd == "less" ||
    cmd == "grep" ||
    cmd == "chmod" ||
    cmd == "chown" ||
    cmd == "brew install" ||
    cmd == "brew uninstall" ||
    cmd == "brew search" ||
    cmd.ends_with("grep") ||
    cmd.ends_with("-name") ||
    (cmd.starts_with("git ") && cmd != "git status" && cmd != "git log" && cmd != "git reflog" && cmd != "git remote -v" && cmd != "git diff --staged")
}

fn keyword_preproc<'a>(
    kb: &'a KnowledgeBase,
    input: &str,
) -> Option<&'a KbEntry> {
    let lower = input.to_lowercase();
    let tokens: Vec<&str> = lower.split_whitespace().collect();

    let mut best: Option<(&KbEntry, usize)> = None;

    for entry in &kb.entries {
        let matched = entry.keywords.iter().filter(|kw| {
            let kw_lower = kw.to_lowercase();
            lower.contains(kw_lower.as_str())
        }).count();

        if matched == 0 { continue; }

        let coverage = {
            let kw_tokens: usize = entry.keywords.iter()
                .filter(|kw| lower.contains(kw.to_lowercase().as_str()))
                .map(|kw| kw.split_whitespace().count())
                .sum();
            kw_tokens as f32 / tokens.len().max(1) as f32
        };

        // FIXED: Stricter thresholds to prevent over-matching on short queries.
        // Previously a 1-word query could match with just 1 keyword hit (coverage 1.0),
        // causing false positives (e.g., typing "cek" matching an unrelated entry).
        let min_coverage = if tokens.len() <= 2 { 0.9 } else { 0.4 };
        let min_matched = if tokens.len() <= 1 { 2 } else { 1 };
        if coverage >= min_coverage && matched >= min_matched {
            if best.map_or(true, |(_, m)| matched > m) {
                best = Some((entry, matched));
            }
        }
    }

    best.map(|(entry, _)| entry)
}

/// Singleton-style pipeline — built once, reused across queries
pub struct RagPipeline {
    pub kb: KnowledgeBase,
    retriever: HybridRetriever,
    /// Keyword synonym map: keyword → related keywords from same KB entry.
    /// Used for query expansion so that "cek ram" also searches for "memori", "free", etc.
    synonym_map: HashMap<String, Vec<String>>,
}

impl RagPipeline {
    /// Build the pipeline. Indexes KB in memory (fast, ~1ms).
    pub fn build(kb_path: &std::path::Path) -> Self {
        let kb = KnowledgeBase::load(kb_path);
        let retriever = HybridRetriever::build(&kb);
        let synonym_map = Self::build_synonym_map(&kb);
        Self { kb, retriever, synonym_map }
    }

    /// Run the full RAG pipeline for a given query.
    /// Returns a CommandResponse enriched with KB context.
    /// `history` is the conversation history for multi-turn context.
    pub async fn run(
        &self,
        ai_client: &mut AiClient,
        user_input: &str,
        history: &[(String, String)],
    ) -> Result<CommandResponse> {
        // 1. Preprocess: Check for greeting
        if is_greeting(user_input) {
            return Ok(make_greeting_response());
        }

        // 2. Preprocess: Check for exact keyword match in KB
        if let Some(entry) = keyword_preproc(&self.kb, user_input) {
            if !is_dynamic_command(&entry.command) {
                return Ok(CommandResponse {
                    command: Some(entry.command.clone()),
                    explanation: entry.description.clone(),
                    beginner_tip: Some(entry.beginner_tip.clone()),
                    is_safe: matches!(entry.risk, RiskTag::Safe | RiskTag::Moderate),
                });
            }
        }

        // 3. Rewrite query using KB keyword synonym expansion
        let rewritten = self.rewrite_query(user_input);

        // 2. Hybrid retrieval
        let hits = self.retriever.retrieve(&rewritten, &self.kb, 6);

        // 3. Rerank (with keyword boost)
        let ranked = rerank(&rewritten, hits);

        // 4. Build context from top-3 entries — keywords included for few-shot
        let top3: Vec<_> = ranked.into_iter().take(3).collect();
        let mut context = build_context(&top3);

        // 5. Append general assistant info to context so the model always has its persona context
        let info_str = include_str!("../data/general_info.json");
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        context.push_str("INFORMASI UMUM ASISTEN (Gunakan ini untuk menjawab sapaan/pertanyaan tentang diri Anda secara natural):\n");
        context.push_str(info_str);

        // 6. Build RAG-augmented system prompt (with keyword-driven few-shot)
        let os_name = if cfg!(target_os = "macos") { "macOS" } else { "Linux" };
        let system = build_rag_system_prompt(&context, os_name);

        // 7. Build user input with conversation history for multi-turn context
        // FIXED: Previously, history was accepted as a parameter but never passed
        // to the LLM, so every query was treated as a fresh conversation.
        let augmented_input = if history.is_empty() {
            user_input.to_string()
        } else {
            let mut input_with_history = String::new();
            // Include last 5 turns of conversation history
            // SECURITY: Sanitize history entries to prevent prompt injection.
            // Strip newlines and control characters that could manipulate the prompt.
            for (user_msg, assistant_msg) in history.iter().rev().take(5).rev() {
                let safe_user = user_msg.chars()
                    .filter(|c| !c.is_control() || *c == ' ')
                    .take(500)  // Limit length to prevent prompt overflow
                    .collect::<String>();
                let safe_assistant = assistant_msg.chars()
                    .filter(|c| !c.is_control() || *c == ' ')
                    .take(500)
                    .collect::<String>();
                input_with_history.push_str(&format!(
                    "[Previous] User: {}\n[Previous] Assistant: {}\n\n",
                    safe_user, safe_assistant
                ));
            }
            input_with_history.push_str(&format!("[Current] User: {}", user_input));
            input_with_history
        };

        // 8. Call LLM
        let response = ai_client.nl_to_command(&system, &augmented_input).await?;
        Ok(response)
    }

    /// Build a synonym map from KB entries.
    /// Each keyword in an entry maps to all OTHER keywords in the same entry.
    /// This enables keyword-aware query expansion: if user says "ram",
    /// the query is expanded with "memori", "memory", "free", "cek ram", etc.
    fn build_synonym_map(kb: &KnowledgeBase) -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();

        for entry in &kb.entries {
            let kws = &entry.keywords;
            for (i, kw) in kws.iter().enumerate() {
                let related: Vec<String> = kws.iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, v)| v.clone())
                    .collect();

                map.entry(kw.to_lowercase())
                    .or_default()
                    .extend(related);
            }
        }

        // Deduplicate each keyword's synonym list
        for synonyms in map.values_mut() {
            synonyms.sort();
            synonyms.dedup();
        }

        map
    }

    /// Keyword-aware query rewriter.
    /// Expands user input using the synonym map built from KB keywords.
    fn rewrite_query(&self, input: &str) -> String {
        let s = input.to_lowercase();
        let mut expansions: Vec<String> = Vec::new();

        // For each token in the query, check if it matches any KB keyword
        let tokens: Vec<&str> = s.split_whitespace().collect();
        for token in &tokens {
            let normalized = token.trim_matches(|c: char| !c.is_alphanumeric());
            if let Some(synonyms) = self.synonym_map.get(normalized) {
                // Add up to 4 synonyms per keyword to avoid query bloat
                for syn in synonyms.iter().take(4) {
                    if !s.contains(syn.as_str()) {
                        expansions.push(syn.clone());
                    }
                }
            }
        }

        if expansions.is_empty() {
            s
        } else {
            format!("{} {}", s, expansions.join(" "))
        }
    }
}

/// Build rich context from retrieved entries — includes keywords as few-shot hints
/// so the LLM can see what user phrasings map to which commands.
fn build_context(entries: &[crate::rag::retriever::RetrievedEntry<'_>]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut ctx = String::from(
        "Referensi perintah Linux yang relevan (gunakan keyword untuk mencocokkan intent pengguna):\n"
    );
    for (i, e) in entries.iter().enumerate() {
        let keywords_str = e.entry.keywords.join(", ");
        let risk_str = match e.entry.risk {
            crate::rag::kb::RiskTag::Safe => "Aman",
            crate::rag::kb::RiskTag::Moderate => "Perlu perhatian",
            crate::rag::kb::RiskTag::Dangerous => "Berbahaya",
        };
        ctx.push_str(&format!(
            "\n[{}] Perintah: `{}`\n    Kategori: {:?}\n    Tingkat Risiko: {}\n    Keyword yang cocok: {}\n    Keterangan: {}\n    Contoh: `{}`\n    Tips: {}\n",
            i + 1,
            e.entry.command,
            e.entry.category,
            risk_str,
            keywords_str,
            e.entry.description,
            e.entry.example,
            e.entry.beginner_tip,
        ));
    }
    ctx
}

fn build_rag_system_prompt(context: &str, os_name: &str) -> String {
    let base = crate::rag::system_prompt(os_name);
    if context.is_empty() {
        return base;
    }
    format!("{}\n\n---\n{}", base, context)
}
