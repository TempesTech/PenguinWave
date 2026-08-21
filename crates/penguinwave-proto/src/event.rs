//! Daemon -> client pushes.
//!
//! Rule the daemon must hold to: **every method that mutates state emits an
//! event, including back to the client that caused it.** Without the self-echo
//! a client needs two state-update paths (its own optimistic write, plus
//! events from everyone else) and a local-echo special case to reconcile them.
//! With it there is exactly one path. The bug this prevents is invisible until
//! a second client connects, which is precisely when it is hardest to debug.
//!
//! The four events that existed pre-split were enough for a single client that
//! polled for everything else. They are not enough once the CLI, the UI and
//! scripts can all mutate concurrently.

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

    /// Filter chain failed to load and audio was routed around the EQ, or
    /// recovered.
    #[serde(rename = "eq.safe_mode")]
    #[ts(rename = "eq.safe_mode")]
    EqSafeMode { active: bool },

    /// The PipeWire graph changed — links, nodes or the default sink. Debounced
    /// by the daemon; a burst of server activity coalesces into one event.
    #[serde(rename = "graph.changed")]
    #[ts(rename = "graph.changed")]
    GraphChanged,

    /// Streams appeared, vanished, moved, or changed volume/mute.
    #[serde(rename = "stream.list_changed")]
    #[ts(rename = "stream.list_changed")]
    StreamListChanged { streams: Vec<StreamInfo> },

    /// A sink was created or deleted — possibly by another client.
    #[serde(rename = "sink.list_changed")]
    #[ts(rename = "sink.list_changed")]
    SinkListChanged { sinks: Vec<SinkInfo> },

    #[serde(rename = "device.attached")]
    #[ts(rename = "device.attached")]
    DeviceAttached { device: DeviceId },

    #[serde(rename = "device.detached")]
    #[ts(rename = "device.detached")]
    DeviceDetached { device: DeviceId },

    /// Planned stop, as opposed to a crash. Managed sinks and the EQ chain are
    /// deliberately left running, so this means "control is going away", not
    /// "your audio is about to break".
    #[serde(rename = "daemon.shutting_down")]
    #[ts(rename = "daemon.shutting_down")]
    DaemonShuttingDown,
}

impl Event {
    /// Wire name, for subscription matching against `session.subscribe`.
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
