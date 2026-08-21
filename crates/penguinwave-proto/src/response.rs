//! Daemon -> client results.
//!
//! Self-describing (`kind` + `data`) rather than bare payloads correlated only
//! by request id. Responses cost one extra field and buy two things: a frame
//! read out of `socat` is interpretable on its own, and a client that
//! mis-tracks its own ids gets a decode error instead of silently parsing a
//! sink list as a stream list.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audio::{LinkInfo, OutputDevice, PortInfo, RouteInfo, SinkInfo, StreamInfo};
use crate::device::{ChatMix, DeviceDescriptor, DeviceId, SystemDeps, UdevStatus, UserDevice};
use crate::eq::{EqPresetMeta, EqState};

/// Everything a client needs to render from cold.
///
/// Returned by `session.snapshot` on connect and on every reconnect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Snapshot {
    pub streams: Vec<StreamInfo>,
    pub sinks: Vec<SinkInfo>,
    pub default_sink: String,
    pub devices: Vec<DeviceDescriptor>,
    pub selected_device: Option<DeviceId>,
    pub user_devices: Vec<UserDevice>,
    pub chatmix: ChatMix,
    pub eq: EqState,
}

/// Daemon identity, returned from `session.hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Hello {
    pub daemon: String,
    /// Inclusive protocol range this daemon speaks.
    pub proto: (u16, u16),
    /// Optional feature flags, so clients can degrade rather than guess.
    pub caps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
#[ts(export)]
pub enum Response {
    /// Acknowledged, nothing to return. The resulting state change arrives as
    /// an event, including to the client that caused it.
    Empty,
    Hello(Hello),
    Snapshot(Snapshot),
    Streams(Vec<StreamInfo>),
    Sinks(Vec<SinkInfo>),
    SinkName(String),
    Route(Option<RouteInfo>),
    OutputDevices(Vec<OutputDevice>),
    Ports(Vec<PortInfo>),
    Links(Vec<LinkInfo>),
    Devices(Vec<DeviceDescriptor>),
    SelectedDevice(Option<DeviceId>),
    UserDevices(Vec<UserDevice>),
    ChatMix(ChatMix),
    EqState(EqState),
    EqPresets(Vec<EqPresetMeta>),
    SystemDeps(SystemDeps),
    UdevStatus(UdevStatus),
    /// base64 PNG for one icon key, or `None` when the key resolves to nothing.
    Icon(Option<String>),
}
