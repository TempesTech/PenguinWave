//! The daemon's systemd user unit.
//!
//! Packaging cannot enable it: user units are per-user and package scripts run
//! as root. The UI offers instead, because it already knows which user is
//! sitting in front of it.

use std::process::Command;

pub const UNIT: &str = "penguinwave.service";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitState {
    Enabled,
    Disabled,
    /// systemd is not managing this session, or the unit is not installed.
    Unavailable,
}

fn systemctl(args: &[&str]) -> Option<std::process::Output> {
    Command::new("systemctl").args(args).output().ok()
}

pub fn state() -> UnitState {
    let Some(output) = systemctl(&["--user", "is-enabled", UNIT]) else {
        return UnitState::Unavailable;
    };
    let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match answer.as_str() {
        "enabled" | "enabled-runtime" | "static" | "linked" => UnitState::Enabled,
        "disabled" | "inactive" => UnitState::Disabled,
        // "not-found" and anything unrecognised: nothing to offer the user.
        _ => UnitState::Unavailable,
    }
}

/// Enable and start the unit for this user.
///
/// A user unit needs no elevation, so this never prompts and never touches
/// anything outside the caller's own session.
pub fn enable() -> Result<(), String> {
    let output = systemctl(&["--user", "enable", "--now", UNIT])
        .ok_or_else(|| "systemctl is not available".to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("could not enable {UNIT}")
    } else {
        stderr
    })
}
