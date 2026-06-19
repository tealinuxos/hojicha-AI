/// Dynamic/JSON-based Linux command knowledge base corpus.
use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Memory,
    Cpu,
    Disk,
    Network,
    Process,
    Files,
    Permissions,
    Package,
    System,
    Text,
    Docker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskTag {
    Safe,
    Moderate,
    Dangerous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbEntry {
    pub id: String,
    pub command: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub category: Category,
    pub risk: RiskTag,
    pub example: String,
    pub beginner_tip: String,
}

pub struct KnowledgeBase {
    pub entries: Vec<KbEntry>,
}

impl KnowledgeBase {
    /// Load knowledge base from the specified path.
    /// If the file does not exist, it will write the default embedded JSON
    /// to the path so the user can easily modify it, then loads it.
    /// If writing fails, it falls back to the embedded JSON in-memory.
    pub fn load(path: &Path) -> Self {
        let default_json = if cfg!(target_os = "macos") {
            include_str!("../data/knowledge_base_macos.json")
        } else {
            include_str!("../data/knowledge_base_linux.json")
        };
        
        // Try reading from file
        if path.exists() {
            match Self::load_from_file(path) {
                Ok(kb) => {
                    // Check if cached DB is outdated (e.g. has fewer entries than the embedded version)
                    let default_entries: Vec<KbEntry> = serde_json::from_str(default_json).unwrap_or_default();
                    if kb.entries.len() < default_entries.len()
                        || std::fs::read_to_string(path).unwrap_or_default().contains("quit app \\\"Docker\\\"")
                        || std::fs::read_to_string(path).unwrap_or_default().contains("machdep.xcpm.cpu_thermal_level")
                        || !std::fs::read_to_string(path).unwrap_or_default().contains("saya berada dimana")
                    {
                        let _ = std::fs::write(path, default_json);
                        if let Ok(new_kb) = Self::load_from_file(path) {
                            return new_kb;
                        }
                    }
                    return kb;
                }
                Err(e) => {
                    eprintln!("⚠️  Gagal membaca knowledge base dari {}: {}. Menggunakan data bawaan.", path.display(), e);
                }
            }
        } else {
            // Path does not exist, try writing the default one
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(path, default_json) {
                eprintln!("⚠️  Gagal menulis file default knowledge base ke {}: {}. Menggunakan data bawaan.", path.display(), e);
            } else {
                println!("📝 Membuat file data default di: {}", path.display());
                match Self::load_from_file(path) {
                    Ok(kb) => return kb,
                    Err(e) => {
                        eprintln!("⚠️  Gagal membaca dari file yang baru dibuat: {}. Menggunakan data bawaan.", e);
                    }
                }
            }
        }

        // Fallback to embedded JSON
        let entries: Vec<KbEntry> = serde_json::from_str(default_json)
            .expect("Embedded knowledge base JSON should be valid");
        Self { entries }
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open file at {:?}", path))?;
        let reader = std::io::BufReader::new(file);
        let entries = serde_json::from_reader(reader)
            .with_context(|| format!("Failed to parse JSON from {:?}", path))?;
        Ok(Self { entries })
    }
}
