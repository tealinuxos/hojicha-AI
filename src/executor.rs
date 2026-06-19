/// Executor module: Runs shell commands and captures output.
use anyhow::Result;
use std::process::Command;

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
