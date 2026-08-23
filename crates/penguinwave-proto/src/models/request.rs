//! Client -> daemon methods.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::audio::{PortRef, SinkConfig, StreamRef};
use crate::models::device::{DeviceId, UserDevice};
use crate::models::eq::{EqBand, EqChain, EqChainId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
#[ts(export)]
pub enum Request {
    // ---- session ----
    /// First message on a connection; negotiates the protocol version.
    #[serde(rename = "session.hello")]
    #[ts(rename = "session.hello")]
    SessionHello { client: String, proto: u16 },

    /// Full state. Clients replace wholesale on reconnect, never merge.
    #[serde(rename = "session.snapshot")]
    #[ts(rename = "session.snapshot")]
    SessionSnapshot,

    /// `events` is a list of names, or `["*"]`.
    #[serde(rename = "session.subscribe")]
    #[ts(rename = "session.subscribe")]
    SessionSubscribe { events: Vec<String> },

    // ---- streams ----
    #[serde(rename = "stream.list")]
    #[ts(rename = "stream.list")]
    StreamList,
    #[serde(rename = "stream.move")]
    #[ts(rename = "stream.move")]
    StreamMove { stream: StreamRef, sink: String },
    /// Send a stream back to the default sink.
    #[serde(rename = "stream.unassign")]
    #[ts(rename = "stream.unassign")]
    StreamUnassign { stream: StreamRef },
    #[serde(rename = "stream.set_volume")]
    #[ts(rename = "stream.set_volume")]
    StreamSetVolume { stream: StreamRef, pct: u8 },
    #[serde(rename = "stream.set_mute")]
    #[ts(rename = "stream.set_mute")]
    StreamSetMute { stream: StreamRef, mute: bool },
    /// Fetch one icon by `StreamInfo::icon_key`.
    #[serde(rename = "stream.icon")]
    #[ts(rename = "stream.icon")]
    StreamIcon { key: String },

    // ---- sinks ----
    #[serde(rename = "sink.create")]
    #[ts(rename = "sink.create")]
    SinkCreate { config: SinkConfig },
    #[serde(rename = "sink.delete")]
    #[ts(rename = "sink.delete")]
    SinkDelete { name: String },
    /// Every sink, hardware included.
    #[serde(rename = "sink.list")]
    #[ts(rename = "sink.list")]
    SinkList,
    /// Only the sinks Penguin Wave manages.
    #[serde(rename = "sink.list_custom")]
    #[ts(rename = "sink.list_custom")]
    SinkListCustom,
    #[serde(rename = "sink.set_volume")]
    #[ts(rename = "sink.set_volume")]
    SinkSetVolume { sink: String, pct: u8 },
    #[serde(rename = "sink.default")]
    #[ts(rename = "sink.default")]
    SinkDefault,
    #[serde(rename = "sink.current_route")]
    #[ts(rename = "sink.current_route")]
    SinkCurrentRoute { sink: String },
    #[serde(rename = "sink.route_to_device")]
    #[ts(rename = "sink.route_to_device")]
    SinkRouteToDevice { sink: String, device: String },
    #[serde(rename = "sink.list_output_devices")]
    #[ts(rename = "sink.list_output_devices")]
    SinkListOutputDevices,

    // ---- graph ----
    #[serde(rename = "graph.list_node_ports")]
    #[ts(rename = "graph.list_node_ports")]
    GraphListNodePorts { node: String },
    #[serde(rename = "graph.link")]
    #[ts(rename = "graph.link")]
    GraphLink { source: PortRef, target: PortRef },
    #[serde(rename = "graph.unlink")]
    #[ts(rename = "graph.unlink")]
    GraphUnlink { source: PortRef, target: PortRef },

    // ---- devices ----
    #[serde(rename = "device.list_supported")]
    #[ts(rename = "device.list_supported")]
    DeviceListSupported,
    #[serde(rename = "device.get_selected")]
    #[ts(rename = "device.get_selected")]
    DeviceGetSelected,
    /// `None` clears the selection.
    #[serde(rename = "device.set_selected")]
    #[ts(rename = "device.set_selected")]
    DeviceSetSelected { device: Option<DeviceId> },
    #[serde(rename = "device.list_user")]
    #[ts(rename = "device.list_user")]
    DeviceListUser,
    #[serde(rename = "device.add_user")]
    #[ts(rename = "device.add_user")]
    DeviceAddUser { device: UserDevice },
    #[serde(rename = "device.remove_user")]
    #[ts(rename = "device.remove_user")]
    DeviceRemoveUser { name: String },

    // ---- chatmix ----
    /// Drive the split by hand; `None` hands control back to the wheel.
    #[serde(rename = "chatmix.set_manual")]
    #[ts(rename = "chatmix.set_manual")]
    ChatmixSetManual { value: Option<u8> },

    // ---- eq ----
    #[serde(rename = "eq.get_state")]
    #[ts(rename = "eq.get_state")]
    EqGetState,
    #[serde(rename = "eq.set_chain")]
    #[ts(rename = "eq.set_chain")]
    EqSetChain { chain: EqChainId, value: EqChain },
    #[serde(rename = "eq.set_chain_enabled")]
    #[ts(rename = "eq.set_chain_enabled")]
    EqSetChainEnabled { chain: EqChainId, enabled: bool },
    #[serde(rename = "eq.set_band")]
    #[ts(rename = "eq.set_band")]
    EqSetBand {
        chain: EqChainId,
        index: u8,
        band: EqBand,
    },
    #[serde(rename = "eq.add_band")]
    #[ts(rename = "eq.add_band")]
    EqAddBand { chain: EqChainId, band: EqBand },
    #[serde(rename = "eq.remove_band")]
    #[ts(rename = "eq.remove_band")]
    EqRemoveBand { chain: EqChainId, index: u8 },
    #[serde(rename = "eq.set_preamp")]
    #[ts(rename = "eq.set_preamp")]
    EqSetPreamp { chain: EqChainId, preamp_db: f32 },
    #[serde(rename = "eq.list_presets")]
    #[ts(rename = "eq.list_presets")]
    EqListPresets,
    #[serde(rename = "eq.save_preset")]
    #[ts(rename = "eq.save_preset")]
    EqSavePreset { name: String, chain: EqChainId },
    #[serde(rename = "eq.apply_preset")]
    #[ts(rename = "eq.apply_preset")]
    EqApplyPreset { name: String, chain: EqChainId },
    #[serde(rename = "eq.delete_preset")]
    #[ts(rename = "eq.delete_preset")]
    EqDeletePreset { name: String },
    /// Clear safe mode and retry the filter chain.
    #[serde(rename = "eq.reset_safe_mode")]
    #[ts(rename = "eq.reset_safe_mode")]
    EqResetSafeMode,

    // ---- system ----
    #[serde(rename = "system.check_deps")]
    #[ts(rename = "system.check_deps")]
    SystemCheckDeps,
    #[serde(rename = "system.check_udev")]
    #[ts(rename = "system.check_udev")]
    SystemCheckUdev,
    /// Returns the command for the client to run with elevation; the daemon
    /// stays unprivileged.
    #[serde(rename = "system.install_udev")]
    #[ts(rename = "system.install_udev")]
    SystemInstallUdev,
}
