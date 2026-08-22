//! What a supported headset model can do.

use crate::error::{HidError, Result};
use crate::transport::HidTransport;
use penguinwave_proto::{Capability, DeviceId};
use std::fmt::Debug;

/// A headset model: a stateless description of one device's HID protocol.
///
/// Every operation takes the transport, so a model owns no connection and can
/// be shared across threads.
pub trait Headset: Send + Sync + Debug {
    fn id(&self) -> DeviceId;
    fn name(&self) -> &str;
    fn capabilities(&self) -> &[Capability];

    fn set_sidetone(&self, _t: &mut dyn HidTransport, _level: u8) -> Result<()> {
        Err(HidError::Unsupported("sidetone"))
    }

    fn set_microphone_volume(&self, _t: &mut dyn HidTransport, _level: u8) -> Result<()> {
        Err(HidError::Unsupported("microphone volume"))
    }

    /// Game/chat balance, 0 all chat to 100 all game.
    fn read_chatmix(&self, _t: &mut dyn HidTransport) -> Result<u8> {
        Err(HidError::Unsupported("chatmix"))
    }

    /// Charge percent, 0..=100.
    fn read_battery(&self, _t: &mut dyn HidTransport) -> Result<u8> {
        Err(HidError::Unsupported("battery status"))
    }
}

/// Linear rescale, saturating when the input range is empty.
pub(crate) fn map(x: i32, in_min: i32, in_max: i32, out_min: i32, out_max: i32) -> i32 {
    if in_max == in_min {
        return out_min;
    }
    (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}
