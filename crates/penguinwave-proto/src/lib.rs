//! Wire types for the Penguin Wave daemon protocol.
//!
//! No I/O: `serde` and `ts-rs` only. TypeScript bindings are generated from
//! these types (`cargo test -p penguinwave-proto export_bindings`).

pub mod envelope;
pub mod error;
pub mod models;

pub use envelope::*;
pub use error::*;
pub use models::audio::*;
pub use models::device::*;
pub use models::eq::*;
pub use models::event::*;
pub use models::request::*;
pub use models::response::*;

/// Protocol version carried on every frame.
pub const PROTO_VERSION: u16 = 1;

/// Inclusive range of protocol versions this build speaks.
pub const PROTO_SUPPORTED: (u16, u16) = (1, 1);

/// Maximum NDJSON frame length. Longer frames close the connection.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
