//! SteelSeries headsets.

mod arctis_nova_7;

pub use arctis_nova_7::ArctisNova7;

pub const VENDOR_ID: u16 = 0x1038;
pub const VENDOR_NAME: &str = "SteelSeries";

/// Prefix requesting the status report that carries ChatMix and battery.
pub const DATA_REQUEST: [u8; 2] = [0x00, 0xb0];

/// Midpoint of the raw ChatMix scale, which runs 0..=128.
pub const CHATMIX_MIDDLE: i32 = 64;
