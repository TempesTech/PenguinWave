//! The boundary between headset models and `hidapi`.

use crate::error::{HidError, Result};
use hidapi::HidApi;
use penguinwave_proto::DeviceId;
use std::sync::Mutex;

/// An open HID channel to one device.
pub trait HidTransport: Send {
    fn write(&mut self, data: &[u8]) -> Result<()>;

    /// Returns the byte count read. A zero-length read is [`HidError::Timeout`].
    fn read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize>;
}

/// Enumerates and opens devices. The seam that lets models be tested without
/// hardware.
pub trait HidBackend: Send + Sync {
    fn list(&self) -> Result<Vec<DeviceId>>;
    fn open(&self, id: DeviceId) -> Result<Box<dyn HidTransport>>;
    fn available(&self) -> bool;
}

/// `hidapi`-backed implementation.
///
/// Holds a single [`HidApi`]; the library rejects a second instance.
pub struct HidApiBackend {
    api: Mutex<HidApi>,
}

impl HidApiBackend {
    pub fn new() -> Result<Self> {
        let api = HidApi::new().map_err(|e| HidError::ApiUnavailable(e.to_string()))?;
        Ok(Self {
            api: Mutex::new(api),
        })
    }
}

impl HidBackend for HidApiBackend {
    fn list(&self) -> Result<Vec<DeviceId>> {
        let mut api = self.api.lock().map_err(|_| poisoned())?;
        api.refresh_devices()
            .map_err(|e| HidError::Io(e.to_string()))?;
        let mut ids: Vec<DeviceId> = api
            .device_list()
            .map(|i| DeviceId::new(i.vendor_id(), i.product_id()))
            .collect();
        ids.sort_by_key(|d| (d.vendor_id, d.product_id));
        ids.dedup();
        Ok(ids)
    }

    fn open(&self, id: DeviceId) -> Result<Box<dyn HidTransport>> {
        let api = self.api.lock().map_err(|_| poisoned())?;
        match api.open(id.vendor_id, id.product_id) {
            Ok(device) => Ok(Box::new(HidApiTransport { device })),
            Err(e) => Err(classify_open(&e.to_string(), id)),
        }
    }

    fn available(&self) -> bool {
        true
    }
}

/// `hidapi` reports open failures as one opaque error, so the message is all
/// there is to go on.
fn classify_open(msg: &str, id: DeviceId) -> HidError {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("permission") || lower.contains("access") {
        HidError::PermissionDenied(id.vendor_id, id.product_id)
    } else {
        HidError::Absent(id.vendor_id, id.product_id)
    }
}

fn poisoned() -> HidError {
    HidError::Io("hidapi lock poisoned".into())
}

struct HidApiTransport {
    device: hidapi::HidDevice,
}

impl HidTransport for HidApiTransport {
    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.device
            .write(data)
            .map(|_| ())
            .map_err(|e| HidError::Io(e.to_string()))
    }

    fn read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        match self.device.read_timeout(buf, timeout_ms) {
            Ok(0) => Err(HidError::Timeout),
            Ok(n) => Ok(n),
            Err(e) => Err(HidError::Io(e.to_string())),
        }
    }
}
