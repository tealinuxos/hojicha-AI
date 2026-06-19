//! Hybrid retriever: fuses BM25 (keyword) + TF-IDF (semantic) scores
//! using Reciprocal Rank Fusion (RRF).

use crate::rag::bm25::BM25Index;
use crate::rag::embedder::TfIdfIndex;
use crate::rag::kb::{KbEntry, KnowledgeBase};

const RRF_K: f32 = 60.0;

#[derive(Debug, Clone)]
pub struct RetrievedEntry<'a> {
    pub entry: &'a KbEntry,
    pub doc_id: usize,
    pub score: f32,
    pub bm25_rank: Option<usize>,
    pub tfidf_rank: Option<usize>,
}

pub struct HybridRetriever {
    pub bm25: BM25Index,
    pub tfidf: TfIdfIndex,
}

impl HybridRetriever {
    pub fn build(kb: &KnowledgeBase) -> Self {
        Self {
            bm25: BM25Index::build(kb),
            tfidf: TfIdfIndex::build(kb),
        }
    }

    /// Retrieve top-k entries using RRF fusion of BM25 + TF-IDF
    pub fn retrieve<'a>(&self, query: &str, kb: &'a KnowledgeBase, top_k: usize) -> Vec<RetrievedEntry<'a>> {
        let pool = top_k * 3;
        let bm25_hits = self.bm25.search(query, pool);
        let tfidf_hits = self.tfidf.search(query, pool);

        // Build rank maps
        let bm25_rank: std::collections::HashMap<usize, usize> = bm25_hits.iter()
            .enumerate().map(|(rank, &(id, _))| (id, rank)).collect();
        let tfidf_rank: std::collections::HashMap<usize, usize> = tfidf_hits.iter()
            .enumerate().map(|(rank, &(id, _))| (id, rank)).collect();

        // Union of all candidate doc_ids
        let mut candidates: std::collections::HashSet<usize> = std::collections::HashSet::new();
        bm25_hits.iter().for_each(|&(id, _)| { candidates.insert(id); });
        tfidf_hits.iter().for_each(|&(id, _)| { candidates.insert(id); });

        // RRF score
        let mut scored: Vec<(usize, f32)> = candidates.into_iter().map(|doc_id| {
            let rrf_bm25 = bm25_rank.get(&doc_id)
                .map(|&r| 1.0 / (RRF_K + r as f32 + 1.0))
                .unwrap_or(0.0);
            let rrf_tfidf = tfidf_rank.get(&doc_id)
                .map(|&r| 1.0 / (RRF_K + r as f32 + 1.0))
                .unwrap_or(0.0);
            (doc_id, rrf_bm25 + rrf_tfidf)
        }).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        scored.into_iter().filter_map(|(doc_id, score)| {
            kb.entries.get(doc_id).map(|entry| RetrievedEntry {
                entry,
                doc_id,
                score,
                bm25_rank: bm25_rank.get(&doc_id).copied(),
                tfidf_rank: tfidf_rank.get(&doc_id).copied(),
            })
        }).collect()
    }
}
