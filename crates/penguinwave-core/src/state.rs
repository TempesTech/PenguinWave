//! The daemon's owned state.

use crate::chatmix::ChatMixController;
use crate::config::ConfigStore;
use crate::error::Result;
use crate::events::EventBus;
use crate::routes::{self, Routes};
use crate::sinks;
use crate::EqManager;
use penguinwave_hid::{DeviceChange, DeviceRegistry, HidBackend};
use penguinwave_pipewire::{PipeWireBackend, WatchHandle};
use penguinwave_proto::{DeviceId, Event, Snapshot};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

/// How often headsets are re-enumerated.
///
/// `hidapi` has no attach notification here, so presence is polled. Slow
/// enough to be free, fast enough that plugging a headset in feels immediate.
const DEVICE_POLL: Duration = Duration::from_secs(2);

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
    /// Dropping this stops the graph watch, so it is held for the daemon's
    /// lifetime rather than discarded at the end of `start`.
    watch: Mutex<Option<WatchHandle>>,
    device_poll_running: AtomicBool,
    /// Desired output routing, persisted so it survives a restart.
    routes: Mutex<Routes>,
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
        let routes = Mutex::new(config.load_routes());

        Arc::new(Self {
            backend,
            hid,
            devices,
            eq,
            chatmix,
            config,
            events,
            watch: Mutex::new(None),
            device_poll_running: AtomicBool::new(false),
            routes,
        })
    }

    /// Bring the system to the desired state and start the background loops.
    ///
    /// Idempotent: the same path serves a cold start, a daemon restart and a
    /// PipeWire recovery, so it never assumes it is running first.
    ///
    /// A failing step is reported but does not stop the rest. The loops are
    /// what recover from a bad start; skipping them because PipeWire was down
    /// for a moment leaves the daemon permanently inert.
    pub fn start(self: &Arc<Self>) -> Result<()> {
        let reconciled = sinks::reconcile(&*self.backend);
        if let Err(e) = &reconciled {
            eprintln!("[core] sink reconcile failed: {e}");
        }
        if let Err(e) = self.refresh_devices() {
            eprintln!("[core] device enumeration failed: {e}");
        }

        self.eq.init();
        if let Err(e) = self.reconcile_routes() {
            eprintln!("[core] route reconcile failed: {e}");
        }
        self.chatmix.start();
        self.start_device_poll();
        self.start_graph_watch();

        reconciled.map(|_| ())
    }

    /// Publish graph changes the daemon did not cause.
    ///
    /// Without this the only events are the ones a client's own mutation
    /// produced, so anything done elsewhere -- an application starting
    /// playback, `pavucontrol`, a cable pulled -- never reaches the UI and it
    /// silently goes stale.
    fn start_graph_watch(self: &Arc<Self>) {
        if self.watch.lock().unwrap().is_some() {
            return;
        }
        let state = Arc::clone(self);
        let sink: penguinwave_pipewire::EventSink = Arc::new(move || state.publish_graph());

        match self.backend.watch(sink) {
            Ok(handle) => *self.watch.lock().unwrap() = Some(handle),
            Err(e) => eprintln!("[core] graph watch unavailable: {e}"),
        }
    }

    /// Remember where a sink should point, and put it there.
    pub fn set_route(&self, sink: &str, device: &str) -> Result<()> {
        let routes = {
            let mut routes = self.routes.lock().unwrap();
            routes.set(sink, device);
            routes.clone()
        };
        self.config.save_routes(&routes)?;
        Ok(())
    }

    pub fn routes(&self) -> Routes {
        self.routes.lock().unwrap().clone()
    }

    /// Re-link sinks whose device came back.
    pub fn reconcile_routes(&self) -> Result<Vec<String>> {
        let routes = self.routes();
        if routes.0.is_empty() {
            return Ok(Vec::new());
        }
        routes::reconcile(&*self.backend, &routes, self.eq.is_active())
    }

    /// One graph change fans out to the three lists that can have moved.
    fn publish_graph(&self) {
        // A PipeWire restart wipes game_sink/chat_sink along with everything
        // else; only `start()` used to re-create them, so a live crash+restart
        // left the daemon running but permanently missing its own sinks. Adopt-
        // or-create here too, before routing and the lists go out.
        if let Err(e) = sinks::reconcile(&*self.backend) {
            eprintln!("[core] sink reconcile failed: {e}");
        }

        // A node reappearing is the common case for a graph change, and it
        // arrives with no links, so routing is restored before the lists go
        // out and clients render them.
        match self.reconcile_routes() {
            Ok(restored) if !restored.is_empty() => {
                eprintln!("[core] restored routing for {}", restored.join(", "));
            }
            Err(e) => eprintln!("[core] route reconcile failed: {e}"),
            _ => {}
        }

        self.events.publish(Event::GraphChanged);
        if let Ok(streams) = self.backend.list_application_streams() {
            self.events.publish(Event::StreamListChanged { streams });
        }
        if let Ok(sinks) = self.backend.list_sinks() {
            self.events.publish(Event::SinkListChanged { sinks });
        }
    }

    /// Re-enumerate headsets so one plugged in later is noticed.
    fn start_device_poll(self: &Arc<Self>) {
        if self.device_poll_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let state = Arc::clone(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(DEVICE_POLL);
            if let Err(e) = state.refresh_devices() {
                eprintln!("[core] device enumeration failed: {e}");
            }
        });
    }

    /// Re-apply the desired state after PipeWire came back.
    pub fn resync(self: &Arc<Self>) -> Result<()> {
        let sinks = sinks::reconcile(&*self.backend)?;
        self.events.publish(Event::SinkListChanged { sinks });
        self.eq.init();
        // The watch thread dies with the audio server it was polling.
        *self.watch.lock().unwrap() = None;
        self.start_graph_watch();
        self.chatmix.start();
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
        *self.watch.lock().unwrap() = None;
        self.chatmix.stop();
        self.eq.shutdown();
    }
}
