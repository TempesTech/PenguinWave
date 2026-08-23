//! HID headset support.
//!
//! Sole owner of `hidapi`. Holds the headset trait, per-model implementations,
//! and the registry that tracks which devices are present and which one is
//! selected.

pub mod error;
pub mod headset;
#[cfg(feature = "mock")]
pub mod mock;
pub mod models;
pub mod registry;
pub mod transport;
pub mod userdev;

pub use error::{HidError, Result};
pub use headset::Headset;
#[cfg(feature = "mock")]
pub use mock::MockBackend;
pub use registry::{DeviceChange, DeviceRegistry};
pub use transport::{HidApiBackend, HidBackend, HidTransport};
