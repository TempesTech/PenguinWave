//! PipeWire backend.
//!
//! Sole owner of every `pactl` / `pw-link` / `pw-dump` invocation. Callers use
//! [`PipeWireBackend`], never a subprocess, so the shell-out implementation can
//! be replaced with native bindings without touching them.

pub mod domain;
pub mod icons;
#[cfg(feature = "mock")]
pub mod mock;
pub mod transport;

pub use domain::backend;
pub use domain::chain;
pub use domain::parse;
pub use backend::PipeWireBackend;
pub use chain::ChainProcess;
#[cfg(feature = "mock")]
pub use mock::MockBackend;
pub use transport::cmd;
pub use transport::pactl;
pub use transport::pactl::PactlBackend;
pub use transport::watch;
pub use watch::{EventSink, WatchHandle};
