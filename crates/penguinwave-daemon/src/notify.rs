//! Minimal `sd_notify`.
//!
//! One datagram to `$NOTIFY_SOCKET`, so `Type=notify` reports readiness when
//! the socket is actually accepting rather than when the process spawned.

use std::os::unix::net::UnixDatagram;

pub fn notify(state: &str) {
    let Some(path) = std::env::var_os("NOTIFY_SOCKET") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    let Ok(socket) = UnixDatagram::unbound() else {
        return;
    };

    // A leading '@' means the abstract namespace, which Rust's UnixDatagram
    // addresses with a leading NUL.
    let bytes = path.to_string_lossy();
    let target = if let Some(rest) = bytes.strip_prefix('@') {
        format!("\0{rest}")
    } else {
        bytes.into_owned()
    };

    let _ = socket.send_to(state.as_bytes(), target);
}

pub fn ready() {
    notify("READY=1");
}

pub fn stopping() {
    notify("STOPPING=1");
}
