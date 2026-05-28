# Hojicha (Asisten Terminal AI untuk Pemula)

Hojicha adalah asisten terminal berbasis AI yang dirancang khusus untuk membantu pemula memahami dan menjalankan perintah shell di Linux dan macOS secara aman. Menggunakan arsitektur RAG (Retrieval-Augmented Generation) berbasis kecerdasan buatan, Hojicha dapat menerjemahkan bahasa sehari-hari menjadi perintah terminal yang siap dieksekusi.

---

## 🚀 Fitur Utama

- **Penerjemah Bahasa Alami**: Cukup ketik apa yang ingin Anda lakukan (contoh: *"cek penggunaan RAM"* atau *"buat folder baru bernama project"*), Hojicha akan memberikan rekomendasi perintah beserta penjelasannya.
- **Dukungan Multi-Provider LLM**: Terhubung ke berbagai engine kecerdasan buatan:
  - **Native** (Inference GGUF lokal offline menggunakan Rust Candle)
  - **Ollama** (Model lokal seperti Qwen, Llama, dll.)
  - **OpenAI** (GPT-4o, GPT-4o-mini)
  - **Google Gemini** (Gemini 1.5 Flash / Pro)
  - **Anthropic Claude** (Claude 3.5 Sonnet / Haiku)
- **Menu Konfigurasi Interaktif (`/model`)**: Konfigurasikan API Key, endpoint server, parameter model, dan ubah provider secara interaktif langsung dari dalam terminal tanpa harus merestart aplikasi.
- **Deteksi Model Ollama Otomatis**: Secara otomatis memindai dan mencantumkan model-model yang telah terinstal di server Ollama lokal Anda.
- **Pemisahan Knowledge Base Berdasarkan OS**: Secara otomatis mendeteksi sistem operasi (macOS vs Linux) untuk memuat data RAG yang paling relevan (misalnya, menggunakan Homebrew untuk macOS dan APT untuk Linux).
- **Sistem Keamanan Pintar**: Menganalisis risiko perintah sebelum dijalankan (Safe, Moderate, Dangerous) untuk mencegah kesalahan eksekusi perintah berbahaya.

---

## 🛠️ Instalasi & Prasyarat

Pastikan Anda telah menginstal **Rust** dan **Cargo** di sistem Anda.

1. **Clone Repository & Build**:
   ```bash
   git clone https://github.com/tealinuxos/hojicha-AI.git
   cd hojicha-AI
   cargo build --release
   ```

2. **Jalankan Aplikasi**:
   ```bash
   cargo run --release
   ```

---

## 📖 Cara Penggunaan

Hojicha mendukung dua mode operasi utama:

### 1. Mode Tanya Langsung (Single Query)
Masukkan pertanyaan Anda langsung dari CLI:
```bash
cargo run -- "cara mencari file berakhiran .pdf di folder ini"
```
Anda juga bisa memaksa Hojicha berjalan secara offline menggunakan model bawaan:
```bash
cargo run -- -n "cek sisa penyimpanan disk"
```

### 2. Mode Interaktif (REPL)
Jalankan Hojicha tanpa argumen untuk masuk ke shell interaktif:
```bash
cargo run
```
Di dalam mode interaktif, Anda dapat menggunakan perintah khusus berikut:
* `/help` atau `/h`: Menampilkan bantuan penggunaan.
* `/model`: Membuka wizard konfigurasi provider LLM.
* `/clear`: Membersihkan riwayat obrolan/history.
* `/exit` atau `/q`: Keluar dari aplikasi Hojicha.

---

## ⚙️ Wizard Konfigurasi (`/model`)

Saat berada di mode interaktif, ketik `/model` untuk membuka wizard. Wizard ini mempermudah Anda untuk:
1. Memilih provider aktif (Native, Ollama, OpenAI, Gemini, Anthropic).
2. Memilih model Ollama lokal secara otomatis melalui scanning API tags.
3. Memasukkan API Key secara aman (input password disamarkan).
4. Menyesuaikan setelan teknis seperti `temperature` dan `max_tokens`.

Konfigurasi disimpan di lokasi standar pengguna: `~/.config/hojicha/config.json`.

---

## 🧪 Menjalankan Pengujian

Untuk menjalankan rangkaian pengujian unit (unit tests):
```bash
cargo test
```
