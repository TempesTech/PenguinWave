//! Session-bus interface alongside the Unix socket.
//!
//! One generic method mirrors the socket protocol exactly -- same
//! `Request`/`Response` JSON, so anything the socket can do this can do too,
//! with no second protocol to keep in sync. A few read-only properties cover
//! the common case (a Waybar widget, a shell script) without requiring the
//! caller to know the wire format at all.

use penguinwave_core::{dispatch, CoreState};
use penguinwave_proto::{Request, Response};
use std::sync::Arc;
use zbus::blocking::connection;
use zbus::{fdo, interface};

// Only `serve()` (main.rs's entry point) uses these; the dbus.rs test
// includes this file on its own via #[path], without main.rs, so they read
// as dead code there even though they're not in the real binary.
#[allow(dead_code)]
const BUS_NAME: &str = "com.penguinwave.Daemon";
#[allow(dead_code)]
const OBJECT_PATH: &str = "/com/penguinwave/Daemon1";

pub(crate) struct Daemon1 {
    pub(crate) state: Arc<CoreState>,
}

#[interface(name = "com.penguinwave.Daemon1")]
impl Daemon1 {
    /// Same request/response JSON as the socket protocol, so any action or
    /// query available there is available here too.
    fn call(&self, request_json: String) -> fdo::Result<String> {
        let request: Request = serde_json::from_str(&request_json)
            .map_err(|e| fdo::Error::InvalidArgs(format!("bad request: {e}")))?;
        let response: Response =
            dispatch(&self.state, request).map_err(|e| fdo::Error::Failed(e.to_string()))?;
        serde_json::to_string(&response)
            .map_err(|e| fdo::Error::Failed(format!("failed to encode response: {e}")))
    }

    #[zbus(property)]
    fn chat_mix(&self) -> u8 {
        self.state.chatmix.state().value
    }

    #[zbus(property)]
    fn chat_mix_manual(&self) -> bool {
        self.state.chatmix.state().manual
    }

    #[zbus(property)]
    fn eq_active(&self) -> bool {
        self.state.eq.is_active()
    }

    #[zbus(property)]
    fn default_sink(&self) -> String {
        self.state
            .snapshot()
            .map(|s| s.default_sink)
            .unwrap_or_default()
    }
}

/// Start serving on the session bus. Runs for the caller's lifetime; drop the
/// returned connection to stop.
///
/// A failure here (no session bus, name already taken by another instance)
/// is logged and otherwise ignored -- the socket is the interface every
/// client actually depends on, this is additive.
#[allow(dead_code)]
pub fn serve(state: Arc<CoreState>) -> Option<zbus::blocking::Connection> {
    match serve_at(BUS_NAME, OBJECT_PATH, state) {
        Ok(conn) => {
            eprintln!("[daemon] D-Bus: {BUS_NAME} at {OBJECT_PATH}");
            Some(conn)
        }
        Err(e) => {
            eprintln!("[daemon] D-Bus unavailable, continuing without it: {e}");
            None
        }
    }
}

/// The name and path are parameters so a test can use its own, isolated from
/// whatever real daemon instance may already hold the production name.
pub(crate) fn serve_at(
    bus_name: &str,
    object_path: &str,
    state: Arc<CoreState>,
) -> zbus::Result<zbus::blocking::Connection> {
    connection::Builder::session()?
        .name(bus_name.to_string())?
        .serve_at(object_path.to_string(), Daemon1 { state })?
        .build()
}
