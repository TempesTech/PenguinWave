//! SteelSeries Arctis Nova 7.

use super::{CHATMIX_MIDDLE, DATA_REQUEST, VENDOR_ID};
use crate::error::{HidError, Result};
use crate::headset::{map, Headset};
use crate::transport::HidTransport;
use penguinwave_proto::{Capability, DeviceId};

const PRODUCT_ID: u16 = 0x2202;
const NAME: &str = "SteelSeries Arctis Nova 7";
const READ_TIMEOUT_MS: i32 = 1000;

const CAPABILITIES: &[Capability] = &[
    Capability::Sidetone,
    Capability::MicrophoneVolume,
    Capability::ChatMixStatus,
    Capability::BatteryStatus,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct ArctisNova7;

impl ArctisNova7 {
    pub fn new() -> Self {
        Self
    }

    fn status_report(&self, t: &mut dyn HidTransport, buf: &mut [u8]) -> Result<()> {
        t.write(&DATA_REQUEST)?;
        t.read(buf, READ_TIMEOUT_MS)?;
        Ok(())
    }
}

impl Headset for ArctisNova7 {
    fn id(&self) -> DeviceId {
        DeviceId::new(VENDOR_ID, PRODUCT_ID)
    }

    fn name(&self) -> &str {
        NAME
    }

    fn capabilities(&self) -> &[Capability] {
        CAPABILITIES
    }

    fn set_sidetone(&self, t: &mut dyn HidTransport, level: u8) -> Result<()> {
        let step = match level {
            0..=25 => 0x0,
            26..=50 => 0x1,
            51..=75 => 0x2,
            _ => 0x3,
        };
        t.write(&[0x00, 0x39, step])
    }

    fn set_microphone_volume(&self, t: &mut dyn HidTransport, level: u8) -> Result<()> {
        t.write(&[0x00, 0x37, (level / 16).min(7)])
    }

    fn read_chatmix(&self, t: &mut dyn HidTransport) -> Result<u8> {
        let mut buf = [0u8; 64];
        self.status_report(t, &mut buf)?;
        Ok(chatmix_from_raw(buf[4], buf[5]))
    }

    fn read_battery(&self, t: &mut dyn HidTransport) -> Result<u8> {
        let mut buf = [0u8; 8];
        self.status_report(t, &mut buf)?;
        battery_from_raw(buf[2])
    }
}

/// The headset reports the two wheel halves separately, each 0..=100.
///
/// The pre-split app mapped them onto a 0..=128 scale where larger meant more
/// chat; this rescales that to the protocol's 0..=100 where larger means more
/// game.
fn chatmix_from_raw(game_raw: u8, chat_raw: u8) -> u8 {
    let game = map(game_raw as i32, 0, 100, 0, 64);
    let chat = map(chat_raw as i32, 0, 100, 0, 64);
    let legacy = CHATMIX_MIDDLE - (game - chat);
    (((128 - legacy) * 100) / 128).clamp(0, 100) as u8
}

/// Charge is reported as a level 1..=4, not a percentage.
fn battery_from_raw(raw: u8) -> Result<u8> {
    match raw {
        1 => Ok(25),
        2 => Ok(50),
        3 => Ok(75),
        4 => Ok(100),
        other => Err(HidError::Protocol(format!("battery level {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chatmix_endpoints_and_centre() {
        assert_eq!(chatmix_from_raw(100, 0), 100);
        assert_eq!(chatmix_from_raw(0, 100), 0);
        assert_eq!(chatmix_from_raw(0, 0), 50);
        assert_eq!(chatmix_from_raw(100, 100), 50);
    }

    #[test]
    fn chatmix_never_panics_on_out_of_range_bytes() {
        for g in 0..=255u8 {
            for c in [0u8, 100, 200, 255] {
                let v = chatmix_from_raw(g, c);
                assert!(v <= 100);
            }
        }
    }

    #[test]
    fn battery_rejects_unknown_level() {
        assert_eq!(battery_from_raw(3), Ok(75));
        assert!(matches!(battery_from_raw(0), Err(HidError::Protocol(_))));
        assert!(matches!(battery_from_raw(9), Err(HidError::Protocol(_))));
    }
}
