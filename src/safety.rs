/// Safety module: Defines which commands are allowed, which require confirmation,
/// and which are absolutely forbidden.
use regex::Regex;
use std::sync::LazyLock;

/// Risk level of a command
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Safe,        // execute directly
    Moderate,    // ask for confirmation
    Dangerous,   // refuse completely
}

/// Result of a safety check
#[derive(Debug, Clone)]
pub struct SafetyCheck {
    pub risk: RiskLevel,
    pub reason: String,
    pub command: String,
}

/// Patterns that are always forbidden regardless of context
const FORBIDDEN_PATTERNS: &[&str] = &[
    r"rm\s+-rf\s+/",           // rm -rf /
    r"rm\s+-rf\s+~",           // rm -rf ~
    r"rm\s+-rf\s+\*",          // rm -rf *
    r"dd\s+if=.*of=/dev/sd",   // dd to disk device
    r"dd\s+if=.*of=/dev/hd",   // dd to disk device
    r"mkfs",                   // format filesystem
    r"fdisk",                  // partition disk
    r"parted",                 // partition disk
    r"shred",                  // secure delete
    r"wipefs",                 // wipe filesystem
    r":!\(:\)\{:\|:&\};:",     // fork bomb
    r">\s*/dev/sda",           // write to disk
    r">\s*/dev/hda",           // write to disk
    r"chmod\s+-R\s+777\s+/",   // chmod 777 on root
    r"chown\s+-R.*:\s*/",      // chown on root
    r"shutdown",               // shutdown system
    r"reboot",                 // reboot system
    r"halt",                   // halt system
    r"poweroff",               // poweroff
    r"init\s+0",               // init runlevel 0
    r"init\s+6",               // init runlevel 6
    r"systemctl\s+(poweroff|halt|reboot)",
    r"curl.*\|\s*bash",        // curl pipe to bash
    r"wget.*\|\s*bash",        // wget pipe to bash
    r"curl.*\|\s*sh",          // curl pipe to sh
    r"wget.*\|\s*sh",          // wget pipe to sh
    r"base64\s+-d.*\|\s*bash", // base64 decode pipe bash
    r"eval\s+\$\(",            // eval subshell
    r"sudo\s+su",              // privilege escalation
    r"su\s+-\s*$",             // switch to root
    r"passwd\s+root",          // change root password
    r"visudo",                 // edit sudoers
];

/// Patterns that need user confirmation (moderate risk)
const MODERATE_PATTERNS: &[(&str, &str)] = &[
    (r"rm\s+", "Menghapus file - pastikan benar!"),
    (r"sudo\s+", "Perintah dengan hak akses root"),
    (r"pip\s+install", "Instalasi package Python"),
    (r"apt\s+install", "Instalasi software baru"),
    (r"apt-get\s+install", "Instalasi software baru"),
    (r"npm\s+install\s+-g", "Instalasi package global npm"),
    (r"dpkg\s+-i", "Instalasi package .deb"),
    (r"chmod\s+", "Mengubah permission file"),
    (r"chown\s+", "Mengubah kepemilikan file"),
    (r"mv\s+", "Memindahkan/mengganti nama file"),
    (r"truncate\s+", "Memotong isi file"),
    (r">\s+[^\|]", "Menimpa isi file"),
    (r"kill\s+", "Menghentikan proses"),
    (r"pkill\s+", "Menghentikan proses berdasarkan nama"),
    (r"systemctl\s+(stop|disable|restart)", "Mengubah status layanan sistem"),
    (r"crontab\s+", "Mengubah cron jobs"),
    (r"iptables\s+", "Mengubah aturan firewall"),
    (r"ufw\s+", "Mengubah firewall"),
];

/// Shell composition patterns that are dangerous (semicolons, pipes to shells, subshells, backticks)
const SHELL_COMPOSITION_PATTERNS: &[&str] = &[
    r";\s*\w",                 // semicolon chaining: ; command
    r"\$\(",                   // subshell: $(...)
    r"`",                      // backtick command substitution
    r"\|\s*(ba)?sh",           // pipe to shell: | sh, | bash
    r"&&",                     // SECURITY: block ALL && chaining (not just && rm)
    r"\|\s*eval",              // pipe to eval
    r">\s*/etc/",              // write to system config
    r">\s*/usr/",              // write to system binaries
];

/// Pre-compiled forbidden regexes (compiled once at startup, not per-call)
static FORBIDDEN_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORBIDDEN_PATTERNS
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
});

