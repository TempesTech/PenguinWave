//! Subprocess plumbing.

use std::process::Command;

use penguinwave_proto::{ErrorKind, PwError};

use crate::domain::parse::is_tool_error;

/// Run a command and return stdout.
///
/// `LC_ALL=C` is set because `pactl` localises decimal separators, and the
/// parsers read numbers.
///
/// A zero exit status is not accepted as proof of success: `pw-cli` exits 0
/// after printing `Error: ...`, which the pre-split code parsed as data.
pub fn run(program: &str, args: &[&str]) -> Result<String, PwError> {
    let output = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PwError::new(
                ErrorKind::PipeWireUnavailable,
                format!("{program} not found; is PipeWire installed?"),
            ),
            _ => PwError::new(ErrorKind::PipeWireFailed, format!("{program}: {e}")),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(classify(program, args, &stderr));
    }
    if is_tool_error(&stdout) {
        return Err(PwError::new(
            ErrorKind::PipeWireFailed,
            format!("{program}: {}", stdout.trim()),
        ));
    }

    Ok(stdout)
}

/// Run a command for its effect.
pub fn run_ok(program: &str, args: &[&str]) -> Result<(), PwError> {
    run(program, args).map(|_| ())
}

fn classify(program: &str, args: &[&str], stderr: &str) -> PwError {
    let msg = stderr.trim();
    let kind = if msg.contains("No such entity") || msg.contains("does not exist") {
        ErrorKind::NotFound
    } else if msg.contains("Connection refused") || msg.contains("Connection terminated") {
        ErrorKind::PipeWireUnavailable
    } else if msg.contains("Access denied") || msg.contains("Permission denied") {
        ErrorKind::PermissionDenied
    } else {
        ErrorKind::PipeWireFailed
    };

    PwError::new(
        kind,
        if msg.is_empty() {
            format!("{program} {} failed", args.join(" "))
        } else {
            format!("{program}: {msg}")
        },
    )
}
