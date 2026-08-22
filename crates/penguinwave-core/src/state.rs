//! The daemon's owned state.

use crate::chatmix::ChatMixController;
use crate::config::ConfigStore;
use crate::error::Result;
use crate::events::EventBus;
use crate::sinks;
use crate::EqManager;
use penguinwave_hid::{DeviceChange, DeviceRegistry, HidBackend};
use penguinwave_pipewire::PipeWireBackend;
use penguinwave_proto::{DeviceId, Event, Snapshot};
use std::sync::{Arc, RwLock};

/// Everything the daemon owns, held by `Arc` and passed to request handlers.
///
/// Nothing here is a process global: a test constructs as many as it likes.
pub struct CoreState {
    pub backend: Arc<dyn PipeWireBackend>,
    pub hid: Arc<dyn HidBackend>,
    pub devices: Arc<RwLock<DeviceRegistry>>,
    pub eq: Arc<EqManager>,
    pub chatmix: Arc<ChatMixController>,
    pub config: ConfigStore,
    pub events: Arc<EventBus>,
}

impl CoreState {
    pub fn new(
        backend: Arc<dyn PipeWireBackend>,
        hid: Arc<dyn HidBackend>,
        config: ConfigStore,
    ) -> Arc<Self> {
        let events = Arc::new(EventBus::new());
        let devices = Arc::new(RwLock::new(DeviceRegistry::new(hid.clone())));
        let eq = EqManager::new(backend.clone(), config.clone(), events.clone());
        let chatmix = ChatMixController::new(backend.clone(), devices.clone(), events.clone());

        Arc::new(Self {
            backend,
            hid,
            devices,
            eq,
            chatmix,
            config,
            events,
        })
    }

    /// Bring the system to the desired state and start the background loops.
    ///
    /// Idempotent: the same path serves a cold start, a daemon restart and a
    /// PipeWire recovery, so it never assumes it is running first.
    pub fn start(self: &Arc<Self>) -> Result<()> {
        sinks::reconcile(&*self.backend)?;
        self.refresh_devices()?;
        self.eq.init();
        self.chatmix.start();
        Ok(())
    }

    /// Re-apply the desired state after PipeWire came back.
    pub fn resync(self: &Arc<Self>) -> Result<()> {
        let sinks = sinks::reconcile(&*self.backend)?;
        self.events.publish(Event::SinkListChanged { sinks });
        self.eq.init();
        Ok(())
    }

    /// Re-enumerate headsets and announce what changed.
    pub fn refresh_devices(&self) -> Result<Vec<DeviceChange>> {
        let changes = self.devices.write().unwrap().refresh()?;
        for change in &changes {
            self.events.publish(match *change {
                DeviceChange::Attached(device) => Event::DeviceAttached { device },
                DeviceChange::Detached(device) => Event::DeviceDetached { device },
            });
        }
        Ok(changes)
    }

    pub fn select_device(&self, id: DeviceId) -> Result<()> {
        self.devices.write().unwrap().select(id)?;
        self.events
            .publish(Event::DeviceSelectionChanged { device: Some(id) });
        Ok(())
    }

    pub fn clear_selection(&self) {
        self.devices.write().unwrap().clear_selection();
        self.events
            .publish(Event::DeviceSelectionChanged { device: None });
    }

    /// Everything a client needs to render from cold.
    pub fn snapshot(&self) -> Result<Snapshot> {
        let devices = self.devices.read().unwrap();
        Ok(Snapshot {
            streams: self.backend.list_application_streams()?,
            sinks: self.backend.list_sinks()?,
            default_sink: self.backend.default_sink().unwrap_or_default(),
            devices: devices.descriptors(),
            selected_device: devices.selected(),
            user_devices: self.config.load_user_devices(),
            chatmix: self.chatmix.state(),
            eq: self.eq.state(),
        })
    }

    /// Planned stop. Managed sinks and the EQ chain are left running so audio
    /// survives a daemon restart.
    pub fn shutdown(&self) {
        self.events.publish(Event::DaemonShuttingDown);
        self.chatmix.stop();
        self.eq.shutdown();
    }
}
