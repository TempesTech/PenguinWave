//! Daemon -> client results.
//!
//! Self-describing (`kind` + `data`) so a frame is interpretable without
//! knowing which request it answers.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audio::{LinkInfo, OutputDevice, PortInfo, RouteInfo, SinkInfo, StreamInfo};
use crate::device::{ChatMix, DeviceDescriptor, DeviceId, SystemDeps, UdevStatus, UserDevice};
use crate::eq::{EqPresetMeta, EqState};

/// Everything a client needs to render from cold.
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

/// Returned from `session.hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Hello {
    pub daemon: String,
    /// Inclusive protocol range this daemon speaks.
    pub proto: (u16, u16),
    pub caps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
#[ts(export)]
pub enum Response {
    /// Acknowledged; the state change arrives as an event.
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
    /// base64 PNG.
    Icon(Option<String>),
}
