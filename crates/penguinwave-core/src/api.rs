//! Request dispatch.
//!
//! One function from `Request` to `Response`. The daemon owns framing and
//! sessions; everything a method actually does lives here.

use crate::error::{CoreError, Result};
use crate::state::CoreState;
use crate::system;
use penguinwave_pipewire::icons;
use penguinwave_proto::{Event, Request, Response, UdevStatus};

/// Methods handled by the daemon's session layer rather than here.
pub fn is_session_method(request: &Request) -> bool {
    matches!(
        request,
        Request::SessionHello { .. } | Request::SessionSubscribe { .. }
    )
}

pub fn dispatch(state: &CoreState, request: Request) -> Result<Response> {
    match request {
        Request::SessionHello { .. } | Request::SessionSubscribe { .. } => Err(CoreError::Invalid(
            "session methods are handled by the daemon".into(),
        )),

        Request::SessionSnapshot => Ok(Response::Snapshot(state.snapshot()?)),

        // ---- streams ----
        Request::StreamList => Ok(Response::Streams(state.backend.list_application_streams()?)),

        Request::StreamMove { stream, sink } => {
            state.backend.move_stream_to_sink(&stream, &sink)?;
            state.publish_streams()
        }

        Request::StreamUnassign { stream } => {
            let default = state.backend.default_sink()?;
            state.backend.move_stream_to_sink(&stream, &default)?;
            state.publish_streams()
        }

        Request::StreamSetVolume { stream, pct } => {
            state.backend.set_stream_volume(&stream, pct.min(100))?;
            state.publish_streams()
        }

        Request::StreamSetMute { stream, mute } => {
            state.backend.set_stream_mute(&stream, mute)?;
            state.publish_streams()
        }

        Request::StreamIcon { key } => Ok(Response::Icon(icons::icon_base64(&key))),

        // ---- sinks ----
        Request::SinkCreate { config } => {
            state.backend.create_virtual_sink(&config)?;
            state.publish_sinks()
        }

        Request::SinkDelete { name } => {
            state.backend.delete_virtual_sink(&name)?;
            state.publish_sinks()
        }

        Request::SinkListCustom => {
            let sinks: Vec<_> = state
                .backend
                .list_sinks()?
                .into_iter()
                .filter(|s| s.managed)
                .collect();
            Ok(Response::Sinks(sinks))
        }

        Request::SinkSetVolume { sink, pct } => {
            state.backend.set_sink_volume(&sink, pct.min(100))?;
            state.publish_sinks()
        }

        Request::SinkDefault => Ok(Response::SinkName(state.backend.default_sink()?)),

        Request::SinkCurrentRoute { sink } => {
            Ok(Response::Route(state.backend.sink_current_route(&sink)?))
        }

        Request::SinkRouteToDevice { sink, device } => {
            state.route_sink(&sink, &device)?;
            state.publish_sinks()
        }

        Request::SinkListOutputDevices => Ok(Response::OutputDevices(
            state.backend.list_output_devices()?,
        )),

        // ---- graph ----
        Request::GraphListNodePorts { node } => {
            Ok(Response::Ports(state.backend.list_node_ports(&node)?))
        }

        Request::GraphLink { source, target } => {
            state.backend.link_ports(&source, &target)?;
            state.events.publish(Event::GraphChanged);
            Ok(Response::Empty)
        }

        Request::GraphUnlink { source, target } => {
            state.backend.unlink_ports(&source, &target)?;
            state.events.publish(Event::GraphChanged);
            Ok(Response::Empty)
        }

        // ---- devices ----
        Request::DeviceListSupported => {
            state.refresh_devices()?;
            Ok(Response::Devices(
                state.devices.read().unwrap().descriptors(),
            ))
        }

        Request::DeviceGetSelected => Ok(Response::SelectedDevice(
            state.devices.read().unwrap().selected(),
        )),

        Request::DeviceSetSelected { device } => {
            match device {
                Some(id) => state.select_device(id)?,
                None => state.devices.write().unwrap().clear_selection(),
            }
            Ok(Response::SelectedDevice(
                state.devices.read().unwrap().selected(),
            ))
        }

        Request::DeviceListUser => Ok(Response::UserDevices(state.config.load_user_devices())),

        Request::DeviceAddUser { device } => {
            let mut devices = state.config.load_user_devices();
            if devices.iter().any(|d| d.name == device.name) {
                return Err(CoreError::Invalid(format!(
                    "a user device named {:?} already exists",
                    device.name
                )));
            }
            devices.push(device);
            state.config.save_user_devices(&devices)?;
            Ok(Response::UserDevices(devices))
        }

        Request::DeviceRemoveUser { name } => {
            let mut devices = state.config.load_user_devices();
            let before = devices.len();
            devices.retain(|d| d.name != name);
            if devices.len() == before {
                return Err(CoreError::NotFoundUserDevice(name));
            }
            state.config.save_user_devices(&devices)?;
            Ok(Response::UserDevices(devices))
        }

        // ---- chatmix ----
        Request::ChatmixSetManual { value } => {
            state.chatmix.set_manual(value)?;
            Ok(Response::ChatMix(state.chatmix.state()))
        }

        // ---- eq ----
        Request::EqGetState => Ok(Response::EqState(state.eq.state())),

        Request::EqSetChain { chain, value } => {
            state.eq.set_chain(chain, value)?;
            Ok(Response::EqState(state.eq.state()))
        }

        Request::EqSetBand { chain, index, band } => {
            state.eq.set_band(chain, index as usize, band)?;
            Ok(Response::EqState(state.eq.state()))
        }

        Request::EqAddBand { chain, band } => {
            state.eq.add_band(chain, band)?;
            Ok(Response::EqState(state.eq.state()))
        }

        Request::EqRemoveBand { chain, index } => {
            state.eq.remove_band(chain, index as usize)?;
            Ok(Response::EqState(state.eq.state()))
        }

        Request::EqSetPreamp { chain, preamp_db } => {
            state.eq.set_preamp(chain, preamp_db)?;
            Ok(Response::EqState(state.eq.state()))
        }

        Request::EqListPresets => Ok(Response::EqPresets(state.eq.list_presets())),

        Request::EqSavePreset { name, chain } => {
            state.eq.save_preset(chain, &name)?;
            Ok(Response::EqPresets(state.eq.list_presets()))
        }

        Request::EqApplyPreset { name, chain } => {
            state.eq.apply_preset(chain, &name)?;
            Ok(Response::EqState(state.eq.state()))
        }

        Request::EqDeletePreset { name } => {
            state.eq.delete_preset(&name)?;
            Ok(Response::EqPresets(state.eq.list_presets()))
        }

        Request::EqResetSafeMode => {
            state.eq.reset_safe_mode()?;
            Ok(Response::EqState(state.eq.state()))
        }

        // ---- system ----
        Request::SystemCheckDeps => Ok(Response::SystemDeps(state.check_deps())),

        Request::SystemCheckUdev | Request::SystemInstallUdev => {
            Ok(Response::UdevStatus(state.check_udev()))
        }
    }
}

impl CoreState {
    fn publish_streams(&self) -> Result<Response> {
        let streams = self.backend.list_application_streams()?;
        self.events.publish(Event::StreamListChanged {
            streams: streams.clone(),
        });
        Ok(Response::Streams(streams))
    }

    fn publish_sinks(&self) -> Result<Response> {
        let sinks = self.backend.list_sinks()?;
        self.events.publish(Event::SinkListChanged {
            sinks: sinks.clone(),
        });
        Ok(Response::Sinks(sinks))
    }

    /// Point a sink at an output device, following it with the EQ when one of
    /// the managed sinks moves.
    fn route_sink(&self, sink: &str, device: &str) -> Result<()> {
        self.backend.route_sink_to_device(sink, device)?;
        if let Some(chain) = crate::eq::nodes::chain_for_sink(sink) {
            if self.eq.is_active() {
                self.eq.route_output(chain, device)?;
            }
        }
        Ok(())
    }

    pub fn check_deps(&self) -> penguinwave_proto::SystemDeps {
        system::check_deps(&*self.hid)
    }

    pub fn check_udev(&self) -> UdevStatus {
        system::check_udev(
            &self.devices.read().unwrap(),
            &self.config.load_user_devices(),
        )
    }
}
