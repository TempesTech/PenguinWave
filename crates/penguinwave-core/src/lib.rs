//! Orchestration.
//!
//! Owns the authoritative state, the ChatMix policy loop, the EQ manager,
//! configuration persistence, and the event bus. Reaches hardware only through
//! the `penguinwave-pipewire` and `penguinwave-hid` traits, so the whole crate
//! is testable against mocks with no audio server and no socket.
//!
//! Contains no transport: it does not know that sockets exist.
//!
//! Filled in at P3. See `claudedocs/workflow_daemon_split.md`.
