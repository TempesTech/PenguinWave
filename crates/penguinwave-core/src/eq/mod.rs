//! Parametric equalizer over a PipeWire filter chain.
//!
//! The graph is fixed: `MAX_BANDS` `bq_raw` nodes plus a preamp per chain. All
//! edits are live coefficient pushes, so the config is never regenerated for a
//! band change.

pub mod biquad;
pub mod conf_gen;
pub mod live;
pub mod nodes;
pub mod presets;
pub mod process;
pub mod wiring;

use crate::config::ConfigStore;
use crate::error::{CoreError, Result};
use crate::events::EventBus;
use penguinwave_pipewire::{ChainProcess, PipeWireBackend};
use penguinwave_proto::{
    EqBand, EqChain, EqChainId, EqPresetMeta, EqState, Event, GAIN_MAX, GAIN_MIN, MAX_BANDS,
};
use process::CrashTracker;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SAMPLE_RATE: f32 = 48_000.0;
const PERSIST_INTERVAL: Duration = Duration::from_millis(500);
const SUPERVISE_INTERVAL: Duration = Duration::from_millis(500);

pub struct EqManager {
    backend: Arc<dyn PipeWireBackend>,
    store: ConfigStore,
    events: Arc<EventBus>,
    state: Mutex<EqState>,
    node_ids: Mutex<HashMap<EqChainId, u32>>,
    child: Mutex<Option<Box<dyn ChainProcess>>>,
    dirty: AtomicBool,
    running: AtomicBool,
}

impl EqManager {
    pub fn new(
        backend: Arc<dyn PipeWireBackend>,
        store: ConfigStore,
        events: Arc<EventBus>,
    ) -> Arc<Self> {
        let state = store.load_eq_state();
        Arc::new(EqManager {
            backend,
            store,
            events,
            state: Mutex::new(state),
            node_ids: Mutex::new(HashMap::new()),
            child: Mutex::new(None),
            dirty: AtomicBool::new(false),
            running: AtomicBool::new(false),
        })
    }

    /// Spawn the chain, resolve its nodes, apply state and wire the routing.
    ///
    /// Also starts the supervision and persistence threads. Idempotent enough
    /// to be called again after [`Self::reset_safe_mode`].
    pub fn init(self: &Arc<Self>) {
        if self.state.lock().unwrap().safe_mode {
            self.events.publish(Event::EqSafeMode { active: true });
            return;
        }
        if self.running.load(Ordering::SeqCst) && self.child.lock().unwrap().is_some() {
            return;
        }

        if let Err(e) = self.start_chain() {
            eprintln!("[eq] failed to start filter chain: {e}");
            self.enter_safe_mode();
            return;
        }

        if !self.running.swap(true, Ordering::SeqCst) {
            self.spawn_supervisor();
            self.spawn_persister();
        }
    }

    fn start_chain(self: &Arc<Self>) -> Result<()> {
        // Remember where each sink points so the EQ output can take that route.
        let mut routes: HashMap<EqChainId, String> = HashMap::new();
        for chain in EqChainId::ALL {
            if let Ok(Some(device)) = wiring::current_device_target(&*self.backend, chain) {
                routes.insert(chain, device);
            }
        }

        let conf = self.store.write_eq_conf(&conf_gen::generate_conf())?;
        let child = self.backend.spawn_filter_chain(&conf)?;
        *self.child.lock().unwrap() = Some(child);

        *self.node_ids.lock().unwrap() = process::wait_for_nodes(&*self.backend)?;
        self.apply_all()?;

        for chain in EqChainId::ALL {
            let device = routes
                .get(&chain)
                .cloned()
                .or_else(|| wiring::fallback_output_device(&*self.backend));
            match device {
                Some(device) => {
                    if let Err(e) = wiring::wire_through_eq(&*self.backend, chain, &device) {
                        eprintln!("[eq] wiring {chain:?} through the EQ failed: {e}");
                    }
                }
                None => eprintln!("[eq] no physical output device found for {chain:?}"),
            }
        }
        Ok(())
    }

    fn spawn_supervisor(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        std::thread::spawn(move || {
            let mut tracker = CrashTracker::default();
            loop {
                std::thread::sleep(SUPERVISE_INTERVAL);
                if !manager.running.load(Ordering::SeqCst) {
                    break;
                }
                if !manager.child_exited() {
                    continue;
                }

                eprintln!("[eq] filter chain exited");
                if tracker.record_exit() {
                    eprintln!("[eq] crash loop, entering safe mode");
                    manager.enter_safe_mode();
                    break;
                }
                std::thread::sleep(tracker.backoff());
                if let Err(e) = manager.start_chain() {
                    eprintln!("[eq] respawn failed: {e}");
                }
            }
        });
    }