/// Pre-compiled moderate regexes (compiled once at startup)
static MODERATE_REGEXES: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
    MODERATE_PATTERNS
        .iter()
        .filter_map(|(p, reason)| Regex::new(p).ok().map(|re| (re, *reason)))
        .collect()
});

/// Pre-compiled shell composition regexes
static SHELL_COMPOSITION_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    SHELL_COMPOSITION_PATTERNS
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
});

/// Check if a command is safe to execute
pub fn check_safety(command: &str) -> SafetyCheck {
    let cmd_lower = command.trim().to_lowercase();

    // Check shell composition patterns first (these are always dangerous)
    for re in SHELL_COMPOSITION_REGEXES.iter() {
        if re.is_match(&cmd_lower) {
            return SafetyCheck {
                risk: RiskLevel::Dangerous,
                reason: format!(
                    "❌ Perintah ini mengandung komposisi shell yang berbahaya dan diblokir.\nPola terdeteksi: {}",
                    re.as_str()
                ),
                command: command.to_string(),
            };
        }
    }

    // Check forbidden patterns (case-insensitive via cmd_lower)
    for re in FORBIDDEN_REGEXES.iter() {
        if re.is_match(&cmd_lower) {
            return SafetyCheck {
                risk: RiskLevel::Dangerous,
                reason: format!(
                    "❌ Perintah ini SANGAT BERBAHAYA dan diblokir demi keamanan sistem Anda.\nPola terdeteksi: {}",
                    re.as_str()
                ),
                command: command.to_string(),
            };
        }
    }

    // Check moderate patterns (FIXED: now also uses cmd_lower for case-insensitive matching)
    for (re, reason) in MODERATE_REGEXES.iter() {
        if re.is_match(&cmd_lower) {
            return SafetyCheck {
                risk: RiskLevel::Moderate,
                reason: format!("⚠️  {}", reason),
                command: command.to_string(),
            };
        }
    }

    SafetyCheck {
        risk: RiskLevel::Safe,
        reason: "✅ Perintah aman untuk dijalankan".to_string(),
        command: command.to_string(),
    }
}

/// Extract the primary command name from a full command string
pub fn extract_base_command(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rm_rf_root_blocked() {
        let check = check_safety("rm -rf /");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_ls_is_safe() {
        let check = check_safety("ls -la");
        assert_eq!(check.risk, RiskLevel::Safe);
    }

    #[test]
    fn test_sudo_is_moderate() {
        let check = check_safety("sudo apt install vim");
        assert_eq!(check.risk, RiskLevel::Moderate);
    }

    #[test]
    fn test_curl_pipe_bash_blocked() {
        let check = check_safety("curl http://example.com/install.sh | bash");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_case_insensitive_moderate() {
        // FIXED: Previously "SUDO" would bypass moderate check because
        // moderate patterns were checked against the original-case command.
        let check = check_safety("SUDO apt install vim");
        assert_eq!(check.risk, RiskLevel::Moderate);
    }

    #[test]
    fn test_case_insensitive_forbidden() {
        let check = check_safety("RM -RF /");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_shell_composition_semicolon() {
        // Semicolon chaining allows bypassing safety by hiding dangerous commands
        let check = check_safety("ls ; rm -rf /tmp/data");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_shell_composition_subshell() {
        let check = check_safety("echo $(cat /etc/shadow)");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_shell_composition_backtick() {
        let check = check_safety("echo `whoami`");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_shell_pipe_to_sh() {
        let check = check_safety("cat script.txt | sh");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_newline_bypass_attempt() {
        // Attempt to hide a dangerous command after a newline
        let check = check_safety("ls\nrm -rf /");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_rm_rf_home_blocked() {
        let check = check_safety("rm -rf ~");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_dd_to_disk_blocked() {
        let check = check_safety("dd if=/dev/zero of=/dev/sda bs=1M");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_mkfs_blocked() {
        let check = check_safety("mkfs.ext4 /dev/sda1");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_chained_delete_blocked() {
        let check = check_safety("cd /tmp && rm -rf data");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_write_to_etc_blocked() {
        let check = check_safety("echo 'malicious' > /etc/passwd");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_pipe_to_eval_blocked() {
        let check = check_safety("echo 'ls' | eval");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_base64_pipe_to_bash_blocked() {
        let check = check_safety("echo bHM= | base64 -d | bash");
        assert_eq!(check.risk, RiskLevel::Dangerous);
    }

    #[test]
    fn test_extract_base_command() {
        assert_eq!(extract_base_command("ls -la /tmp"), "ls");
        assert_eq!(extract_base_command("  sudo apt install vim  "), "sudo");
        assert_eq!(extract_base_command(""), "");
    }
}
