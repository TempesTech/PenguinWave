//! On-disk state under `$XDG_CONFIG_HOME/penguinwave`.

use crate::error::{io_err, CoreError, Result};
use crate::api::routes::Routes;
use penguinwave_proto::{EqPreset, EqState, UserDevice};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Every file the daemon owns, rooted at one directory.
///
/// The root is injectable so tests never touch the real config.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    root: PathBuf,
}

impl ConfigStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `$XDG_CONFIG_HOME/penguinwave`, falling back to `$HOME/.config`.
    pub fn default_root() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(std::env::temp_dir)
            .join("penguinwave")
    }

    pub fn from_env() -> Self {
        Self::new(Self::default_root())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn eq_dir(&self) -> PathBuf {
        self.root.join("eq")
    }

    pub fn eq_state_path(&self) -> PathBuf {
        self.eq_dir().join("state.json")
    }

    pub fn eq_conf_path(&self) -> PathBuf {
        self.eq_dir().join("penguinwave-eq.conf")
    }

    pub fn presets_path(&self) -> PathBuf {
        self.eq_dir().join("presets.json")
    }

    /// Marker file, not a memory flag: safe mode exists for the crash case and
    /// must survive one.
    pub fn safe_mode_marker(&self) -> PathBuf {
        self.eq_dir().join(".safe_mode")
    }

    pub fn devices_path(&self) -> PathBuf {
        self.root.join("devices.toml")
    }

    pub fn routes_path(&self) -> PathBuf {
        self.root.join("routes.json")
    }

    /// Desired output routing, so a device that comes back is re-linked.
    pub fn load_routes(&self) -> Routes {
        read_json(&self.routes_path()).unwrap_or_default()
    }

    pub fn save_routes(&self, routes: &Routes) -> Result<()> {
        write_json_atomic(&self.routes_path(), routes)
    }

    /// Load EQ state, taking safe mode from the marker rather than the file.
    pub fn load_eq_state(&self) -> EqState {
        let mut state: EqState = read_json(&self.eq_state_path()).unwrap_or_default();
        state.safe_mode = self.safe_mode_marker().exists();
        state
    }

    pub fn save_eq_state(&self, state: &EqState) -> Result<()> {
        write_json_atomic(&self.eq_state_path(), state)
    }

    pub fn load_user_presets(&self) -> Vec<EqPreset> {
        read_json(&self.presets_path()).unwrap_or_default()
    }

    pub fn save_user_presets(&self, presets: &[EqPreset]) -> Result<()> {
        write_json_atomic(&self.presets_path(), &presets)
    }

    pub fn write_eq_conf(&self, content: &str) -> Result<PathBuf> {
        let path = self.eq_conf_path();
        write_atomic(&path, content.as_bytes())?;
        Ok(path)
    }

    pub fn set_safe_mode(&self, active: bool) -> Result<()> {
        let marker = self.safe_mode_marker();
        if active {
            write_atomic(&marker, b"")
        } else if marker.exists() {
            fs::remove_file(&marker).map_err(|e| io_err(&marker, e))
        } else {
            Ok(())
        }
    }

    pub fn safe_mode(&self) -> bool {
        self.safe_mode_marker().exists()
    }

    pub fn load_user_devices(&self) -> Vec<UserDevice> {
        penguinwave_hid::userdev::load_from(&self.devices_path())
    }

    pub fn save_user_devices(&self, devices: &[UserDevice]) -> Result<()> {
        penguinwave_hid::userdev::save_to(&self.devices_path(), devices)?;
        Ok(())
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| CoreError::Internal(format!("serialize {}: {e}", path.display())))?;
    write_atomic(path, json.as_bytes())
}

/// Write through a temporary file so a reader never sees a half-written file.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| io_err(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| io_err(path, e))
}
