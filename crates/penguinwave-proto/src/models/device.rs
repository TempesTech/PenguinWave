//! Headsets: supported models, user-defined entries, ChatMix.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Stable identity of a headset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeviceId {
    pub vendor_id: u16,
    pub product_id: u16,
}

impl DeviceId {
    pub fn new(vendor_id: u16, product_id: u16) -> Self {
        Self {
            vendor_id,
            product_id,
        }
    }

    /// Lower-case 4-digit hex, the form udev rules match on.
    pub fn vendor_id_hex(&self) -> String {
        format!("{:04x}", self.vendor_id)
    }

    pub fn product_id_hex(&self) -> String {
        format!("{:04x}", self.product_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Capability {
    Sidetone,
    BatteryStatus,
    NotificationSound,
    Lights,
    InactiveTime,
    ChatMixStatus,
    VoicePrompts,
    RotateToMute,
    EqualizerPreset,
    Equalizer,
    ParametricEqualizer,
    MicrophoneMuteLedBrightness,
    MicrophoneVolume,
    VolumeLimiter,
    BtWhenPoweredOn,
    BtCallVolume,
}

/// Whether a device is reachable right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DevicePresence {
    Connected,
    /// Not plugged in or powered off. Expected, not a failure.
    Absent,
    /// Present, but a HID transaction failed.
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BatteryInfo {
    pub presence: DevicePresence,
    /// Percent 0..=100.
    pub level: Option<u8>,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            presence: DevicePresence::Absent,
            level: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeviceDescriptor {
    #[serde(flatten)]
    pub id: DeviceId,
    pub name: String,
    pub presence: DevicePresence,
    pub capabilities: Vec<Capability>,
}

/// A device the user added by hand.
///
/// Ids are 4-digit hex strings; validate with [`is_valid_hex_id`] before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserDevice {
    pub name: String,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub pipewire_sink: Option<String>,
}

/// A USB vendor/product id is exactly 4 hex digits.
///
/// Security boundary: these values are interpolated into a root-owned udev
/// rules file, where anything else could inject directives such as `RUN+=`.
pub fn is_valid_hex_id(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Game/chat balance. 0 is all chat, 100 is all game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChatMix {
    pub value: u8,
    /// Set by hand rather than read off the headset wheel.
    pub manual: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemDeps {
    pub pipewire: bool,
    pub pactl: bool,
    pub pw_link: bool,
    pub libhidapi: bool,
}

/// Whether the udev rule granting HID access is installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UdevStatus {
    pub installed: bool,
    /// Argv for the client to run with elevation. The daemon never calls
    /// `pkexec` itself; it stays unprivileged.
    pub install_command: Option<Vec<String>>,
}
