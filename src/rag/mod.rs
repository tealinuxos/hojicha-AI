pub mod client;
pub mod local;
pub mod ollama;
pub mod utils;

pub use client::{AiClient, CommandResponse, OutputSummary};
pub use local::LocalModelClient;
pub use ollama::OllamaClient;
