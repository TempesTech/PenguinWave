//! Sinks, streams, ports and links.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const GAME_SINK: &str = "game_sink";
pub const CHAT_SINK: &str = "chat_sink";

/// Every node the EQ filter chain registers.
pub const EQ_NODE_PREFIX: &str = "penguinwave_eq";

/// Whether a node belongs to Penguin Wave itself.
///
/// The EQ chain's playback streams are ordinary sink-inputs, so without this
/// they are listed as if they were applications the user could move.
pub fn is_own_node(node_name: &str) -> bool {
    node_name == GAME_SINK || node_name == CHAT_SINK || node_name.starts_with(EQ_NODE_PREFIX)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SinkConfig {
    pub name: String,
    pub display_name: String,
}

impl SinkConfig {
    pub fn defaults() -> Vec<SinkConfig> {
        vec![
            SinkConfig {
                name: GAME_SINK.into(),
                display_name: "Game".into(),
            },
            SinkConfig {
                name: CHAT_SINK.into(),
                display_name: "Chat".into(),
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SinkInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// 0..=100.
    pub volume: u8,
    pub is_muted: bool,
    /// True for sinks Penguin Wave created; only these are reconciled on startup.
    pub managed: bool,
}

/// Identifies a playback stream.
///
/// Not a bare index: PulseAudio recycles sink-input indices, so `app_name` and
/// `pid` are revalidated before any mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreamRef {
    pub index: u32,
    pub app_name: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreamInfo {
    #[serde(flatten)]
    pub stream: StreamRef,
    /// Resolved display name.
    pub name: String,
    pub sink: String,
    /// 0..=100.
    pub volume: u8,
    pub is_muted: bool,
    /// Key for `stream.icon`. Icons are fetched separately, never inlined here.
    pub icon_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OutputDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortInfo {
    pub id: u32,
    pub node_name: String,
    pub port_name: String,
    pub direction: PortDirection,
}

/// One end of a link, in `pw-link`'s `node:port` form.
///
/// Node names are not unique: several nodes of one application share a name,
/// so `node_name:port_name` can match more than one port. Prefer `id` when
/// present; the names are for display and for `pw-link`'s own CLI form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortRef {
    pub node_name: String,
    pub port_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkInfo {
    pub source: PortRef,
    pub target: PortRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RouteInfo {
    pub sink: String,
    pub device: String,
    pub description: String,
}
