//! Desired output routing, and putting it back when it disappears.
//!
//! A wireless headset powering on arrives as a brand new node with no links.
//! Nothing in PipeWire remembers that a sink used to feed it, so without this
//! the sink is left dangling and the user has to pick the device again.

use crate::eq::{nodes, wiring};
use crate::error::Result;
use penguinwave_pipewire::PipeWireBackend;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Sink name -> output device node name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routes(pub BTreeMap<String, String>);

impl Routes {
    pub fn get(&self, sink: &str) -> Option<&String> {
        self.0.get(sink)
    }

    pub fn set(&mut self, sink: &str, device: &str) {
        self.0.insert(sink.to_string(), device.to_string());
    }

    pub fn remove(&mut self, sink: &str) {
        self.0.remove(sink);
    }
}

/// Where a sink's audio currently ends up, through the EQ or directly.
pub fn current_target(backend: &dyn PipeWireBackend, sink: &str) -> Result<Option<String>> {
    match nodes::chain_for_sink(sink) {
        Some(chain) => wiring::current_device_target(backend, chain),
        None => wiring::direct_target(backend, sink),
    }
}

/// Re-link any sink whose desired device is present but unconnected.
///
/// Only a sink with no target at all is touched. One pointing somewhere else
/// was deliberately changed -- by this app or by `pavucontrol` -- and forcing
/// it back would fight the user.
pub fn reconcile(
    backend: &dyn PipeWireBackend,
    routes: &Routes,
    eq_active: bool,
) -> Result<Vec<String>> {
    let present: Vec<String> = backend.list_sinks()?.into_iter().map(|s| s.name).collect();
    let mut restored = Vec::new();

    for (sink, device) in &routes.0 {
        if !present.iter().any(|n| n == sink) || !present.iter().any(|n| n == device) {
            continue;
        }
        if current_target(backend, sink)?.is_some() {
            continue;
        }

        match nodes::chain_for_sink(sink) {
            Some(chain) if eq_active => wiring::wire_through_eq(backend, chain, device)?,
            _ => backend.route_sink_to_device(sink, device)?,
        }
        restored.push(sink.clone());
    }
    Ok(restored)
}
