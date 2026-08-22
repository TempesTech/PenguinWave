//! Inserting the EQ chains between the virtual sinks and the output device,
//! and restoring the direct link for shutdown and safe mode.

use crate::eq::nodes::{is_managed_node, output_node_name, sink_node_name, source_sink_for};
use crate::error::{CoreError, Result};
use penguinwave_pipewire::PipeWireBackend;
use penguinwave_proto::{EqChainId, PortRef};

const CHANNELS: [&str; 2] = ["FL", "FR"];

fn port(node: &str, name: String) -> PortRef {
    PortRef {
        node_name: node.to_string(),
        port_name: name,
        id: None,
    }
}

/// Every port currently linked from `source`.
///
/// Matched by port id where the backend reports one: node names are not
/// unique, so matching on the name alone can tear down another node's link.
fn targets_of(backend: &dyn PipeWireBackend, source: &PortRef) -> Result<Vec<PortRef>> {
    Ok(backend
        .list_links()?
        .into_iter()
        .filter(|l| same_port(&l.source, source))
        .map(|l| l.target)
        .collect())
}

fn same_port(a: &PortRef, b: &PortRef) -> bool {
    match (a.id, b.id) {
        (Some(x), Some(y)) => x == y,
        _ => a.node_name == b.node_name && a.port_name == b.port_name,
    }
}

fn relink(backend: &dyn PipeWireBackend, source: &PortRef, target: &PortRef) -> Result<()> {
    for existing in targets_of(backend, source)? {
        if !same_port(&existing, target) {
            let _ = backend.unlink_ports(source, &existing);
        }
    }
    backend.link_ports(source, target)?;
    Ok(())
}

fn reject_managed(device: &str) -> Result<()> {
    if is_managed_node(device) {
        return Err(CoreError::Invalid(format!(
            "refusing to route EQ into managed node {device:?}: feedback loop"
        )));
    }
    Ok(())
}

/// Pick a real hardware output: the default sink when it is a physical device,
/// otherwise the first ALSA or Bluetooth sink.
pub fn fallback_output_device(backend: &dyn PipeWireBackend) -> Option<String> {
    if let Ok(default) = backend.default_sink() {
        if !is_managed_node(&default) && !default.starts_with("auto_null") {
            return Some(default);
        }
    }
    backend
        .list_sinks()
        .ok()?
        .into_iter()
        .map(|s| s.name)
        .find(|n| n.starts_with("alsa_output") || n.starts_with("bluez_output"))
}

/// Device the chain currently feeds, whether through the EQ or directly.
pub fn current_device_target(
    backend: &dyn PipeWireBackend,
    chain: EqChainId,
) -> Result<Option<String>> {
    let eq_out = port(output_node_name(chain), "output_FL".into());
    if let Some(target) = targets_of(backend, &eq_out)?.first() {
        return Ok(Some(target.node_name.clone()));
    }

    let monitor = port(source_sink_for(chain), "monitor_FL".into());
    Ok(targets_of(backend, &monitor)?
        .into_iter()
        .map(|t| t.node_name)
        .find(|node| !is_managed_node(node)))
}

/// Insert the EQ chain into the graph.
///
/// The capture stream targets the monitor itself (`stream.capture.sink` in the
/// conf), so the input side only needs the direct monitor-to-device links
/// dropped; leaving them doubles the audio, dry plus wet.
pub fn wire_through_eq(
    backend: &dyn PipeWireBackend,
    chain: EqChainId,
    device: &str,
) -> Result<()> {
    reject_managed(device)?;
    let source = source_sink_for(chain);
    let eq_input = sink_node_name(chain);

    for ch in CHANNELS {
        let monitor = port(source, format!("monitor_{ch}"));
        let input = port(eq_input, format!("input_{ch}"));

        for target in targets_of(backend, &monitor)? {
            if target.node_name != eq_input {
                let _ = backend.unlink_ports(&monitor, &target);
            }
        }
        if !targets_of(backend, &monitor)?
            .iter()
            .any(|t| same_port(t, &input))
        {
            backend.link_ports(&monitor, &input)?;
        }

        relink(
            backend,
            &port(output_node_name(chain), format!("output_{ch}")),
            &port(device, format!("playback_{ch}")),
        )?;
    }
    Ok(())
}

/// Point the chain's output at another device, leaving the input side alone.
pub fn route_eq_output(
    backend: &dyn PipeWireBackend,
    chain: EqChainId,
    device: &str,
) -> Result<()> {
    reject_managed(device)?;
    for ch in CHANNELS {
        relink(
            backend,
            &port(output_node_name(chain), format!("output_{ch}")),
            &port(device, format!("playback_{ch}")),
        )?;
    }
    Ok(())
}

/// Bypass the EQ at the graph level: monitor straight to the device.
///
/// Used for shutdown and safe mode, so audio keeps flowing un-EQ'd. No managed
/// guard: callers resolve the target through the two functions above, which
/// already filter.
pub fn wire_direct(backend: &dyn PipeWireBackend, chain: EqChainId, device: &str) -> Result<()> {
    let source = source_sink_for(chain);
    for ch in CHANNELS {
        relink(
            backend,
            &port(source, format!("monitor_{ch}")),
            &port(device, format!("playback_{ch}")),
        )?;
    }
    Ok(())
}
