//! In-memory backend for tests.
//!
//! Seeded from the captured fixtures, so callers are tested against the shape
//! of real output rather than invented data. Mutations are applied to the
//! in-memory state, which lets a test assert on the resulting graph.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use penguinwave_proto::{
    ErrorKind, LinkInfo, OutputDevice, PortDirection, PortInfo, PortRef, PwError, RouteInfo,
    SinkConfig, SinkInfo, StreamInfo, StreamRef,
};

use crate::backend::{PipeWireBackend, Result, ServerInfo};
use crate::chain::ChainProcess;
use crate::parse;
use crate::watch::{EventSink, WatchHandle};

const SINKS: &str = include_str!("../tests/fixtures/sinks_full.txt");
const SINK_INPUTS: &str = include_str!("../tests/fixtures/sink_inputs_full.txt");
const SINKS_SHORT: &str = include_str!("../tests/fixtures/sinks_short.txt");
const LINKS: &str = include_str!("../tests/fixtures/pw_link_ids.txt");
const DUMP: &str = include_str!("../tests/fixtures/pw_dump_all.json");
const DEFAULT_SINK: &str = include_str!("../tests/fixtures/default_sink.txt");

#[derive(Debug, Clone, Default)]
struct State {
    sinks: Vec<SinkInfo>,
    streams: Vec<StreamInfo>,
    links: Vec<LinkInfo>,
    ports: Vec<PortInfo>,
    default_sink: String,
    /// Set to make every call fail, for testing recovery paths.
    unavailable: bool,
    /// Exit codes handed to successive spawned chains; `None` keeps running.
    chain_exits: Vec<Option<i32>>,
}

