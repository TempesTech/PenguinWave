//! User-defined devices, stored in `$XDG_CONFIG_HOME/penguinwave/devices.toml`.

use crate::error::{HidError, Result};
use penguinwave_proto::{is_valid_hex_id, UserDevice};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct UserDevicesFile {
    #[serde(default)]
    devices: Vec<UserDevice>,
}

pub fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("penguinwave").join("devices.toml")
}

/// Load from `path`. A missing or unparsable file yields an empty list.
pub fn load_from(path: &Path) -> Vec<UserDevice> {
    fs::read_to_string(path)
        .ok()
        .and_then(|c| toml::from_str::<UserDevicesFile>(&c).ok())
        .map(|f| f.devices)
        .unwrap_or_default()
}

pub fn load() -> Vec<UserDevice> {
    load_from(&config_path())
}

pub fn save_to(path: &Path, devices: &[UserDevice]) -> Result<()> {
    for d in devices {
        validate(d)?;
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| HidError::Io(e.to_string()))?;
    }
    let content = toml::to_string(&UserDevicesFile {
        devices: devices.to_vec(),
    })
    .map_err(|e| HidError::Io(e.to_string()))?;
    fs::write(path, content).map_err(|e| HidError::Io(e.to_string()))
}

pub fn save(devices: &[UserDevice]) -> Result<()> {
    save_to(&config_path(), devices)
}

/// Reject ids that are not exactly 4 hex digits.
///
/// Security boundary: these values reach a root-owned udev rules file, where
/// anything else could inject directives such as `RUN+=`.
pub fn validate(device: &UserDevice) -> Result<()> {
    if device.name.trim().is_empty() {
        return Err(HidError::Protocol("device name is empty".into()));
    }
    for id in [&device.vendor_id, &device.product_id]
        .into_iter()
        .flatten()
    {
        if !is_valid_hex_id(id) {
            return Err(HidError::Protocol(format!("invalid hex id {id:?}")));
        }
    }
    Ok(())
}
