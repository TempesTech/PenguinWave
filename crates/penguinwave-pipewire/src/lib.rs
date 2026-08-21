//! PipeWire backend.
//!
//! Sole owner of every `pactl` / `pw-link` / `pw-dump` invocation. Callers use
//! [`PipeWireBackend`], never a subprocess, so the shell-out implementation can
//! be replaced with native bindings without touching them.

pub mod backend;
pub mod cmd;
pub mod icons;
pub mod pactl;
pub mod parse;
pub mod watch;

pub use backend::PipeWireBackend;
pub use pactl::PactlBackend;
pub use watch::{EventSink, WatchHandle};
