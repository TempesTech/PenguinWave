//! Penguin Wave daemon.
//!
//! Transport only: socket server, session management, request dispatch onto
//! `penguinwave-core`, and event fan-out. Runs unprivileged, per user, under
//! `systemd --user`.
//!
//! Filled in at P4. See `claudedocs/workflow_daemon_split.md`.

fn main() {
    eprintln!("penguinwave-daemon: not implemented yet (P4)");
    std::process::exit(1);
}