#[derive(Clone)]
pub struct MockBackend {
    state: Arc<Mutex<State>>,
    /// Registered by `watch`, so a test can fire a graph change.
    watcher: Arc<Mutex<Option<EventSink>>>,
    /// Calls made, in order, for asserting that a caller did what it claimed.
    pub calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBackend {
    /// Seeded from the fixtures.
    pub fn new() -> Self {
        let sinks_short = parse::parse_sinks_short(SINKS_SHORT);
        let streams = parse::parse_sink_inputs(SINK_INPUTS)
            .into_iter()
            .map(|s| {
                let sink = sinks_short
                    .iter()
                    .find(|(id, _)| *id == s.sink_id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_default();
                StreamInfo {
                    name: if s.stream.app_name.is_empty() {
                        s.binary.clone().unwrap_or_else(|| "Unknown".into())
                    } else {
                        s.stream.app_name.clone()
                    },
                    sink,
                    volume: s.volume,
                    is_muted: s.is_muted,
                    icon_key: s.icon_name.clone(),
                    stream: s.stream,
                }
            })
            .collect();

        Self {
            state: Arc::new(Mutex::new(State {
                sinks: parse::parse_sinks(SINKS),
                streams,
                links: parse::parse_links(LINKS),
                ports: parse::parse_ports_from_dump(DUMP).unwrap_or_default(),
                default_sink: DEFAULT_SINK.trim().to_string(),
                unavailable: false,
                chain_exits: Vec::new(),
            })),
            calls: Arc::new(Mutex::new(Vec::new())),
            watcher: Arc::new(Mutex::new(None)),
        }
    }

    /// Empty session: no sinks, streams or links.
    pub fn empty() -> Self {
        let mock = Self::new();
        {
            let mut s = mock.state.lock().unwrap();
            *s = State {
                default_sink: String::new(),
                ..Default::default()
            };
        }
        mock.calls.lock().unwrap().clear();
        mock
    }

    /// Make every call fail with `PipeWireUnavailable`.
    pub fn set_unavailable(&self, value: bool) {
        self.state.lock().unwrap().unavailable = value;
    }

    /// Exit codes for successive `spawn_filter_chain` calls; `None` runs forever.
    pub fn queue_chain_exits(&self, exits: Vec<Option<i32>>) {
        self.state.lock().unwrap().chain_exits = exits;
    }

    /// Make `resolve_node_id` answer for a node that is not in the fixtures.
    pub fn add_node(&self, name: &str, id: u32) {
        self.state.lock().unwrap().ports.push(PortInfo {
            id,
            node_name: name.to_string(),
            port_name: "input_FL".into(),
            direction: PortDirection::Input,
        });
    }

    /// Fire the registered graph watch, as the audio server would.
    pub fn fire_graph_change(&self) {
        let sink = self.watcher.lock().unwrap().clone();
        if let Some(sink) = sink {
            sink();
        }
    }

    pub fn is_watching(&self) -> bool {
        self.watcher.lock().unwrap().is_some()
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, call: impl Into<String>) -> Result<()> {
        self.calls.lock().unwrap().push(call.into());
        if self.state.lock().unwrap().unavailable {
            return Err(PwError::new(
                ErrorKind::PipeWireUnavailable,
                "mock: server unavailable",
            ));
        }
        Ok(())
    }

    /// Mirrors `PactlBackend`: an index is valid only while it still belongs to
    /// the same application.
    fn revalidate(&self, stream: &StreamRef) -> Result<usize> {
        let state = self.state.lock().unwrap();
        let position = state
            .streams
            .iter()
            .position(|s| s.stream.index == stream.index)
            .ok_or_else(|| {
                PwError::new(
                    ErrorKind::Conflict,
                    format!("stream {} no longer exists", stream.index),
                )
            })?;

        if state.streams[position].stream.app_name != stream.app_name {
            return Err(PwError::new(
                ErrorKind::Conflict,
                format!("stream {} now belongs to another application", stream.index),
            ));
        }

        Ok(position)
    }
}

impl PipeWireBackend for MockBackend {
    fn list_application_streams(&self) -> Result<Vec<StreamInfo>> {
        self.record("list_application_streams")?;
        Ok(self.state.lock().unwrap().streams.clone())
    }

    fn list_sinks(&self) -> Result<Vec<SinkInfo>> {
        self.record("list_sinks")?;
        Ok(self.state.lock().unwrap().sinks.clone())
    }

    fn list_output_devices(&self) -> Result<Vec<OutputDevice>> {
        self.record("list_output_devices")?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .sinks
            .iter()
            .filter(|s| !s.managed)
            .map(|s| OutputDevice {
                id: s.id,
                name: s.name.clone(),
                description: s.description.clone(),
            })
            .collect())
    }

    fn list_node_ports(&self, node: &str) -> Result<Vec<PortInfo>> {
        self.record(format!("list_node_ports {node}"))?;
        let ports: Vec<_> = self
            .state
            .lock()
            .unwrap()
            .ports
            .iter()
            .filter(|p| p.node_name == node)
            .cloned()
            .collect();

        if ports.is_empty() {
            return Err(PwError::not_found(format!("ports for node '{node}'")));
        }
        Ok(ports)
    }

    fn default_sink(&self) -> Result<String> {
        self.record("default_sink")?;
        Ok(self.state.lock().unwrap().default_sink.clone())
    }

    fn create_virtual_sink(&self, cfg: &SinkConfig) -> Result<()> {
        self.record(format!("create_virtual_sink {}", cfg.name))?;
        let mut state = self.state.lock().unwrap();

        if state.sinks.iter().any(|s| s.name == cfg.name) {
            return Err(PwError::new(
                ErrorKind::Conflict,
                format!("sink '{}' already exists", cfg.name),
            ));
        }

        let id = state.sinks.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        state.sinks.push(SinkInfo {
            id,
            name: cfg.name.clone(),
            description: cfg.display_name.clone(),
            volume: 100,
            is_muted: false,
            managed: true,
        });
        Ok(())
    }

    fn delete_virtual_sink(&self, name: &str) -> Result<()> {
        self.record(format!("delete_virtual_sink {name}"))?;
        let mut state = self.state.lock().unwrap();

        let sink = state
            .sinks
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| PwError::not_found(format!("sink '{name}'")))?;

        if !sink.managed {
            return Err(PwError::bad_request(format!(
                "sink '{name}' was not created by Penguin Wave"
            )));
        }

        state.sinks.retain(|s| s.name != name);
        Ok(())
    }

    fn move_stream_to_sink(&self, stream: &StreamRef, sink: &str) -> Result<()> {
        self.record(format!("move_stream_to_sink {} {sink}", stream.index))?;
        let position = self.revalidate(stream)?;
        let mut state = self.state.lock().unwrap();

        if !state.sinks.iter().any(|s| s.name == sink) {
            return Err(PwError::not_found(format!("sink '{sink}'")));
        }
        state.streams[position].sink = sink.to_string();
        Ok(())
    }

    fn set_sink_volume(&self, sink: &str, pct: u8) -> Result<()> {
        self.record(format!("set_sink_volume {sink} {pct}"))?;
        let mut state = self.state.lock().unwrap();
        let entry = state
            .sinks
            .iter_mut()
            .find(|s| s.name == sink)
            .ok_or_else(|| PwError::not_found(format!("sink '{sink}'")))?;
        entry.volume = pct.min(100);
        Ok(())
    }