    fn child_exited(&self) -> bool {
        let mut guard = self.child.lock().unwrap();
        match guard.as_mut().map(|c| c.try_wait()) {
            Some(Ok(Some(_))) => {
                *guard = None;
                true
            }
            _ => false,
        }
    }

    fn spawn_persister(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(PERSIST_INTERVAL);
            if !manager.running.load(Ordering::SeqCst) {
                break;
            }
            if manager.dirty.swap(false, Ordering::SeqCst) {
                manager.persist();
            }
        });
    }

    fn persist(&self) {
        let state = self.state.lock().unwrap().clone();
        if let Err(e) = self.store.save_eq_state(&state) {
            eprintln!("[eq] persist failed: {e}");
        }
    }

    /// Stop respawning, restore direct links, and record the marker.
    fn enter_safe_mode(self: &Arc<Self>) {
        self.running.store(false, Ordering::SeqCst);
        self.restore_direct_routing();
        self.kill_child();

        let _ = self.store.set_safe_mode(true);
        self.state.lock().unwrap().safe_mode = true;
        self.events.publish(Event::EqSafeMode { active: true });
    }

    /// Audio keeps flowing un-EQ'd rather than stopping.
    fn restore_direct_routing(&self) {
        for chain in EqChainId::ALL {
            let device = wiring::current_device_target(&*self.backend, chain)
                .ok()
                .flatten()
                .or_else(|| wiring::fallback_output_device(&*self.backend));
            if let Some(device) = device {
                let _ = wiring::wire_direct(&*self.backend, chain, &device);
            }
        }
    }

    fn kill_child(&self) {
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            child.kill();
        }
        *self.child.lock().unwrap() = None;
    }

    /// Clear the marker and start over, from the safe-mode banner.
    pub fn reset_safe_mode(self: &Arc<Self>) -> Result<()> {
        self.store.set_safe_mode(false)?;
        self.state.lock().unwrap().safe_mode = false;
        self.events.publish(Event::EqSafeMode { active: false });
        self.init();
        Ok(())
    }

    /// Restore direct routing, kill the chain, flush state.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.restore_direct_routing();
        self.kill_child();
        self.persist();
    }

    fn node_id(&self, chain: EqChainId) -> Result<u32> {
        self.node_ids
            .lock()
            .unwrap()
            .get(&chain)
            .copied()
            .ok_or_else(|| CoreError::NodeNotFound(nodes::sink_node_name(chain).to_string()))
    }

    fn chain(&self, chain_id: EqChainId) -> Result<EqChain> {
        self.state
            .lock()
            .unwrap()
            .chains
            .get(&chain_id)
            .cloned()
            .ok_or_else(|| CoreError::NodeNotFound(format!("{chain_id:?} chain")))
    }

    /// Push a whole chain: preamp plus every slot, in one call.
    fn apply_chain(&self, chain_id: EqChainId) -> Result<()> {
        let chain = self.chain(chain_id)?;
        let node_id = self.node_id(chain_id)?;

        let mut params = Vec::with_capacity((MAX_BANDS + 1) * 5);
        let preamp = if chain.enabled {
            biquad::gain(chain.preamp_db)
        } else {
            biquad::IDENTITY
        };
        params.extend(live::coefficient_params(conf_gen::PREAMP_NODE, &preamp));

        for slot in 0..MAX_BANDS {
            let bq = match (chain.enabled, chain.bands.get(slot)) {
                (true, Some(band)) => biquad::coefficients(band, SAMPLE_RATE),
                _ => biquad::IDENTITY,
            };
            params.extend(live::coefficient_params(
                &conf_gen::band_node_name(slot),
                &bq,
            ));
        }

        self.backend.set_node_props(node_id, &params)?;
        Ok(())
    }

    fn apply_all(&self) -> Result<()> {
        for chain in EqChainId::ALL {
            self.apply_chain(chain)?;
        }
        Ok(())
    }

    fn push_node(&self, chain_id: EqChainId, node: &str, bq: &biquad::Biquad) -> Result<()> {
        let params = live::coefficient_params(node, bq);
        self.backend
            .set_node_props(self.node_id(chain_id)?, &params)?;
        Ok(())
    }

    fn after_mutation(&self) {
        self.dirty.store(true, Ordering::SeqCst);
        self.events.publish(Event::EqStateChanged(self.state()));
    }

    fn ensure_active(&self) -> Result<()> {
        if self.state.lock().unwrap().safe_mode {
            return Err(CoreError::Invalid("EQ is in safe mode".into()));
        }
        Ok(())
    }

    pub fn state(&self) -> EqState {
        self.state.lock().unwrap().clone()
    }

    pub fn is_active(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_chain_enabled(&self, chain_id: EqChainId, enabled: bool) -> Result<()> {
        self.ensure_active()?;
        {
            let mut state = self.state.lock().unwrap();
            state
                .chains
                .get_mut(&chain_id)
                .ok_or_else(|| CoreError::NodeNotFound(format!("{chain_id:?} chain")))?
                .enabled = enabled;
        }
        self.apply_chain(chain_id)?;
        self.after_mutation();
        Ok(())
    }

    /// Hot path: one band changed, so only its node is pushed.
    pub fn set_band(&self, chain_id: EqChainId, index: usize, band: EqBand) -> Result<()> {
        self.ensure_active()?;
        let band = band.clamped();
        let enabled =
            {
                let mut state = self.state.lock().unwrap();
                let chain = state
                    .chains
                    .get_mut(&chain_id)
                    .ok_or_else(|| CoreError::NodeNotFound(format!("{chain_id:?} chain")))?;
                *chain.bands.get_mut(index).ok_or_else(|| {
                    CoreError::Invalid(format!("band index {index} out of range"))
                })? = band;
                chain.enabled
            };

        if enabled {
            let bq = biquad::coefficients(&band, SAMPLE_RATE);
            self.push_node(chain_id, &conf_gen::band_node_name(index), &bq)?;
        }
        self.after_mutation();
        Ok(())
    }

    pub fn add_band(&self, chain_id: EqChainId, band: EqBand) -> Result<usize> {
        self.ensure_active()?;
        let band = band.clamped();
        let (index, enabled) = {
            let mut state = self.state.lock().unwrap();
            let chain = state
                .chains
                .get_mut(&chain_id)
                .ok_or_else(|| CoreError::NodeNotFound(format!("{chain_id:?} chain")))?;
            if chain.bands.len() >= MAX_BANDS {
                return Err(CoreError::Invalid(format!(
                    "chain already holds {MAX_BANDS} bands"
                )));
            }
            chain.bands.push(band);
            (chain.bands.len() - 1, chain.enabled)
        };

        if enabled {
            let bq = biquad::coefficients(&band, SAMPLE_RATE);
            self.push_node(chain_id, &conf_gen::band_node_name(index), &bq)?;
        }
        self.after_mutation();
        Ok(index)
    }

    /// Removal shifts every later index, so the whole chain is re-pushed.
    pub fn remove_band(&self, chain_id: EqChainId, index: usize) -> Result<()> {
        self.ensure_active()?;
        {
            let mut state = self.state.lock().unwrap();
            let chain = state
                .chains
                .get_mut(&chain_id)
                .ok_or_else(|| CoreError::NodeNotFound(format!("{chain_id:?} chain")))?;
            if index >= chain.bands.len() {
                return Err(CoreError::Invalid(format!(
                    "band index {index} out of range"
                )));
            }
            chain.bands.remove(index);
        }
        self.apply_chain(chain_id)?;
        self.after_mutation();
        Ok(())
    }

    pub fn set_preamp(&self, chain_id: EqChainId, gain_db: f32) -> Result<()> {
        self.ensure_active()?;
        let gain_db = gain_db.clamp(GAIN_MIN, GAIN_MAX);
        let enabled = {
            let mut state = self.state.lock().unwrap();
            let chain = state
                .chains
                .get_mut(&chain_id)
                .ok_or_else(|| CoreError::NodeNotFound(format!("{chain_id:?} chain")))?;
            chain.preamp_db = gain_db;
            chain.enabled
        };

        if enabled {
            self.push_node(chain_id, conf_gen::PREAMP_NODE, &biquad::gain(gain_db))?;
        }
        self.after_mutation();
        Ok(())
    }

    pub fn list_presets(&self) -> Vec<EqPresetMeta> {
        presets::list(&self.store)
    }

    pub fn apply_preset(&self, chain_id: EqChainId, name: &str) -> Result<()> {
        self.ensure_active()?;
        let preset = presets::get(&self.store, name)?;
        self.state
            .lock()
            .unwrap()
            .chains
            .insert(chain_id, preset.chain.clamped());
        self.apply_chain(chain_id)?;
        self.after_mutation();
        Ok(())
    }

    pub fn save_preset(&self, chain_id: EqChainId, name: &str) -> Result<()> {
        let chain = self.chain(chain_id)?;
        presets::save(&self.store, name, chain)
    }

    pub fn delete_preset(&self, name: &str) -> Result<()> {
        presets::delete(&self.store, name)
    }

    /// Follow a sink's output device change while the EQ is inserted.
    pub fn route_output(&self, chain_id: EqChainId, device: &str) -> Result<()> {
        self.ensure_active()?;
        wiring::route_eq_output(&*self.backend, chain_id, device)
    }
}
