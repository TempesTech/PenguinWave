//! HID headset support.
//!
//! Sole owner of `hidapi`. Holds the headset trait, per-model implementations,
//! and the registry that tracks which devices are present and which one is
//! selected.

pub mod domain;
pub mod error;
#[cfg(feature = "mock")]
pub mod mock;
pub mod models;
pub mod transport;

pub use domain::headset;
pub use domain::registry;
pub use error::{HidError, Result};
pub use headset::Headset;
#[cfg(feature = "mock")]
pub use mock::MockBackend;
pub use registry::{DeviceChange, DeviceRegistry};
pub use transport::userdev;
pub use transport::{HidApiBackend, HidBackend, HidTransport};