    fn set_stream_volume(&self, stream: &StreamRef, pct: u8) -> Result<()> {
        self.record(format!("set_stream_volume {} {pct}", stream.index))?;
        let position = self.revalidate(stream)?;
        self.state.lock().unwrap().streams[position].volume = pct.min(100);
        Ok(())
    }

    fn set_stream_mute(&self, stream: &StreamRef, mute: bool) -> Result<()> {
        self.record(format!("set_stream_mute {} {mute}", stream.index))?;
        let position = self.revalidate(stream)?;
        self.state.lock().unwrap().streams[position].is_muted = mute;
        Ok(())
    }

    fn link_ports(&self, source: &PortRef, target: &PortRef) -> Result<()> {
        self.record(format!(
            "link_ports {}:{} -> {}:{}",
            source.node_name, source.port_name, target.node_name, target.port_name
        ))?;
        self.state.lock().unwrap().links.push(LinkInfo {
            source: source.clone(),
            target: target.clone(),
        });
        Ok(())
    }

    fn unlink_ports(&self, source: &PortRef, target: &PortRef) -> Result<()> {
        self.record(format!(
            "unlink_ports {}:{} -> {}:{}",
            source.node_name, source.port_name, target.node_name, target.port_name
        ))?;
        let mut state = self.state.lock().unwrap();
        let before = state.links.len();
        state
            .links
            .retain(|l| !(&l.source == source && &l.target == target));

        if state.links.len() == before {
            return Err(PwError::not_found("link"));
        }
        Ok(())
    }

    fn list_links(&self) -> Result<Vec<LinkInfo>> {
        self.record("list_links")?;
        Ok(self.state.lock().unwrap().links.clone())
    }

    fn sink_current_route(&self, sink: &str) -> Result<Option<RouteInfo>> {
        self.record(format!("sink_current_route {sink}"))?;
        let state = self.state.lock().unwrap();

        Ok(state
            .links
            .iter()
            .find(|l| l.source.node_name == sink && l.source.port_name.starts_with("monitor_"))
            .map(|l| RouteInfo {
                sink: sink.to_string(),
                device: l.target.node_name.clone(),
                description: l.target.node_name.clone(),
            }))
    }

    fn route_sink_to_device(&self, sink: &str, device: &str) -> Result<()> {
        self.record(format!("route_sink_to_device {sink} {device}"))?;
        let mut state = self.state.lock().unwrap();

        state.links.retain(|l| {
            !(l.source.node_name == sink && l.source.port_name.starts_with("monitor_"))
        });

        for channel in ["FL", "FR"] {
            state.links.push(LinkInfo {
                source: PortRef {
                    node_name: sink.to_string(),
                    port_name: format!("monitor_{channel}"),
                    id: None,
                },
                target: PortRef {
                    node_name: device.to_string(),
                    port_name: format!("playback_{channel}"),
                    id: None,
                },
            });
        }
        Ok(())
    }

    fn resolve_node_id(&self, node_name: &str) -> Result<u32> {
        self.record(format!("resolve_node_id {node_name}"))?;
        self.state
            .lock()
            .unwrap()
            .ports
            .iter()
            .find(|p| p.node_name == node_name)
            .map(|p| p.id)
            .ok_or_else(|| PwError::not_found(format!("node '{node_name}'")))
    }

    fn set_node_props(&self, node_id: u32, props: &[(String, f32)]) -> Result<()> {
        self.record(format!("set_node_props {node_id} ({} props)", props.len()))
    }

    fn spawn_filter_chain(&self, _conf_path: &std::path::Path) -> Result<Box<dyn ChainProcess>> {
        self.record("spawn_filter_chain")?;
        let exit = {
            let mut state = self.state.lock().unwrap();
            if state.chain_exits.is_empty() {
                None
            } else {
                state.chain_exits.remove(0)
            }
        };
        Ok(Box::new(MockChain { exit }))
    }

    fn probe(&self) -> Result<ServerInfo> {
        self.record("probe")?;
        let state = self.state.lock().unwrap();
        Ok(ServerInfo {
            server_name: "mock".into(),
            default_sink: state.default_sink.clone(),
        })
    }

    fn watch(&self, sink: EventSink) -> Result<WatchHandle> {
        self.record("watch")?;
        *self.watcher.lock().unwrap() = Some(sink);
        Ok(WatchHandle::new(Arc::new(AtomicBool::new(false))))
    }
}

/// A chain that either runs forever or is already dead, per `queue_chain_exit`.
struct MockChain {
    exit: Option<i32>,
}

impl ChainProcess for MockChain {
    fn try_wait(&mut self) -> Result<Option<i32>> {
        Ok(self.exit)
    }

    fn kill(&mut self) {
        self.exit = Some(0);
    }
}
