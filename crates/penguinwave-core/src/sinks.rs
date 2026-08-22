//! Adopt-or-create reconciliation of the managed virtual sinks.

use crate::error::Result;
use penguinwave_pipewire::PipeWireBackend;
use penguinwave_proto::{SinkConfig, SinkInfo};

/// Bring the managed sinks up to the desired set without disturbing what is
/// already there.
///
/// One code path serves cold start, daemon restart and PipeWire recovery: an
/// existing sink is adopted, never deleted and recreated, because deleting one
/// drops every stream routed to it.
pub fn reconcile(backend: &dyn PipeWireBackend) -> Result<Vec<SinkInfo>> {
    let existing = backend.list_sinks()?;

    for cfg in SinkConfig::defaults() {
        if !existing.iter().any(|s| s.name == cfg.name) {
            backend.create_virtual_sink(&cfg)?;
        }
    }

    Ok(backend.list_sinks()?)
}

/// Whether both managed sinks are present.
pub fn sinks_ready(sinks: &[SinkInfo]) -> bool {
    SinkConfig::defaults()
        .iter()
        .all(|cfg| sinks.iter().any(|s| s.name == cfg.name))
}
