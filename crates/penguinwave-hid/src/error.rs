//! HID failure taxonomy.

use penguinwave_proto::{ErrorKind, PwError};

pub type Result<T> = std::result::Result<T, HidError>;

/// Why a HID operation did not succeed.
///
/// `Absent` is kept apart from the transfer failures: an unplugged headset is
/// expected, a failed transfer on a present one is not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HidError {
    #[error("device {0:04x}:{1:04x} is not connected")]
    Absent(u16, u16),

    #[error("no driver for device {0:04x}:{1:04x}")]
    Unknown(u16, u16),

    #[error("hidapi unavailable: {0}")]
    ApiUnavailable(String),

    #[error("permission denied opening {0:04x}:{1:04x}; the udev rule is probably missing")]
    PermissionDenied(u16, u16),

    #[error("HID read timed out")]
    Timeout,

    #[error("HID transfer failed: {0}")]
    Io(String),

    #[error("unexpected response from device: {0}")]
    Protocol(String),

    #[error("device does not support {0}")]
    Unsupported(&'static str),
}

impl HidError {
    /// Whether the device answered at all.
    pub fn is_absent(&self) -> bool {
        matches!(self, HidError::Absent(..) | HidError::Unknown(..))
    }
}

impl From<HidError> for PwError {
    fn from(e: HidError) -> Self {
        let kind = match e {
            HidError::Absent(..) => ErrorKind::DeviceAbsent,
            HidError::Unknown(..) => ErrorKind::NotFound,
            HidError::PermissionDenied(..) => ErrorKind::PermissionDenied,
            HidError::Unsupported(_) => ErrorKind::BadRequest,
            _ => ErrorKind::Internal,
        };
        PwError::new(kind, e.to_string())
    }
}
