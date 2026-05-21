/// Lightweight TF-IDF vector embedder — pure Rust.
/// Builds a vocabulary from KB corpus, converts queries and entries
/// to sparse TF-IDF vectors, computes cosine similarity.

use std::collections::HashMap;
use crate::rag::kb::KnowledgeBase;
use crate::rag::bm25::tokenize;

#[derive(Clone)]
pub struct TfIdfIndex {
    /// vocab: term → index
    vocab: HashMap<String, usize>,
    /// idf weights per term index
    idf: Vec<f32>,
    /// doc vectors: doc_id → sparse (term_idx, weight)
    doc_vecs: Vec<Vec<(usize, f32)>>,
}

impl TfIdfIndex {
    pub fn build(kb: &KnowledgeBase) -> Self {
        let docs: Vec<Vec<String>> = kb.entries.iter()
            .map(|e| {
                let text = format!("{} {} {} {}",
                    e.command, e.description,
                    e.keywords.join(" "), e.beginner_tip);
                tokenize(&text)
            })
            .collect();

        // Build vocabulary
        let mut vocab: HashMap<String, usize> = HashMap::new();
        for doc in &docs {
            for term in doc {
                let n = vocab.len();
                vocab.entry(term.clone()).or_insert(n);
            }
        }

        let vocab_size = vocab.len();
        let n_docs = docs.len() as f32;

        // Document frequency per term
        let mut df = vec![0usize; vocab_size];
        for doc in &docs {
            let unique: std::collections::HashSet<&String> = doc.iter().collect();
            for term in unique {
                if let Some(&idx) = vocab.get(term) {
                    df[idx] += 1;
                }
            }
        }

        // IDF: log((N+1)/(df+1)) + 1  (smooth)
        let idf: Vec<f32> = df.iter()
            .map(|&d| ((n_docs + 1.0) / (d as f32 + 1.0)).ln() + 1.0)
            .collect();

        // Build doc TF-IDF vectors (L2-normalized)
        let doc_vecs: Vec<Vec<(usize, f32)>> = docs.iter().map(|doc| {
            let mut tf: HashMap<usize, f32> = HashMap::new();
            for term in doc {
                if let Some(&idx) = vocab.get(term) {
                    *tf.entry(idx).or_insert(0.0) += 1.0;
                }
            }
            let len = doc.len().max(1) as f32;
            let mut vec: Vec<(usize, f32)> = tf.into_iter()
                .map(|(idx, count)| (idx, (count / len) * idf[idx]))
                .collect();
            l2_normalize_sparse(&mut vec);
            vec
        }).collect();

        Self { vocab, idf, doc_vecs }
    }

    /// Compute cosine similarity between a query and a document
    pub fn similarity(&self, query: &str, doc_id: usize) -> f32 {
        let query_vec = self.embed_query(query);
        let doc_vec = match self.doc_vecs.get(doc_id) {
            Some(v) => v,
            None => return 0.0,
        };
        dot_sparse(&query_vec, doc_vec)
    }

    fn embed_query(&self, query: &str) -> Vec<(usize, f32)> {
        let tokens = tokenize(query);
        let mut tf: HashMap<usize, f32> = HashMap::new();
        for term in &tokens {
            if let Some(&idx) = self.vocab.get(term) {
                *tf.entry(idx).or_insert(0.0) += 1.0;
            }
        }
        let len = tokens.len().max(1) as f32;
        let mut vec: Vec<(usize, f32)> = tf.into_iter()
            .map(|(idx, count)| (idx, (count / len) * self.idf[idx]))
            .collect();
        l2_normalize_sparse(&mut vec);
        vec
    }

    /// Return top-k (doc_id, score) by cosine similarity
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(usize, f32)> {
        let q = self.embed_query(query);
        if q.is_empty() {
            return vec![];
        }

        let mut scores: Vec<(usize, f32)> = self.doc_vecs.iter()
            .enumerate()
            .map(|(id, dv)| (id, dot_sparse(&q, dv)))
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }
}

fn l2_normalize_sparse(vec: &mut Vec<(usize, f32)>) {
    let norm: f32 = vec.iter().map(|(_, w)| w * w).sum::<f32>().sqrt();
    if norm > 1e-9 {
        for (_, w) in vec.iter_mut() {
            *w /= norm;
        }
    }
}

fn dot_sparse(a: &[(usize, f32)], b: &[(usize, f32)]) -> f32 {
    // Hash-based dot product for small sparse vectors
    let mut dot = 0.0f32;
    let b_map: HashMap<usize, f32> = b.iter().cloned().collect();
    for &(idx, wa) in a {
        if let Some(&wb) = b_map.get(&idx) {
            dot += wa * wb;
        }
    }
    dot
}
