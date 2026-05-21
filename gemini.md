# TERA-AI — Arsitektur Aplikasi

> **TERA**: Terminal-based Rusty Assistant — Asisten terminal Linux bertenaga AI lokal untuk pengguna pemula.

---

## Ringkasan

TERA adalah CLI tool berbasis Rust yang membantu pemula Linux berinteraksi dengan terminal menggunakan bahasa alami (Indonesia/Inggris). Aplikasi ini berjalan **100% offline** menggunakan model LLM lokal melalui [Ollama](https://ollama.com).

---

## Alur Kerja Utama

```
Pengguna menulis bahasa alami
         │
         ▼
  ┌─────────────┐
  │  lib.rs     │  ← Orchestrator utama
  │  REPL Loop  │
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  prompt.rs  │  ← Membangun system prompt + user prompt
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │   ai.rs     │  ← Kirim ke Ollama HTTP API → dapat JSON response
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  safety.rs  │  ← Cek keamanan perintah (Safe / Moderate / Dangerous)
  └──────┬──────┘
         │ ──── Dangerous? BLOKIR & jelaskan
         │ ──── Moderate?  Minta konfirmasi user
         │ ──── Safe?      Lanjut
         ▼
  ┌─────────────┐
  │ executor.rs │  ← Jalankan command via `sh -c`
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │   ai.rs     │  ← Kirim output ke Ollama untuk dirangkum
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │   ui.rs     │  ← Tampilkan hasil + ringkasan ke terminal
  └─────────────┘
```

---

## Struktur Modul

| File | Tanggung Jawab |
|------|----------------|
| `src/main.rs` | Entry point, async `main()` dengan Tokio |
| `src/lib.rs` | CLI struct (`Cli`), orchestrator `run()`, REPL loop, `process_query()` |
| `src/ai.rs` | `OllamaClient` — komunikasi HTTP ke Ollama API, parsing JSON response |
| `src/safety.rs` | `check_safety()` — regex-based command safety checker |
| `src/executor.rs` | `execute_command()` — jalankan shell command, capture output |
| `src/prompt.rs` | `system_prompt()`, `output_summary_prompt()` — builder prompt LLM |
| `src/ui.rs` | Semua fungsi output terminal berwarna menggunakan `colored` |

---

## Komponen Detail

### `ai.rs` — Ollama Client

```rust
pub struct OllamaClient {
    client: Client,      // reqwest HTTP client
    base_url: String,    // default: http://localhost:11434
    pub model: String,   // default: qwen2.5:1.5b
}
```

**Endpoints yang digunakan:**
- `GET /api/tags` — cek koneksi & daftar model
- `POST /api/generate` — generate response dari model

**Parameter LLM (dioptimalkan untuk command generation):**
```json
{
  "temperature": 0.1,   // deterministic output
  "num_predict": 512,   // respons pendek
  "top_k": 10,
  "top_p": 0.9,
  "stream": false
}
```

---

### `safety.rs` — Safety Checker

Tiga level risiko:

| Level | Aksi | Contoh |
|-------|------|--------|
| `Safe` | Langsung eksekusi | `ls -la`, `free -h`, `df -h` |
| `Moderate` | Minta konfirmasi | `sudo apt install`, `rm file.txt`, `chmod 755` |
| `Dangerous` | **BLOKIR PERMANEN** | `rm -rf /`, `curl \| bash`, `dd if=... of=/dev/sda` |

**Pola yang diblokir (contoh):**
- `rm -rf /` atau `rm -rf ~` atau `rm -rf *`
- `dd` ke device block (`/dev/sda`, `/dev/hda`)
- `mkfs`, `fdisk`, `parted`, `shred`, `wipefs`
- Fork bomb: `:!(){ :|:& };:`
- Pipe to shell: `curl | bash`, `wget | sh`
- Privilege escalation: `sudo su`, `su -`, `passwd root`
- System halt: `shutdown`, `reboot`, `halt`, `poweroff`

---

### `prompt.rs` — Prompt Engineering

**System prompt** memaksa model untuk selalu merespons dalam format JSON terstruktur:

```json
{
  "command": "free -h",
  "explanation": "Menampilkan penggunaan RAM saat ini.",
  "beginner_tip": "Kolom 'available' adalah RAM yang masih bisa digunakan.",
  "is_safe": true
}
```

**Output summary prompt** meminta ringkasan dalam bahasa Indonesia yang ramah pemula:

```json
{
  "summary": "Laptop Anda memiliki 8GB RAM total, 3.2GB sedang digunakan.",
  "key_info": "Tersisa 4.5GB RAM yang bisa digunakan.",
  "next_suggestion": "Jika RAM hampir penuh, coba tutup aplikasi yang tidak dipakai."
}
```

---

### `lib.rs` — Orchestrator & REPL

**Mode operasi:**

1. **Single-query mode** — `tera "cek ram saya"`
2. **Interactive REPL** — `tera` (tanpa argumen)

**REPL commands:**
- `exit` / `quit` / `q` — keluar
- `help` / `?` — tampilkan bantuan
- `model` — info model yang aktif
- `clear` — hapus riwayat percakapan

**Conversation history:** Menyimpan 5 percakapan terakhir untuk konteks follow-up.

---

## CLI Options

```
tera [OPTIONS] [QUERY]

Arguments:
  [QUERY]   Pertanyaan dalam bahasa alami (tanpa argumen = mode interaktif)

Options:
  -m, --model <MODEL>          Model Ollama [default: qwen2.5:1.5b]
      --ollama-url <URL>        URL server Ollama [default: http://localhost:11434]
      --no-summary             Nonaktifkan ringkasan AI (lebih cepat)
  -y, --yes                    Auto-konfirmasi semua perintah moderate
  -h, --help                   Tampilkan bantuan
  -V, --version                Tampilkan versi
```

---

## Model yang Direkomendasikan

| Model | Ukuran | RAM Min | Kecepatan | Kualitas |
|-------|--------|---------|-----------|---------|
| `qwen2.5:0.5b` | ~400MB | 1GB | ⚡⚡⚡ Sangat cepat | ⭐⭐ |
| `qwen2.5:1.5b` | ~1GB | 2GB | ⚡⚡ Cepat | ⭐⭐⭐ *(default)* |
| `qwen2.5:3b` | ~2GB | 4GB | ⚡ Sedang | ⭐⭐⭐⭐ |
| `llama3.2:1b` | ~700MB | 2GB | ⚡⚡ Cepat | ⭐⭐⭐ |
| `llama3.2:3b` | ~2GB | 4GB | ⚡ Sedang | ⭐⭐⭐⭐ |

**Untuk pemula dengan RAM terbatas:** `qwen2.5:0.5b`
**Rekomendasi umum:** `qwen2.5:1.5b`

---

## Dependencies

| Crate | Versi | Fungsi |
|-------|-------|--------|
| `clap` | 4 | CLI argument parsing dengan derive macro |
| `tokio` | 1 | Async runtime |
| `reqwest` | 0.12 | HTTP client ke Ollama API |
| `serde` / `serde_json` | 1 | JSON serialization/deserialization |
| `colored` | 2 | Warna terminal output |
| `anyhow` | 1 | Error handling ergonomis |
| `regex` | 1 | Pattern matching safety checker |
| `dialoguer` | 0.11 | Interactive terminal prompts |

---

## Instalasi & Penggunaan

### 1. Prasyarat

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull model (pilih salah satu)
ollama pull qwen2.5:1.5b
```

### 2. Build

```bash
cd tera-ai
cargo build --release

# Binary ada di: ./target/release/tera
```

### 3. Install global (opsional)

```bash
cargo install --path .
# Setelah itu: tera "pertanyaan kamu"
```

### 4. Jalankan

```bash
# Mode interaktif
tera

# Single query
tera "cek ram laptop saya"
tera "lihat file di folder ini"
tera "berapa ukuran folder Downloads?"
tera "show running processes"

# Gunakan model berbeda
tera -m qwen2.5:0.5b "cek koneksi internet"

# Tanpa ringkasan AI (lebih cepat)
tera --no-summary "df -h"

# Auto-yes (skip konfirmasi)
tera -y "sudo apt update"
```

---

## Prinsip Desain

1. **Safety First** — Perintah berbahaya diblokir sebelum sampai ke shell
2. **Offline First** — Tidak ada koneksi internet yang dibutuhkan setelah setup
3. **Beginner Friendly** — Semua output dalam bahasa Indonesia yang mudah dipahami
4. **Low Resource** — Model kecil, tidak perlu GPU
5. **Transparent** — Selalu tampilkan perintah sebelum dijalankan
6. **Educational** — Setiap perintah disertai penjelasan dan tips

---

## Keamanan

- ✅ Semua perintah ditampilkan ke user **sebelum** dijalankan
- ✅ Perintah destruktif diblokir dengan regex pattern matching
- ✅ Perintah berisiko sedang memerlukan konfirmasi eksplisit user
- ✅ AI tidak bisa bypass safety layer (safety check dilakukan di Rust, bukan AI)
- ✅ Tidak ada telemetri atau data yang dikirim ke server eksternal
- ✅ Model berjalan lokal via Ollama

---

*Dibuat dengan ❤️ untuk pemula Linux yang ingin belajar tanpa takut merusak sistem.*
