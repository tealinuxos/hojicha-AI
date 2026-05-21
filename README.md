# Tera-AI CLI Skeleton

A minimal terminal application skeleton built with Rust and `clap`.

## Prerequisites
- Rust and Cargo installed.

## Installation
Clone the repository and build the project:
```bash
cargo build --release
```

## Usage
Run the application using Cargo:

### Echo a message
```bash
cargo run -- echo "Hello, Rust!"
```

### Ask a Question (Local AI)
Interact with a local AI model (simulated in skeleton mode):
```bash
cargo run -- ask "Apa itu Rust?" --model models/llama-3.gguf
```

### Interactive Mode (REPL)
Run without arguments to enter interactive mode:
```bash
cargo run
```
Inside the REPL, type any message to see it echoed back, or type `exit`/`quit` to leave.

### Project Structure
- `src/ai.rs`: Local AI inference engine logic.
- `models/`: Directory to store GGUF/GGML model files.
- `src/lib.rs`: Core application logic and command definitions.

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
