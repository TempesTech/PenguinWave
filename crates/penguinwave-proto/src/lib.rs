//! Wire types for the Penguin Wave daemon protocol.
//!
//! This crate is the single source of truth for everything that crosses the
//! unix socket between `penguinwave-daemon` and its clients (`penguinwave-cli`,
//! the Tauri UI, and anything else).
//!
//! Hard constraints, enforced by review:
//!
//! - **No I/O.** No `std::process`, no `std::fs`, no sockets, no `hidapi`.
//!   Depends on `serde` and `ts-rs` only, so it stays compilable for wasm.
//! - **No behavior.** Types and their invariants; the logic lives in
//!   `penguinwave-core`.
//!
//! TypeScript definitions are generated from these types (`cargo test
//! export_bindings`) rather than hand-maintained, because parallel type
//! definitions across a socket boundary drift, and the drift surfaces as a
//! runtime `undefined` instead of a compile error.

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
///
/// Bumped on any incompatible change to the wire format. The daemon rejects
/// clients outside its supported range rather than attempting to interoperate:
/// a half-understood protocol corrupts audio routing, and a loud failure beats
/// a subtle one.
pub const PROTO_VERSION: u16 = 1;

/// Inclusive range of protocol versions this build can speak.
pub const PROTO_SUPPORTED: (u16, u16) = (1, 1);

/// Maximum length of a single NDJSON frame, in bytes.
///
/// Frames longer than this close the connection. The realistic path to an
/// oversized frame is base64 application icons, which is why icons are fetched
/// per-key via [`request::Request::StreamIcon`] rather than inlined into
/// stream listings.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
