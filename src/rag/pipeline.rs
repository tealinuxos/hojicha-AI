/// Full RAG pipeline orchestrator.
/// Flow: rewrite → retrieve → rerank → build context prompt → LLM

use anyhow::Result;

use crate::rag::client::{AiClient, CommandResponse};
use crate::rag::kb::KnowledgeBase;
use crate::rag::reranker::rerank;
use crate::rag::retriever::HybridRetriever;

/// Singleton-style pipeline — built once, reused across queries
pub struct RagPipeline {
    pub kb: KnowledgeBase,
    retriever: HybridRetriever,
}

impl RagPipeline {
    /// Build the pipeline. Indexes KB in memory (fast, ~1ms).
    pub fn build(kb_path: &std::path::Path) -> Self {
        let kb = KnowledgeBase::load(kb_path);
        let retriever = HybridRetriever::build(&kb);
        Self { kb, retriever }
    }

    /// Run the full RAG pipeline for a given query.
    /// Returns a CommandResponse enriched with KB context.
    pub async fn run(
        &self,
        ai_client: &mut AiClient,
        user_input: &str,
    ) -> Result<CommandResponse> {
        // 1. Rewrite query for better retrieval
        let rewritten = rewrite_query(user_input);

        // 2. Hybrid retrieval
        let hits = self.retriever.retrieve(&rewritten, &self.kb, 6);

        // 3. Rerank
        let ranked = rerank(&rewritten, hits);

        // 4. Build context from top-3 entries
        let top3: Vec<_> = ranked.into_iter().take(3).collect();
        let context = build_context(&top3);

        // 5. Append general assistant info to context so the model always has its persona context
        // Removed hardcoded general_info.json inclusion to allow greetings and info to be resolved purely via retrieved RAG knowledge base.

        // 6. Build RAG-augmented system prompt
        let system = build_rag_system_prompt(&context);

        // 7. Call LLM
        let response = ai_client.nl_to_command(&system, user_input).await?;
        Ok(response)
    }
}

/// Simple query rewriter: normalizes case and trims whitespace
fn rewrite_query(input: &str) -> String {
    input.to_lowercase().trim().to_string()
}

fn build_context(entries: &[crate::rag::retriever::RetrievedEntry<'_>]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut ctx = String::from("Referensi perintah Linux yang relevan:\n");
    for (i, e) in entries.iter().enumerate() {
        ctx.push_str(&format!(
            "\n[{}] Perintah: `{}`\n    Keterangan: {}\n    Tips: {}\n",
            i + 1,
            e.entry.command,
            e.entry.description,
            e.entry.beginner_tip,
        ));
    }
    ctx
}

fn build_rag_system_prompt(context: &str) -> String {
    let base = crate::rag::system_prompt();
    if context.is_empty() {
        return base;
    }
    format!("{}\n\n---\n{}", base, context)
}
