# Hojicha AI

Asisten terminal berbasis AI dengan konfigurasi provider LLM cloud dan lokal.

## Prerequisites
- Rust and Cargo installed.

## Installation
Clone the repository and build the project:
```bash
cargo build --release
```

## Usage
Run the application using Cargo:

### Ask a Question
```bash
cargo run -- "cek ram laptop saya"
```

Provider LLM yang didukung: Ollama lokal, OpenAI, Gemini, OpenRouter, dan Groq.

### Interactive Mode (REPL)
Run without arguments to enter interactive mode:
```bash
cargo run
```
Inside the REPL, type `/model` to configure the active provider, API key, model, and base URL.

### Display Version
```bash
cargo run -- --version
```

## Project Structure
- `src/main.rs`: Contains the CLI definition and command handling logic.
- `Cargo.toml`: Manages dependencies (e.g., `clap`).

## Testing
Run unit tests:
```bash
cargo test
```
