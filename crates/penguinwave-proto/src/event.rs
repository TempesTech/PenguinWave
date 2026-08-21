//! Daemon -> client pushes.
//!
//! Every mutating method emits an event, including back to the client that
//! caused it, so clients have one state-update path rather than two.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audio::{SinkInfo, StreamInfo};
use crate::device::{ChatMix, DeviceId};
use crate::eq::EqState;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
#[ts(export)]
pub enum Event {
    /// Wheel turned, or a manual value applied.
    #[serde(rename = "chatmix.changed")]
    #[ts(rename = "chatmix.changed")]
    ChatmixChanged(ChatMix),

    #[serde(rename = "eq.state_changed")]
    #[ts(rename = "eq.state_changed")]
    EqStateChanged(EqState),

    /// Safe mode entered or cleared.
    #[serde(rename = "eq.safe_mode")]
    #[ts(rename = "eq.safe_mode")]
    EqSafeMode { active: bool },

    /// Links, nodes or the default sink changed. Debounced.
    #[serde(rename = "graph.changed")]
    #[ts(rename = "graph.changed")]
    GraphChanged,

    /// Streams appeared, vanished, moved, or changed volume/mute.
    #[serde(rename = "stream.list_changed")]
    #[ts(rename = "stream.list_changed")]
    StreamListChanged { streams: Vec<StreamInfo> },

    /// A sink was created or deleted, possibly by another client.
    #[serde(rename = "sink.list_changed")]
    #[ts(rename = "sink.list_changed")]
    SinkListChanged { sinks: Vec<SinkInfo> },

    #[serde(rename = "device.attached")]
    #[ts(rename = "device.attached")]
    DeviceAttached { device: DeviceId },

    #[serde(rename = "device.detached")]
    #[ts(rename = "device.detached")]
    DeviceDetached { device: DeviceId },

    /// Planned stop. Managed sinks and the EQ chain are left running.
    #[serde(rename = "daemon.shutting_down")]
    #[ts(rename = "daemon.shutting_down")]
    DaemonShuttingDown,
}

impl Event {
    /// Wire name, for subscription matching.
    pub fn name(&self) -> &'static str {
        match self {
            Event::ChatmixChanged(_) => "chatmix.changed",
            Event::EqStateChanged(_) => "eq.state_changed",
            Event::EqSafeMode { .. } => "eq.safe_mode",
            Event::GraphChanged => "graph.changed",
            Event::StreamListChanged { .. } => "stream.list_changed",
            Event::SinkListChanged { .. } => "sink.list_changed",
            Event::DeviceAttached { .. } => "device.attached",
            Event::DeviceDetached { .. } => "device.detached",
            Event::DaemonShuttingDown => "daemon.shutting_down",
        }
    }
}
