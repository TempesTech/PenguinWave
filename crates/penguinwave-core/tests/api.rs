mod common;
use common::TempDir;

use penguinwave_core::{dispatch, ConfigStore, CoreState};
use penguinwave_hid::MockBackend as MockHid;
use penguinwave_pipewire::{MockBackend, PipeWireBackend};
use penguinwave_proto::{DeviceId, Request, Response, SinkConfig, UserDevice};
use std::sync::Arc;

const NOVA7: DeviceId = DeviceId {
    vendor_id: 0x1038,
    product_id: 0x2202,
};

struct Harness {
    state: Arc<CoreState>,
    audio: Arc<MockBackend>,
    hid: Arc<MockHid>,
    _dir: TempDir,
}

fn harness(tag: &str) -> Harness {
    let dir = TempDir::new(tag);
    let audio = Arc::new(MockBackend::new());
    let hid = Arc::new(MockHid::new());
    let state = CoreState::new(audio.clone(), hid.clone(), ConfigStore::new(&dir.0));
    Harness {
        state,
        audio,
        hid,
        _dir: dir,
    }
}

fn call(h: &Harness, request: Request) -> Response {
    dispatch(&h.state, request).unwrap()
}

fn user_device(name: &str) -> UserDevice {
    UserDevice {
        name: name.into(),
        vendor_id: Some("1038".into()),
        product_id: Some("2202".into()),
        pipewire_sink: None,
    }
}

#[test]
fn session_methods_are_not_handled_here() {
    let h = harness("session");
    assert!(dispatch(
        &h.state,
        Request::SessionHello {
            client: "test".into(),
            proto: 1
        }
    )
    .is_err());
    assert!(matches!(
        call(&h, Request::SessionSnapshot),
        Response::Snapshot(_)
    ));
}

#[test]
fn every_mutating_method_emits_an_event() {
    let h = harness("mutevents");
    let stream = h.audio.list_application_streams().unwrap()[0]
        .stream
        .clone();
    let rx = h.state.events.subscribe();

    call(
        &h,
        Request::StreamSetVolume {
            stream: stream.clone(),
            pct: 40,
        },
    );
    call(
        &h,
        Request::SinkSetVolume {
            sink: "game_sink".into(),
            pct: 60,
        },
    );
    call(
        &h,
        Request::SinkCreate {
            config: SinkConfig {
                name: "extra".into(),
                display_name: "Extra".into(),
            },
        },
    );

    let names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    assert_eq!(
        names,
        [
            "stream.list_changed",
            "sink.list_changed",
            "sink.list_changed"
        ]
    );
}

#[test]
fn volumes_are_clamped_before_reaching_the_backend() {
    let h = harness("clamp");
    call(
        &h,
        Request::SinkSetVolume {
            sink: "game_sink".into(),
            pct: 250,
        },
    );
    assert!(h
        .audio
        .calls()
        .iter()
        .any(|c| c == "set_sink_volume game_sink 100"));
}

#[test]
fn unassigning_a_stream_sends_it_to_the_default_sink() {
    let h = harness("unassign");
    let stream = h.audio.list_application_streams().unwrap()[0]
        .stream
        .clone();
    let default = h.audio.default_sink().unwrap();

    call(&h, Request::StreamUnassign { stream });

    assert!(h
        .audio
        .calls()
        .iter()
        .any(|c| c.starts_with("move_stream_to_sink") && c.ends_with(&default)));
}

#[test]
fn list_custom_returns_only_managed_sinks() {
    let h = harness("custom");
    let Response::Sinks(sinks) = call(&h, Request::SinkListCustom) else {
        panic!("wrong response kind");
    };
    assert!(!sinks.is_empty());
    assert!(sinks.iter().all(|s| s.managed));
}

#[test]
fn selecting_and_clearing_a_device_round_trips() {
    let h = harness("select");
    h.hid.add(NOVA7, vec![]);
    h.state.refresh_devices().unwrap();

    let Response::SelectedDevice(selected) = call(
        &h,
        Request::DeviceSetSelected {
            device: Some(NOVA7),
        },
    ) else {
        panic!("wrong response kind");
    };
    assert_eq!(selected, Some(NOVA7));

    let Response::SelectedDevice(cleared) = call(&h, Request::DeviceSetSelected { device: None })
    else {
        panic!("wrong response kind");
    };
    assert_eq!(cleared, None);
}

