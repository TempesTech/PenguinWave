//! Sinks, streams, ports and links.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The two virtual sinks Penguin Wave manages by default.
pub const GAME_SINK: &str = "game_sink";
pub const CHAT_SINK: &str = "chat_sink";

/// A virtual sink the daemon creates and owns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SinkConfig {
    pub name: String,
    pub display_name: String,
}

impl SinkConfig {
    /// The sinks created on first run.
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

/// A sink as it currently exists in the audio server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SinkInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// 0..=100. Not `f32`: the UI works in whole percent and the audio server
    /// is told whole percent, so carrying more precision only invites
    /// round-trip drift between client and daemon.
    pub volume: u8,
    pub is_muted: bool,
    /// True when Penguin Wave created it, as opposed to a sink that was
    /// already there. Only managed sinks are adopted or reconciled on startup.
    pub managed: bool,
}

/// Identifies a playback stream.
///
/// Deliberately not a bare index. PulseAudio recycles sink-input indices the
/// moment a stream dies, so a client holding a stale index can silently
/// retarget the wrong application. `index` is the fast path; the rest is
/// revalidated before any mutation, and a mismatch is reported as
/// [`crate::error::ErrorKind::Conflict`] rather than acted on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreamRef {
    pub index: u32,
    /// `application.name` as reported when the client last saw the stream.
    pub app_name: String,
    /// Owning process id, when the audio server reports one.
    pub pid: Option<u32>,
}

/// A playback stream belonging to some application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreamInfo {
    #[serde(flatten)]
    pub stream: StreamRef,
    /// Resolved display name: `application.name` -> `application.process.binary`
    /// -> `/proc/<pid>/cmdline` -> a `media.role` hybrid label.
    pub name: String,
    /// Sink this stream is currently routed to, by name.
    pub sink: String,
    /// 0..=100.
    pub volume: u8,
    pub is_muted: bool,
    /// Opaque key for [`crate::request::Request::StreamIcon`].
    ///
    /// Resolving an icon walks the filesystem for `.desktop` entries, so it is
    /// never done inline in a stream listing. `None` means no icon was found
    /// and the client should not ask again.
    pub icon_key: Option<String>,
}

/// A hardware output the user can route a sink to.
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

/// A port on a node, as addressed by `pw-link`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortInfo {
    pub id: u32,
    pub node_name: String,
    pub port_name: String,
    pub direction: PortDirection,
}

/// One end of a link, in the `node:port` form `pw-link` accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortRef {
    pub node_name: String,
    pub port_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkInfo {
    pub source: PortRef,
    pub target: PortRef,
}

/// The card route a sink currently feeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RouteInfo {
    pub sink: String,
    pub device: String,
    pub description: String,
}
