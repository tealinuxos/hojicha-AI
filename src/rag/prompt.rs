/// System prompt that defines the assistant's persona and behavior
pub fn system_prompt() -> String {
    let os = if std::env::consts::OS == "macos" { "macOS" } else { "Linux" };

    format!(
        r#"Kamu adalah Hojicha, asisten terminal {} untuk pemula.
Tugas Anda adalah menganalisis pertanyaan user dan mengembalikan respons dalam format JSON yang valid.

Skema JSON yang harus dikembalikan:
{{
  "command": <perintah shell untuk dijalankan, atau null jika tidak ada perintah yang cocok atau tidak aman>,
  "explanation": <penjelasan singkat dan ramah tentang perintah tersebut, atau balasan percakapan Anda dalam bahasa Indonesia>,
  "beginner_tip": <tips praktis bagi pemula untuk menggunakan perintah ini, atau null jika tidak ada>,
  "is_safe": <true jika perintah aman untuk dijalankan secara langsung, false jika berbahaya seperti menghapus data secara tidak sengaja>
}}

ATURAN:
1. Output WAJIB berupa JSON yang valid, tanpa teks penjelasan tambahan di luar JSON.
2. Gunakan referensi perintah di bawah ini untuk mencari perintah yang relevan."#,
        os
    )
}

/// Build the prompt for summarizing command output
pub fn output_summary_prompt(command: &str, output: &str, success: bool) -> String {
    let status = if success { "berhasil" } else { "gagal" };
    let os = if std::env::consts::OS == "macos" { "macOS" } else { "Linux" };
    format!(
        r#"Perintah `{}` telah {} dijalankan.

Output terminal:
```
{}
```

Jelaskan output di atas dalam bahasa Indonesia yang sederhana untuk pemula {}.
Format respons JSON:
{{
  "summary": "<penjelasan singkat output dalam 2-3 kalimat bahasa Indonesia>",
  "key_info": "<informasi terpenting dari output ini>",
  "next_suggestion": "<saran langkah berikutnya yang mungkin berguna, atau null>"
}}

Hanya kembalikan JSON, tidak ada teks lain."#,
        command,
        status,
        output,
        os
    )
}

/// Build a conversational follow-up prompt
pub fn followup_prompt(history: &[(String, String)], user_input: &str) -> String {
    let mut prompt = system_prompt();
    prompt.push_str("\n\nRiwayat percakapan:\n");
    for (user, assistant) in history.iter().take(5) {
        prompt.push_str(&format!("User: {}\nHojicha: {}\n\n", user, assistant));
    }
    prompt.push_str(&format!("User: {}\nHojicha:", user_input));
    prompt
}
