//! The trait every caller talks to.

use penguinwave_proto::{
    LinkInfo, OutputDevice, PortInfo, PortRef, PwError, RouteInfo, SinkConfig, SinkInfo,
    StreamInfo, StreamRef,
};

use crate::domain::chain::ChainProcess;
use crate::transport::watch::{EventSink, WatchHandle};

pub type Result<T> = std::result::Result<T, PwError>;

/// Audio server info, used as a liveness probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub server_name: String,
    pub default_sink: String,
}

pub trait PipeWireBackend: Send + Sync {
    // discovery
    fn list_application_streams(&self) -> Result<Vec<StreamInfo>>;
    fn list_sinks(&self) -> Result<Vec<SinkInfo>>;
    fn list_output_devices(&self) -> Result<Vec<OutputDevice>>;
    fn list_node_ports(&self, node: &str) -> Result<Vec<PortInfo>>;
    fn default_sink(&self) -> Result<String>;

    // mutation
    fn create_virtual_sink(&self, cfg: &SinkConfig) -> Result<()>;
    fn delete_virtual_sink(&self, name: &str) -> Result<()>;
    fn move_stream_to_sink(&self, stream: &StreamRef, sink: &str) -> Result<()>;
    fn set_sink_volume(&self, sink: &str, pct: u8) -> Result<()>;
    fn set_stream_volume(&self, stream: &StreamRef, pct: u8) -> Result<()>;
    fn set_stream_mute(&self, stream: &StreamRef, mute: bool) -> Result<()>;

    // graph
    fn link_ports(&self, source: &PortRef, target: &PortRef) -> Result<()>;
    fn unlink_ports(&self, source: &PortRef, target: &PortRef) -> Result<()>;
    fn list_links(&self) -> Result<Vec<LinkInfo>>;

    // card routing
    fn sink_current_route(&self, sink: &str) -> Result<Option<RouteInfo>>;
    fn route_sink_to_device(&self, sink: &str, device: &str) -> Result<()>;

    // node params, used by the EQ live-coefficient path
    fn resolve_node_id(&self, node_name: &str) -> Result<u32>;
    fn set_node_props(&self, node_id: u32, props: &[(String, f32)]) -> Result<()>;

    /// Start the EQ filter chain from a generated config file.
    fn spawn_filter_chain(&self, conf_path: &std::path::Path) -> Result<Box<dyn ChainProcess>>;

    // liveness
    fn probe(&self) -> Result<ServerInfo>;

    /// Report graph changes until the handle is dropped.
    ///
    /// The `pactl` implementation polls; a native one would use the registry.
    /// Callers cannot tell which they got.
    fn watch(&self, sink: EventSink) -> Result<WatchHandle>;
}
