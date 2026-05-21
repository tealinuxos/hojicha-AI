/// Rule engine — deterministic fast-path responses for known intents.
/// Returns a CommandResponse without touching the LLM or RAG pipeline.

use crate::intent::{Intent, ProcessAction, SystemMetric};
use crate::rag::client::CommandResponse;
use crate::rag::kb::KnowledgeBase;

/// Try to resolve an intent via rules. Returns None to fall through to RAG.
pub fn try_rule_engine(intent: &Intent, kb: &KnowledgeBase) -> Option<CommandResponse> {
    let find_entry = |id: &str| -> Option<&crate::rag::kb::KbEntry> {
        kb.entries.iter().find(|e| e.id == id)
    };

    match intent {
        // ── System Metrics ───────────────────────────────────────────
        Intent::CheckSystem(SystemMetric::Ram) => {
            let entry = find_entry("mem-free")?;
            Some(CommandResponse {
                command: Some(entry.command.clone()),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::CheckSystem(SystemMetric::Cpu) => {
            let entry = find_entry("cpu-top")?;
            Some(CommandResponse {
                command: Some(entry.command.clone()),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::CheckSystem(SystemMetric::Disk) => {
            let entry = find_entry("disk-df")?;
            Some(CommandResponse {
                command: Some(entry.command.clone()),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::CheckSystem(SystemMetric::Network) => {
            let entry = find_entry("net-ping")?;
            Some(CommandResponse {
                command: Some(entry.command.clone()),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::CheckSystem(SystemMetric::Temperature) => {
            let entry = find_entry("cpu-temp")?;
            Some(CommandResponse {
                command: Some(entry.command.clone()),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        // ── Files ────────────────────────────────────────────────────
        Intent::ListFiles(path) => {
            let entry = find_entry("file-ls")?;
            let cmd = match path {
                Some(p) if !p.is_empty() => format!("{} {}", entry.command, p),
                _ => entry.command.clone(),
            };
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::WhereAmI => {
            let entry = find_entry("file-pwd")?;
            Some(CommandResponse {
                command: Some(entry.command.clone()),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::FindFile(pattern) => {
            let entry = find_entry("file-find")?;
            let cmd = if pattern == "*" || pattern.is_empty() {
                "find . -type f".into()
            } else {
                format!("{} \"*{}*\"", entry.command, pattern.trim_matches('"'))
            };
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry.description.replace("{}", pattern),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        // ── System Info ──────────────────────────────────────────────
        Intent::SystemInfo => {
            let entry_uname = find_entry("sys-uname")?;
            let entry_lsb = find_entry("sys-lsb")?;
            let cmd = format!("{} && {} 2>/dev/null", entry_uname.command, entry_lsb.command);
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry_uname.description.clone(),
                beginner_tip: Some(entry_uname.beginner_tip.clone()),
                is_safe: entry_uname.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::ShowHistory => {
            let entry = find_entry("sys-history")?;
            let cmd = if entry.command == "history" {
                "history | tail -20".to_string()
            } else {
                entry.command.clone()
            };
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        // ── Process Management ───────────────────────────────────────
        Intent::ManageProcess(ProcessAction::List) => {
            let entry = find_entry("proc-ps")?;
            let cmd = if entry.command == "ps aux" {
                "ps aux | head -20".to_string()
            } else {
                entry.command.clone()
            };
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry.description.clone(),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        Intent::ManageProcess(ProcessAction::Kill(name)) => {
            let entry = find_entry("proc-pkill")?;
            let cmd = format!("{} {}", entry.command, name);
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry.description.replace("{}", name),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        // ── Package Install (provides command but needs confirmation) ─
        Intent::InstallPackage(pkg) => {
            let entry = find_entry("pkg-install")?;
            let cmd = format!("{} {}", entry.command, pkg);
            Some(CommandResponse {
                command: Some(cmd),
                explanation: entry.description.replace("{}", pkg),
                beginner_tip: Some(entry.beginner_tip.clone()),
                is_safe: entry.risk == crate::rag::kb::RiskTag::Safe,
            })
        }

        // ── General — no rule, fall through ─────────────────────────
        Intent::General => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::SystemMetric;
    use std::path::Path;

    #[test]
    fn test_try_rule_engine_ram() {
        let kb = KnowledgeBase::load(Path::new("non_existent_kb_for_test.json"));
        let intent = Intent::CheckSystem(SystemMetric::Ram);
        let resp = try_rule_engine(&intent, &kb).unwrap();
        assert_eq!(resp.command.unwrap(), "vm_stat");
        assert_eq!(resp.explanation, "Menampilkan statistik penggunaan memori virtual sistem (macOS)");
        assert!(resp.is_safe);
        // Clean up the generated file after test
        let _ = std::fs::remove_file("non_existent_kb_for_test.json");
    }
}
