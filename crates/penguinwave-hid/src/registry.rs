//! Which headsets are known, which are plugged in, and which one is selected.

use crate::error::{HidError, Result};
use crate::headset::Headset;
use crate::models;
use crate::transport::{HidBackend, HidTransport};
use penguinwave_proto::{BatteryInfo, DeviceDescriptor, DeviceId, DevicePresence};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// A change in what is plugged in, produced by [`DeviceRegistry::refresh`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceChange {
    Attached(DeviceId),
    Detached(DeviceId),
}

/// Owns the supported-model table and the presence set.
///
/// Devices are addressed by [`DeviceId`] throughout; nothing is indexed by
/// position, so a model list that grows cannot silently reassign a selection.
pub struct DeviceRegistry {
    backend: Arc<dyn HidBackend>,
    models: HashMap<DeviceId, Arc<dyn Headset>>,
    present: BTreeSet<DeviceId>,
    selected: Option<DeviceId>,
}

impl DeviceRegistry {
    /// Registry over every model this build knows.
    pub fn new(backend: Arc<dyn HidBackend>) -> Self {
        Self::with_models(backend, models::all())
    }

    pub fn with_models(backend: Arc<dyn HidBackend>, models: Vec<Arc<dyn Headset>>) -> Self {
        Self {
            backend,
            models: models.into_iter().map(|m| (m.id(), m)).collect(),
            present: BTreeSet::new(),
            selected: None,
        }
    }

    /// Re-enumerate and report what changed.
    ///
    /// Selects the first supported device to appear while nothing is selected.
    /// A selection is never cleared by a detach: the headset is reported
    /// absent and the choice survives being powered off.
    pub fn refresh(&mut self) -> Result<Vec<DeviceChange>> {
        let listed = self.backend.list()?;
        let now: BTreeSet<DeviceId> = listed
            .into_iter()
            .filter(|id| self.models.contains_key(id))
            .collect();

        let mut changes = Vec::new();
        for id in now.difference(&self.present) {
            changes.push(DeviceChange::Attached(*id));
        }
        for id in self.present.difference(&now) {
            changes.push(DeviceChange::Detached(*id));
        }
        self.present = now;

        if self.selected.is_none() {
            self.selected = self.present.iter().next().copied();
        }
        Ok(changes)
    }

    pub fn selected(&self) -> Option<DeviceId> {
        self.selected
    }

    pub fn select(&mut self, id: DeviceId) -> Result<()> {
        if !self.models.contains_key(&id) {
            return Err(HidError::Unknown(id.vendor_id, id.product_id));
        }
        self.selected = Some(id);
        Ok(())
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    pub fn is_present(&self, id: DeviceId) -> bool {
        self.present.contains(&id)
    }

    pub fn model(&self, id: DeviceId) -> Result<Arc<dyn Headset>> {
        self.models
            .get(&id)
            .cloned()
            .ok_or(HidError::Unknown(id.vendor_id, id.product_id))
    }

    pub fn descriptors(&self) -> Vec<DeviceDescriptor> {
        let mut out: Vec<DeviceDescriptor> = self
            .models
            .values()
            .map(|m| DeviceDescriptor {
                id: m.id(),
                name: m.name().to_string(),
                presence: if self.present.contains(&m.id()) {
                    DevicePresence::Connected
                } else {
                    DevicePresence::Absent
                },
                capabilities: m.capabilities().to_vec(),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Open the device, refusing early if it was not seen by the last refresh.
    pub fn open(&self, id: DeviceId) -> Result<Box<dyn HidTransport>> {
        if !self.models.contains_key(&id) {
            return Err(HidError::Unknown(id.vendor_id, id.product_id));
        }
        if !self.present.contains(&id) {
            return Err(HidError::Absent(id.vendor_id, id.product_id));
        }
        self.backend.open(id)
    }

    /// Run one HID transaction against a device.
    pub fn with_device<T>(
        &self,
        id: DeviceId,
        f: impl FnOnce(&dyn Headset, &mut dyn HidTransport) -> Result<T>,
    ) -> Result<T> {
        let model = self.model(id)?;
        let mut transport = self.open(id)?;
        f(model.as_ref(), transport.as_mut())
    }

    pub fn set_sidetone(&self, id: DeviceId, level: u8) -> Result<()> {
        self.with_device(id, |m, t| m.set_sidetone(t, level))
    }

    pub fn set_microphone_volume(&self, id: DeviceId, level: u8) -> Result<()> {
        self.with_device(id, |m, t| m.set_microphone_volume(t, level))
    }

    pub fn read_chatmix(&self, id: DeviceId) -> Result<u8> {
        self.with_device(id, |m, t| m.read_chatmix(t))
    }

    /// Battery as reported to clients: a failed read is `Faulted`, not an error.
    pub fn battery(&self, id: DeviceId) -> BatteryInfo {
        match self.with_device(id, |m, t| m.read_battery(t)) {
            Ok(level) => BatteryInfo {
                presence: DevicePresence::Connected,
                level: Some(level),
            },
            Err(e) if e.is_absent() => BatteryInfo::default(),
            Err(_) => BatteryInfo {
                presence: DevicePresence::Faulted,
                level: None,
            },
        }
    }
}
