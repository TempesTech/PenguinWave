//! `PipeWireBackend` over `pactl`, `pw-link` and `pw-dump`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use penguinwave_proto::{
    ErrorKind, LinkInfo, OutputDevice, PortInfo, PortRef, PwError, RouteInfo, SinkConfig, SinkInfo,
    StreamInfo, StreamRef,
};

use crate::backend::{PipeWireBackend, Result, ServerInfo};
use crate::chain::ChainProcess;
use crate::cmd::{run, run_ok};
use crate::icons;
use crate::parse;
use crate::watch::{EventSink, WatchHandle};

/// Interval between graph polls.
///
/// Polling is how this backend implements `watch`; a native registry
/// implementation would push instead.
const WATCH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Default, Clone)]
pub struct PactlBackend;

impl PactlBackend {
    pub fn new() -> Self {
        Self
    }

    /// Resolve a stream reference to a live sink-input index.
    ///
    /// The index alone is not trustworthy: PulseAudio reuses sink-input indices
    /// as soon as a stream ends, so a client holding a stale one can retarget a
    /// different application. The index is accepted only if it still belongs to
    /// the same application.
    ///
    /// One process can own several streams, so this cannot distinguish them
    /// from each other; it only rules out cross-application mistakes.
    fn revalidate(&self, stream: &StreamRef) -> Result<u32> {
        let raw = parse::parse_sink_inputs(&run("pactl", &["list", "sink-inputs"])?);

        let Some(found) = raw.iter().find(|s| s.stream.index == stream.index) else {
            return Err(PwError::new(
                ErrorKind::Conflict,
                format!("stream {} no longer exists", stream.index),
            ));
        };

        if found.stream.app_name != stream.app_name {
            return Err(PwError::new(
                ErrorKind::Conflict,
                format!(
                    "stream {} now belongs to '{}', not '{}'",
                    stream.index, found.stream.app_name, stream.app_name
                ),
            ));
        }

        Ok(stream.index)
    }

    fn sink_names(&self) -> Result<Vec<(u32, String)>> {
        Ok(parse::parse_sinks_short(&run(
            "pactl",
            &["list", "sinks", "short"],
        )?))
    }

    fn port_spec(port: &PortRef) -> String {
        format!("{}:{}", port.node_name, port.port_name)
    }

    /// Address a port by id where one is known.
    ///
    /// Node names are not unique — several nodes of one application share a
    /// name — so linking by name can connect the wrong port.
    fn port_arg(port: &PortRef) -> String {
        match port.id {
            Some(id) => id.to_string(),
            None => Self::port_spec(port),
        }
    }
}

impl PipeWireBackend for PactlBackend {
    fn list_application_streams(&self) -> Result<Vec<StreamInfo>> {
        let raw = parse::parse_sink_inputs(&run("pactl", &["list", "sink-inputs"])?);
        let sinks = self.sink_names()?;

        Ok(raw
            .into_iter()
            // The EQ chain's own playback streams are sink-inputs like any
            // other; listing them invites the user to move Penguin Wave's
            // plumbing around inside Penguin Wave.
            .filter(|s| {
                !s.node_name
                    .as_deref()
                    .is_some_and(penguinwave_proto::is_own_node)
            })
            .map(|s| {
                let sink = sinks
                    .iter()
                    .find(|(id, _)| *id == s.sink_id)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_default();

                StreamInfo {
                    name: parse::display_name(&s),
                    sink,
                    volume: s.volume,
                    is_muted: s.is_muted,
                    icon_key: icons::icon_key(&s),
                    stream: s.stream,
                }
            })
            .collect())
    }

    fn list_sinks(&self) -> Result<Vec<SinkInfo>> {
        Ok(parse::parse_sinks(&run("pactl", &["list", "sinks"])?))
    }

    fn list_output_devices(&self) -> Result<Vec<OutputDevice>> {
        Ok(parse::parse_output_devices(&run(
            "pactl",
            &["list", "sinks"],
        )?))
    }

    fn list_node_ports(&self, node: &str) -> Result<Vec<PortInfo>> {
        let dump = run("pw-dump", &[])?;
        let ports = parse::parse_ports_from_dump(&dump)
            .map_err(|e| PwError::new(ErrorKind::PipeWireFailed, format!("pw-dump: {e}")))?;

        let matching: Vec<_> = ports.into_iter().filter(|p| p.node_name == node).collect();
        if matching.is_empty() {
            return Err(PwError::not_found(format!("ports for node '{node}'")));
        }
        Ok(matching)
    }

    fn default_sink(&self) -> Result<String> {
        Ok(run("pactl", &["get-default-sink"])?.trim().to_string())
    }

    fn create_virtual_sink(&self, cfg: &SinkConfig) -> Result<()> {
        run_ok(
            "pactl",
            &[
                "load-module",
                "module-null-sink",
                &format!("sink_name={}", cfg.name),
                &format!(
                    "sink_properties=device.description=\"{}\"",
                    cfg.display_name
                ),
            ],
        )
    }

    fn delete_virtual_sink(&self, name: &str) -> Result<()> {
        let sinks = parse::parse_sinks(&run("pactl", &["list", "sinks"])?);
        let sink = sinks
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| PwError::not_found(format!("sink '{name}'")))?;

        if !sink.managed {
            return Err(PwError::bad_request(format!(
                "sink '{name}' was not created by Penguin Wave"
            )));
        }

