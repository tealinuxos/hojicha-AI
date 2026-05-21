pub mod bm25;
pub mod client;
pub mod embedder;
pub mod kb;
pub mod local;
pub mod ollama;
pub mod pipeline;
pub mod prompt;
pub mod reranker;
pub mod retriever;
pub mod utils;

pub use client::{AiClient, CommandResponse, OutputSummary};
pub use local::LocalModelClient;
pub use ollama::OllamaClient;
pub use pipeline::RagPipeline;
pub use prompt::{system_prompt, output_summary_prompt, followup_prompt};
