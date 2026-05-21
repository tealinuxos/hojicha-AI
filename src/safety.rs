/// Safety module: Defines which commands are allowed, which require confirmation,
/// and which are absolutely forbidden.
use regex::Regex;

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

/// Check if a command is safe to execute
pub fn check_safety(command: &str) -> SafetyCheck {
    let cmd_lower = command.trim().to_lowercase();

    // Check forbidden patterns
    for pattern in FORBIDDEN_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&cmd_lower) {
                return SafetyCheck {
                    risk: RiskLevel::Dangerous,
                    reason: format!(
                        "❌ Perintah ini SANGAT BERBAHAYA dan diblokir demi keamanan sistem Anda.\nPola terdeteksi: {}",
                        pattern
                    ),
                    command: command.to_string(),
                };
            }
        }
    }

    // Check moderate patterns
    for (pattern, reason) in MODERATE_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(command) {
                return SafetyCheck {
                    risk: RiskLevel::Moderate,
                    reason: format!("⚠️  {}", reason),
                    command: command.to_string(),
                };
            }
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
    command.trim().split_whitespace().next().unwrap_or("")
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
}