        run_ok("pactl", &["unload-module", &sink.id.to_string()])
    }

    fn move_stream_to_sink(&self, stream: &StreamRef, sink: &str) -> Result<()> {
        let index = self.revalidate(stream)?;
        run_ok("pactl", &["move-sink-input", &index.to_string(), sink])
    }

    fn set_sink_volume(&self, sink: &str, pct: u8) -> Result<()> {
        run_ok(
            "pactl",
            &["set-sink-volume", sink, &format!("{}%", pct.min(100))],
        )
    }

    fn set_stream_volume(&self, stream: &StreamRef, pct: u8) -> Result<()> {
        let index = self.revalidate(stream)?;
        run_ok(
            "pactl",
            &[
                "set-sink-input-volume",
                &index.to_string(),
                &format!("{}%", pct.min(100)),
            ],
        )
    }

    fn set_stream_mute(&self, stream: &StreamRef, mute: bool) -> Result<()> {
        let index = self.revalidate(stream)?;
        run_ok(
            "pactl",
            &[
                "set-sink-input-mute",
                &index.to_string(),
                if mute { "1" } else { "0" },
            ],
        )
    }

    /// Linking is idempotent.
    ///
    /// `pw-link` fails with `File exists` when the link is already there. The
    /// caller asked for the link to exist, and it does, so that is success:
    /// treating it as an error makes every re-wire of an already-correct graph
    /// look like a failure.
    fn link_ports(&self, source: &PortRef, target: &PortRef) -> Result<()> {
        match run_ok(
            "pw-link",
            &[&Self::port_arg(source), &Self::port_arg(target)],
        ) {
            Err(e) if e.msg.contains("File exists") => Ok(()),
            other => other,
        }
    }

    /// Unlinking is idempotent, for the same reason as [`Self::link_ports`].
    fn unlink_ports(&self, source: &PortRef, target: &PortRef) -> Result<()> {
        match run_ok(
            "pw-link",
            &["-d", &Self::port_arg(source), &Self::port_arg(target)],
        ) {
            Err(e) if e.msg.contains("No such") || e.msg.contains("not found") => Ok(()),
            other => other,
        }
    }

    fn list_links(&self) -> Result<Vec<LinkInfo>> {
        Ok(parse::parse_links(&run("pw-link", &["-I", "-l"])?))
    }

    fn sink_current_route(&self, sink: &str) -> Result<Option<RouteInfo>> {
        let links = self.list_links()?;
        let devices = self.list_output_devices()?;

        Ok(parse::trace_route(sink, &links).map(|device| {
            let description = devices
                .iter()
                .find(|d| d.name == device)
                .map(|d| d.description.clone())
                .unwrap_or_else(|| device.clone());

            RouteInfo {
                sink: sink.to_string(),
                device,
                description,
            }
        }))
    }

    fn route_sink_to_device(&self, sink: &str, device: &str) -> Result<()> {
        let links = self.list_links()?;

        for channel in ["FL", "FR"] {
            let monitor = format!("monitor_{channel}");

            // Unlink by port id: several nodes can share a name, so matching on
            // the name alone risks unlinking a different application's port.
            for link in links
                .iter()
                .filter(|l| l.source.node_name == sink && l.source.port_name == monitor)
            {
                self.unlink_ports(&link.source, &link.target)?;
            }

            let source = links
                .iter()
                .find(|l| l.source.node_name == sink && l.source.port_name == monitor)
                .map(|l| l.source.clone())
                .unwrap_or(PortRef {
                    node_name: sink.to_string(),
                    port_name: monitor,
                    id: None,
                });

            self.link_ports(
                &source,
                &PortRef {
                    node_name: device.to_string(),
                    port_name: format!("playback_{channel}"),
                    id: None,
                },
            )?;
        }

        Ok(())
    }

    fn resolve_node_id(&self, node_name: &str) -> Result<u32> {
        let dump = run("pw-dump", &[])?;
        parse::node_id_for(&dump, node_name)
            .ok_or_else(|| PwError::not_found(format!("node '{node_name}'")))
    }

    fn set_node_props(&self, node_id: u32, props: &[(String, f32)]) -> Result<()> {
        let params = props
            .iter()
            .map(|(k, v)| format!("\"{k}\" {v}"))
            .collect::<Vec<_>>()
            .join(" ");

        run_ok(
            "pw-cli",
            &[
                "set-param",
                &node_id.to_string(),
                "Props",
                &format!("{{ params = [ {params} ] }}"),
            ],
        )
    }

    fn spawn_filter_chain(&self, conf_path: &std::path::Path) -> Result<Box<dyn ChainProcess>> {
        crate::chain::spawn(conf_path)
    }

    fn probe(&self) -> Result<ServerInfo> {
        let info = run("pactl", &["info"])?;
        let server_name = info
            .lines()
            .find_map(|l| l.strip_prefix("Server Name: "))
            .unwrap_or("unknown")
            .trim()
            .to_string();

        Ok(ServerInfo {
            server_name,
            default_sink: self.default_sink()?,
        })
    }

    fn watch(&self, sink: EventSink) -> Result<WatchHandle> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let backend = self.clone();

        thread::spawn(move || {
            let mut previous = backend.graph_signature();
            while !thread_stop.load(Ordering::Relaxed) {
                thread::sleep(WATCH_INTERVAL);
                let current = backend.graph_signature();
                if current != previous {
                    previous = current;
                    sink();
                }
            }
        });

        Ok(WatchHandle::new(stop))
    }
}

impl PactlBackend {
    /// Cheap value that changes whenever the graph does.
    ///
    /// Errors collapse to `None`, so a momentary failure reads as a change and
    /// the caller resyncs rather than going stale.
    fn graph_signature(&self) -> Option<String> {
        let links = run("pw-link", &["-I", "-l"]).ok()?;
        let sinks = run("pactl", &["list", "sinks", "short"]).ok()?;
        let streams = run("pactl", &["list", "short", "sink-inputs"]).ok()?;
        Some(format!("{links}{sinks}{streams}"))
    }
}

