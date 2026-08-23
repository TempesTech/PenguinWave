//! Orchestration.
//!
//! Owns the authoritative state, the ChatMix policy loop, the EQ manager,
//! configuration persistence, and the event bus. Reaches hardware only through
//! the `penguinwave-pipewire` and `penguinwave-hid` traits, so the whole crate
//! is testable against mocks with no audio server and no socket.
//!
//! Contains no transport: it does not know that sockets exist.

pub mod api;
pub mod domain;
pub mod eq;
pub mod error;
pub mod state;

pub use api::dispatch;
pub use domain::config::ConfigStore;
pub use domain::events::EventBus;
pub use eq::EqManager;
pub use error::{CoreError, Result};
pub use state::CoreState;
