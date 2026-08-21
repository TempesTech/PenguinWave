//! Wire types for the Penguin Wave daemon protocol.
//!
//! No I/O: `serde` and `ts-rs` only. TypeScript bindings are generated from
//! these types (`cargo test -p penguinwave-proto export_bindings`).

pub mod audio;
pub mod device;
pub mod envelope;
pub mod eq;
pub mod error;
pub mod event;
pub mod request;
pub mod response;

pub use audio::*;
pub use device::*;
pub use envelope::*;
pub use eq::*;
pub use error::*;
pub use event::*;
pub use request::*;
pub use response::*;

/// Protocol version carried on every frame.
pub const PROTO_VERSION: u16 = 1;

/// Inclusive range of protocol versions this build speaks.
pub const PROTO_SUPPORTED: (u16, u16) = (1, 1);

/// Maximum NDJSON frame length. Longer frames close the connection.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
