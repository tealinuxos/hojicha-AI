# Hojicha AI

Asisten terminal berbasis AI dengan konfigurasi provider LLM cloud dan lokal.  
Dirancang untuk pemula Linux/macOS yang ingin menggunakan terminal dengan bantuan AI.

---

## ⚡ Quick Install (Global)

Setelah clone, jalankan script berikut untuk langsung bisa panggil `hojicha` dari direktori mana pun:

```bash
git clone https://github.com/tealinuxos/hojicha-AI
cd hojicha-AI
./install.sh
```

Script ini akan:
1. Build binary release (`cargo build --release`)
2. Install binary ke `~/.local/bin/hojicha`
3. Bantu setup PATH jika belum ada

Setelah install, buka terminal baru (atau `source ~/.zshrc`) lalu:

```bash
# Mode interaktif — ketik dari direktori mana pun
hojicha

# Mode langsung
hojicha "cek ram laptop saya"
hojicha "list semua file di folder ini"

# Lihat semua opsi
hojicha --help
```

> **Catatan:** Saat `hojicha` dijalankan, working directory-nya otomatis mengikuti dari mana kamu memanggil perintah tersebut. Jadi kalau kamu ketik `hojicha` dari `~/Documents`, semua perintah file akan berjalan di sana.

---

## Prerequisites

- Rust dan Cargo terinstall → [rustup.rs](https://rustup.rs)

---

## Provider LLM yang Didukung

Provider LLM yang didukung: **Ollama** (lokal), **OpenAI**, **Gemini**, **OpenRouter**, dan **Groq**.

Konfigurasi provider bisa dilakukan lewat wizard interaktif saat pertama kali menjalankan `hojicha`, atau ketik `/model` di dalam REPL.

---

## Penggunaan Lanjutan

### Manual Build (tanpa install script)
```bash
cargo build --release
./target/release/hojicha
```

### Run via Cargo (tanpa install)
```bash
cargo run -- "cek ram laptop saya"
cargo run                          # mode interaktif
```

### Opsi CLI
```bash
hojicha --help
hojicha --local "pertanyaan"       # pakai model built-in offline
hojicha -y "hapus file tmp"        # langsung eksekusi tanpa konfirmasi
```

---

## Project Structure
- `src/main.rs`: Entry point binary
- `src/lib.rs`: Core logic, CLI definition, RAG pipeline runner
- `src/rag/`: RAG pipeline, LLM clients, BM25, embedder, retriever
- `src/data/`: Knowledge base JSON (embedded ke binary saat compile)
- `src/config/`: Konfigurasi per-provider LLM
- `install.sh`: Script install global
- `PKGBUILD`: Build script untuk Arch Linux package

## Testing
```bash
cargo test
```
