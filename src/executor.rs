/// Executor module: Runs shell commands and captures output.
use anyhow::Result;
use std::process::Command;
use std::sync::Mutex;
use std::path::PathBuf;
use crate::safety::{check_safety, RiskLevel};

static PREV_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Maximum output lines to retain (prevents memory blowup from verbose commands)
const MAX_OUTPUT_LINES: usize = 500;

/// Output of a command execution
#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

/// Execute a shell command and capture its output.
/// Output is truncated to MAX_OUTPUT_LINES to prevent memory issues
/// from extremely verbose commands (e.g., `find /`).
pub fn execute_command(command: &str) -> Result<CommandOutput> {
    // Persistent directory navigation (cd handling)
    let trimmed = command.trim();
    if trimmed == "cd" || trimmed.starts_with("cd ") {
        let is_compound = trimmed.contains(';') || trimmed.contains('&') || trimmed.contains('|');
        let first_cmd = trimmed.split(|c| c == ';' || c == '&' || c == '|').next().unwrap_or(trimmed).trim();
        
        let mut cd_success = false;
        let mut err_msg = String::new();
        let current_dir = std::env::current_dir().ok();

        let target_path = if first_cmd == "cd" {
            std::env::var("HOME").ok().map(PathBuf::from)
        } else if first_cmd.starts_with("cd ") {
            let path_str = first_cmd[3..].trim();
            let path_str = path_str.trim_matches(|c| c == '"' || c == '\'');
            
            if path_str == "-" {
                // FIXED: Handle poisoned mutex gracefully instead of panicking
                let prev = PREV_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if prev.is_none() {
                    err_msg = "cd: OLDPWD tidak diset".to_string();
                }
                prev
            } else if path_str == "~" {
                std::env::var("HOME").ok().map(PathBuf::from)
            } else if path_str.starts_with("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    Some(PathBuf::from(home).join(&path_str[2..]))
                } else {
                    Some(PathBuf::from(path_str))
                }
            } else {
                Some(PathBuf::from(path_str))
            }
        } else {
            None
        };

        if let Some(ref path) = target_path {
            if let Err(e) = std::env::set_current_dir(path) {
                err_msg = format!("cd: {}: {}", path.display(), e);
            } else {
                cd_success = true;
                if let Some(old) = current_dir {
                    // FIXED: Handle poisoned mutex gracefully instead of panicking
                    *PREV_DIR.lock().unwrap_or_else(|e| e.into_inner()) = Some(old);
                }
            }
        } else if err_msg.is_empty() {
            err_msg = "cd: Gagal menentukan direktori target".to_string();
        }

        // If it's a simple cd command (not compound), return immediately
        if !is_compound {
            if cd_success {
                let stdout = if first_cmd.ends_with(" -") {
                    std::env::current_dir().map(|p| format!("{}\n", p.display())).unwrap_or_default()
                } else {
                    String::new()
                };
                return Ok(CommandOutput {
                    stdout,
                    stderr: String::new(),
                    exit_code: 0,
                    success: true,
                });
            } else {
                return Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: err_msg,
                    exit_code: 1,
                    success: false,
                });
            }
        }

        // FIXED: For compound cd commands (e.g., `cd /tmp && ls`), strip the cd
        // portion before passing to sh -c. Previously the full command was passed,
        // causing the cd to execute twice (once via Rust, once via sh -c).
        if cd_success {
            // Extract the non-cd remainder after `&&`
            if let Some(pos) = trimmed.find("&&") {
                let rest = trimmed[pos + 2..].trim();
                if !rest.is_empty() {
                    // SECURITY: Run safety check on the remainder command before
                    // passing to sh -c. Previously, `cd /tmp && <malicious>` would
                    // execute <malicious> without any safety screening.
                    let safety = check_safety(rest);
                    if safety.risk == RiskLevel::Dangerous {
                        return Ok(CommandOutput {
                            stdout: String::new(),
                            stderr: format!("❌ Blocked: {}", safety.reason),
                            exit_code: 1,
                            success: false,
                        });
                    }
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(rest)
                        .output()?;
                    let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);
                    let success = output.status.success();
                    let stdout = truncate_output(&raw_stdout, MAX_OUTPUT_LINES);
                    let stderr = truncate_output(&raw_stderr, MAX_OUTPUT_LINES);
                    return Ok(CommandOutput { stdout, stderr, exit_code, success });
                }
            }
            // For non-&& compounds (;, |, ||), fall through to sh -c with original command.
            // The shell will re-execute cd but since cwd is already changed, it's a no-op.
        } else {
            // cd failed — return error without executing the rest
            return Ok(CommandOutput {
                stdout: String::new(),
                stderr: err_msg,
                exit_code: 1,
                success: false,
            });
        }
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();

    // FIXED: Apply truncation to prevent unbounded memory usage from verbose output.
    // Previously, truncate_output() was defined but never called.
    let stdout = truncate_output(&raw_stdout, MAX_OUTPUT_LINES);
    let stderr = truncate_output(&raw_stderr, MAX_OUTPUT_LINES);

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code,
        success,
    })
}

/// Truncate output to avoid overwhelming memory and the AI context.
pub fn truncate_output(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= max_lines {
        output.to_string()
    } else {
        let shown = &lines[..max_lines];
        format!(
            "{}\n... ({} baris lagi tidak ditampilkan)",
            shown.join("\n"),
            lines.len() - max_lines
        )
    }
}