#[test]
fn user_devices_add_reject_duplicates_and_remove() {
    let h = harness("userdev");

    call(
        &h,
        Request::DeviceAddUser {
            device: user_device("Mine"),
        },
    );
    assert!(dispatch(
        &h.state,
        Request::DeviceAddUser {
            device: user_device("Mine")
        }
    )
    .is_err());

    let Response::UserDevices(devices) = call(
        &h,
        Request::DeviceRemoveUser {
            name: "Mine".into(),
        },
    ) else {
        panic!("wrong response kind");
    };
    assert!(devices.is_empty());
    assert!(dispatch(
        &h.state,
        Request::DeviceRemoveUser {
            name: "Mine".into()
        }
    )
    .is_err());
}

#[test]
fn an_invalid_user_device_id_is_refused() {
    let h = harness("baduserdev");
    let mut bad = user_device("Bad");
    bad.vendor_id = Some("' RUN+=\"/bin/sh\"".into());

    assert!(dispatch(&h.state, Request::DeviceAddUser { device: bad }).is_err());
}

#[test]
fn chatmix_manual_can_be_set_and_released() {
    let h = harness("mix");
    let Response::ChatMix(pinned) = call(&h, Request::ChatmixSetManual { value: Some(30) }) else {
        panic!("wrong response kind");
    };
    assert!(pinned.manual);
    assert_eq!(pinned.value, 30);

    let Response::ChatMix(released) = call(&h, Request::ChatmixSetManual { value: None }) else {
        panic!("wrong response kind");
    };
    assert!(!released.manual);
}

#[test]
fn routing_a_managed_sink_follows_through_to_the_eq() {
    let h = harness("route");
    h.state.eq.init();
    let before = h.audio.calls().len();

    call(
        &h,
        Request::SinkRouteToDevice {
            sink: "game_sink".into(),
            device: "alsa_output.pci-0000_00_1f.3.analog-stereo".into(),
        },
    );

    let after = &h.audio.calls()[before..];
    assert!(after.iter().any(|c| c.starts_with("route_sink_to_device")));
    assert!(after.iter().any(|c| c.starts_with("link_ports")));
}

#[test]
fn routing_an_unmanaged_sink_leaves_the_eq_alone() {
    let h = harness("route2");
    h.state.eq.init();
    let before = h.audio.calls().len();

    call(
        &h,
        Request::SinkRouteToDevice {
            sink: "some_other_sink".into(),
            device: "alsa_output.pci-0000_00_1f.3.analog-stereo".into(),
        },
    );

    let after = &h.audio.calls()[before..];
    assert!(!after.iter().any(|c| c.starts_with("link_ports")));
}

#[test]
fn graph_edits_publish_a_graph_change() {
    let h = harness("graph");
    let link = h.audio.list_links().unwrap()[0].clone();
    let rx = h.state.events.subscribe();

    call(
        &h,
        Request::GraphUnlink {
            source: link.source.clone(),
            target: link.target.clone(),
        },
    );
    call(
        &h,
        Request::GraphLink {
            source: link.source,
            target: link.target,
        },
    );

    let names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    assert_eq!(names, ["graph.changed", "graph.changed"]);
}

#[test]
fn a_backend_failure_maps_to_a_wire_error() {
    let h = harness("failure");
    h.audio.set_unavailable(true);

    let err = dispatch(&h.state, Request::StreamList).unwrap_err();
    let wire: penguinwave_proto::PwError = err.into();
    assert!(wire.retryable);
}

#[test]
fn check_deps_and_udev_answer_without_privilege() {
    let h = harness("system");

    let Response::SystemDeps(_) = call(&h, Request::SystemCheckDeps) else {
        panic!("wrong response kind");
    };
    let Response::UdevStatus(status) = call(&h, Request::SystemCheckUdev) else {
        panic!("wrong response kind");
    };
    // The test machine's real rules file is almost certainly not ours.
    if !status.installed {
        let cmd = status.install_command.unwrap();
        assert_eq!(cmd[0], "pkexec");
        assert!(cmd.last().unwrap().contains("udevadm"));
    }
}

#[test]
fn selecting_a_device_publishes_a_selection_change() {
    let h = harness("selectevent");
    h.hid.add(NOVA7, vec![]);
    h.state.refresh_devices().unwrap();
    let rx = h.state.events.subscribe();

    call(
        &h,
        Request::DeviceSetSelected {
            device: Some(NOVA7),
        },
    );
    call(&h, Request::DeviceSetSelected { device: None });

    let names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    assert_eq!(
        names,
        ["device.selection_changed", "device.selection_changed"]
    );
}
