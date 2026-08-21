//! Headsets: supported models, user-defined entries, ChatMix.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Stable identity of a headset.
///
/// Replaces the `selected_device: i8` index into a `Vec` used before the
/// daemon split. An index silently retargets whenever the device list changes
/// order or length; a `(vendor, product)` pair does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
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
}

/// Features a headset exposes over HID.
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
///
/// Split out of the old `DeviceStatus`, which conflated "unplugged" with "HID
/// read failed". The daemon reports the first as an ordinary state transition
/// the UI renders calmly, and the second as an error worth logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DevicePresence {
    /// Present and responding.
    Connected,
    /// Not plugged in, or powered off. Expected, not a failure.
    Absent,
    /// Present but a HID transaction failed. Worth a bug report.
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BatteryInfo {
    pub presence: DevicePresence,
    /// Percent 0..=100, or `None` when unknown.
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

/// A headset model the daemon knows how to talk to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeviceDescriptor {
    #[serde(flatten)]
    pub id: DeviceId,
    pub name: String,
    pub presence: DevicePresence,
    pub capabilities: Vec<Capability>,
}

/// A device the user added by hand, for hardware with no built-in support.
///
/// `vendor_id` / `product_id` stay `String` on the wire because they are
/// entered as 4-digit hex and written into a root-owned udev rules file.
/// Validate with [`is_valid_hex_id`] before interpolating them anywhere.
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
/// Values in this shape are safe to interpolate into udev `ATTRS{...}=="..."`
/// match strings. Anything else risks breaking out of the quoted match and
/// injecting extra udev directives (e.g. `RUN+=...`) into a root-owned rules
/// file, so this check is a security boundary, not input tidying.
pub fn is_valid_hex_id(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Game/chat balance, 0..=100. 0 is all chat, 100 is all game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChatMix {
    pub value: u8,
    /// True when set from the UI/CLI rather than read off the headset wheel.
    pub manual: bool,
}

/// Result of the environment checks behind the Maintenance page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemDeps {
    pub pipewire: bool,
    pub pactl: bool,
    pub pw_link: bool,
    pub libhidapi: bool,
}

/// Whether the udev rule granting HID access is installed.
///
/// When missing, the daemon returns the command for the client to run under
/// its own polkit agent. The daemon never invokes `pkexec` itself: it runs
/// unprivileged, always, and acquiring root even transiently would make the
/// socket's `0600` permission the wrong security boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UdevStatus {
    pub installed: bool,
    /// Populated when `installed` is false: the exact argv the client should
    /// run with elevation.
    pub install_command: Option<Vec<String>>,
}
