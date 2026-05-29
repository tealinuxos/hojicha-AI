/// BM25 (Best Match 25) full-text retrieval — pure Rust, no external deps.
/// Standard parameters: k1=1.5, b=0.75

use std::collections::HashMap;
use crate::rag::kb::{KbEntry, KnowledgeBase};

const K1: f32 = 1.5;
const B: f32 = 0.75;

#[derive(Debug, Clone)]
pub struct BM25Index {
    /// term → doc_id → term_frequency
    tf: HashMap<String, HashMap<usize, f32>>,
    /// term → number of docs containing it
    df: HashMap<String, usize>,
    /// doc lengths (in tokens)
    doc_lengths: Vec<usize>,
    avg_doc_len: f32,
    num_docs: usize,
}

impl BM25Index {
    pub fn build(kb: &KnowledgeBase) -> Self {
        let mut tf: HashMap<String, HashMap<usize, f32>> = HashMap::new();
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut doc_lengths = Vec::new();

        for (doc_id, entry) in kb.entries.iter().enumerate() {
            let text = entry_to_text(entry);
            let tokens = tokenize(&text);
            doc_lengths.push(tokens.len());

            let mut local_tf: HashMap<String, usize> = HashMap::new();
            for tok in &tokens {
                *local_tf.entry(tok.clone()).or_insert(0) += 1;
            }

            for (term, count) in &local_tf {
                tf.entry(term.clone())
                    .or_default()
                    .insert(doc_id, *count as f32);
                *df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let total_len: usize = doc_lengths.iter().sum();
        let avg_doc_len = if doc_lengths.is_empty() {
            1.0
        } else {
            total_len as f32 / doc_lengths.len() as f32
        };

        Self {
            tf,
            df,
            doc_lengths,
            avg_doc_len,
            num_docs: kb.entries.len(),
        }
    }

    pub fn score(&self, query: &str, doc_id: usize) -> f32 {
        let tokens = tokenize(query);
        let dl = self.doc_lengths.get(doc_id).copied().unwrap_or(1) as f32;
        let n = self.num_docs as f32;
        let mut score = 0.0f32;

        for term in &tokens {
            let df = self.df.get(term).copied().unwrap_or(0) as f32;
            if df == 0.0 {
                continue;
            }
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            let freq = self.tf.get(term)
                .and_then(|d| d.get(&doc_id))
                .copied()
                .unwrap_or(0.0);
            let tf = (freq * (K1 + 1.0)) / (freq + K1 * (1.0 - B + B * dl / self.avg_doc_len));
            score += idf * tf;
        }
        score
    }

    /// Return top-k (doc_id, score) pairs
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(usize, f32)> {
        let mut scores: Vec<(usize, f32)> = (0..self.num_docs)
            .map(|id| (id, self.score(query, id)))
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }
}

/// Convert an entry to indexable text
fn entry_to_text(entry: &KbEntry) -> String {
    let kws = entry.keywords.join(" ");
    format!("{} {} {} {}", entry.command, entry.description, kws, entry.beginner_tip)
}

/// Simple whitespace + punctuation tokenizer with Indonesian stemming hints
pub fn tokenize(text: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "saya", "aku", "kamu", "dia", "mereka", "kita", "kami",
        "yang", "dan", "di", "ke", "dari", "untuk", "dengan", "pada", "adalah", "itu", "ini", "bisa", "ada",
        "yaitu", "yakni", "seperti", "atau", "bahwa", "oleh",
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "with", "by", "of", "about",
        "is", "are", "was", "were", "be", "been", "have", "has", "had", "this", "that", "i", "you", "my", "your"
    ].iter().cloned().collect();

    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1 && !stop_words.contains(s))
        .map(|s| s.to_string())
        .collect()
}
