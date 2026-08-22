//! ChatMix policy: turn a headset wheel position into sink volumes.

use crate::error::Result;
use crate::events::EventBus;
use crate::sinks;
use penguinwave_hid::DeviceRegistry;
use penguinwave_pipewire::PipeWireBackend;
use penguinwave_proto::{ChatMix, Event, CHAT_SINK, GAME_SINK};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

/// Wheel poll interval. Fast enough to feel immediate on a physical dial.
const POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Retry interval while no headset is selected.
const IDLE_INTERVAL: Duration = Duration::from_secs(2);

pub struct ChatMixController {
    backend: Arc<dyn PipeWireBackend>,
    devices: Arc<RwLock<DeviceRegistry>>,
    events: Arc<EventBus>,
    state: Mutex<ChatMix>,
    running: AtomicBool,
}

impl ChatMixController {
    pub fn new(
        backend: Arc<dyn PipeWireBackend>,
        devices: Arc<RwLock<DeviceRegistry>>,
        events: Arc<EventBus>,
    ) -> Arc<Self> {
        Arc::new(Self {
            backend,
            devices,
            events,
            state: Mutex::new(ChatMix {
                value: 50,
                manual: false,
            }),
            running: AtomicBool::new(false),
        })
    }

    pub fn state(&self) -> ChatMix {
        *self.state.lock().unwrap()
    }

    /// Pin the balance by hand, or hand control back to the wheel.
    pub fn set_manual(&self, value: Option<u8>) -> Result<()> {
        let next = match value {
            Some(v) => ChatMix {
                value: v.min(100),
                manual: true,
            },
            None => ChatMix {
                value: self.state().value,
                manual: false,
            },
        };
        self.apply(next)
    }

    /// Read the wheel once and apply it, unless a manual value is pinned.
    pub fn poll_once(&self) -> Result<bool> {
        if self.state().manual {
            return Ok(false);
        }
        let Some(id) = self.devices.read().unwrap().selected() else {
            return Ok(false);
        };
        let value = self.devices.read().unwrap().read_chatmix(id)?;

        if value == self.state().value {
            return Ok(false);
        }
        self.apply(ChatMix {
            value,
            manual: false,
        })?;
        Ok(true)
    }

    /// Write the balance to the sinks and announce it.
    ///
    /// Volumes are only written once both managed sinks exist; before that the
    /// value is still recorded so clients see the wheel move.
    fn apply(&self, next: ChatMix) -> Result<()> {
        let sinks = self.backend.list_sinks()?;
        if sinks::sinks_ready(&sinks) {
            self.backend.set_sink_volume(GAME_SINK, next.value)?;
            self.backend.set_sink_volume(CHAT_SINK, 100 - next.value)?;
        }
        *self.state.lock().unwrap() = next;
        self.events.publish(Event::ChatmixChanged(next));
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Poll the wheel until [`Self::stop`].
    pub fn start(self: &Arc<Self>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let controller = Arc::clone(self);
        std::thread::spawn(move || {
            while controller.running.load(Ordering::SeqCst) {
                let idle = controller.devices.read().unwrap().selected().is_none();
                if idle {
                    std::thread::sleep(IDLE_INTERVAL);
                    continue;
                }
                if let Err(e) = controller.poll_once() {
                    // An unplugged headset is expected; do not spin on it.
                    eprintln!("[chatmix] {e}");
                    std::thread::sleep(IDLE_INTERVAL);
                    continue;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        });
    }
}
