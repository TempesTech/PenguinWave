mod common;
use common::TempDir;

use penguinwave_core::{dispatch, sinks, ConfigStore, CoreState};
use penguinwave_hid::MockBackend as MockHid;
use penguinwave_pipewire::{MockBackend, PipeWireBackend};
use penguinwave_proto::{DeviceId, Request, SinkConfig};
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

/// Status report: byte 2 battery, bytes 4 and 5 the ChatMix wheel.
fn report(game: u8, chat: u8) -> Vec<u8> {
    let mut r = vec![0u8; 64];
    r[2] = 4;
    r[4] = game;
    r[5] = chat;
    r
}

#[test]
fn reconcile_adopts_existing_sinks_instead_of_recreating_them() {
    let audio = Arc::new(MockBackend::new());
    // The fixtures already contain both managed sinks.
    let before = audio.list_sinks().unwrap();
    assert!(sinks::sinks_ready(&before));

    sinks::reconcile(&*audio).unwrap();

    assert!(!audio
        .calls()
        .iter()
        .any(|c| c.starts_with("create_virtual_sink")));
    assert!(!audio
        .calls()
        .iter()
        .any(|c| c.starts_with("delete_virtual_sink")));
}

#[test]
fn reconcile_creates_only_the_missing_sinks() {
    let audio = Arc::new(MockBackend::empty());

    let after = sinks::reconcile(&*audio).unwrap();

    let created: Vec<String> = audio
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("create_virtual_sink"))
        .collect();
    assert_eq!(created.len(), SinkConfig::defaults().len());
    assert!(sinks::sinks_ready(&after));

    // Running it again adopts what the first run made.
    let before = audio.calls().len();
    sinks::reconcile(&*audio).unwrap();
    assert!(!audio.calls()[before..]
        .iter()
        .any(|c| c.starts_with("create_virtual_sink")));
}

#[test]
fn snapshot_reports_devices_and_selection() {
    let h = harness("snapshot");
    h.hid.add(NOVA7, vec![]);
    h.state.refresh_devices().unwrap();

    let snap = h.state.snapshot().unwrap();
    assert_eq!(snap.selected_device, Some(NOVA7));
    assert!(!snap.devices.is_empty());
    assert!(sinks::sinks_ready(&snap.sinks));
}

#[test]
fn device_changes_are_published() {
    let h = harness("devevents");
    let rx = h.state.events.subscribe();

    h.hid.add(NOVA7, vec![]);
    h.state.refresh_devices().unwrap();
    h.hid.remove(NOVA7);
    h.state.refresh_devices().unwrap();

    let names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    assert_eq!(names, ["device.attached", "device.detached"]);
}

#[test]
fn selecting_an_unsupported_device_fails() {
    let h = harness("badselect");
    assert!(h
        .state
        .select_device(DeviceId::new(0x0001, 0x0002))
        .is_err());
}

#[test]
fn the_wheel_drives_both_sink_volumes() {
    let h = harness("wheel");
    h.hid.add(NOVA7, vec![report(100, 0)]);
    h.state.refresh_devices().unwrap();
    let before = h.audio.calls().len();

    assert!(h.state.chatmix.poll_once().unwrap());

    let volumes: Vec<String> = h.audio.calls()[before..]
        .iter()
        .filter(|c| c.starts_with("set_sink_volume"))
        .cloned()
        .collect();
    assert_eq!(
        volumes,
        [
            "set_sink_volume game_sink 100",
            "set_sink_volume chat_sink 0"
        ]
    );
    assert_eq!(h.state.chatmix.state().value, 100);
}

#[test]
fn an_unchanged_wheel_writes_nothing() {
    let h = harness("nochange");
    h.hid.add(NOVA7, vec![report(50, 50), report(50, 50)]);
    h.state.refresh_devices().unwrap();

    assert!(!h.state.chatmix.poll_once().unwrap());
    let before = h.audio.calls().len();
    assert!(!h.state.chatmix.poll_once().unwrap());
    assert_eq!(h.audio.calls().len(), before);
}

#[test]
fn a_manual_value_pins_the_balance_against_the_wheel() {
    let h = harness("manual");
    h.hid.add(NOVA7, vec![report(100, 0)]);
    h.state.refresh_devices().unwrap();

    h.state.chatmix.set_manual(Some(20)).unwrap();
    assert!(h.state.chatmix.state().manual);
    assert_eq!(h.state.chatmix.state().value, 20);

    assert!(!h.state.chatmix.poll_once().unwrap());
    assert_eq!(h.state.chatmix.state().value, 20);

    h.state.chatmix.set_manual(None).unwrap();
    assert!(h.state.chatmix.poll_once().unwrap());
    assert_eq!(h.state.chatmix.state().value, 100);
}

#[test]
fn a_manual_value_is_clamped() {
    let h = harness("manualclamp");
    h.state.chatmix.set_manual(Some(240)).unwrap();
    assert_eq!(h.state.chatmix.state().value, 100);
}

#[test]
fn chatmix_changes_are_published() {
    let h = harness("mixevents");
    h.hid.add(NOVA7, vec![report(100, 0)]);
    h.state.refresh_devices().unwrap();
    let rx = h.state.events.subscribe();

    h.state.chatmix.poll_once().unwrap();

    let names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    assert_eq!(names, ["chatmix.changed"]);
}

#[test]
fn polling_without_a_headset_is_not_an_error() {
    let h = harness("nohead");
    assert!(!h.state.chatmix.poll_once().unwrap());
}

#[test]
fn an_absent_headset_surfaces_as_an_error_not_a_panic() {
    let h = harness("gone");
    h.hid.add(NOVA7, vec![]);
    h.state.refresh_devices().unwrap();
    h.hid.remove(NOVA7);

    assert!(h.state.chatmix.poll_once().is_err());
}

#[test]
fn shutdown_announces_itself_and_stops_the_loop() {
    let h = harness("stop");
    let rx = h.state.events.subscribe();

    h.state.shutdown();

    assert!(!h.state.chatmix.is_running());
    assert!(rx.try_iter().any(|e| e.name() == "daemon.shutting_down"));
}

#[test]
fn start_registers_a_graph_watch() {
    let h = harness("watch");
    h.state.start().unwrap();
    assert!(
        h.audio.is_watching(),
        "nothing subscribed to graph changes, so the UI only ever sees its own mutations"
    );
}

#[test]
fn a_graph_change_publishes_the_lists_that_can_have_moved() {
    let h = harness("watchevents");
    h.state.start().unwrap();
    let rx = h.state.events.subscribe();

    h.audio.fire_graph_change();

    let mut names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["graph.changed", "sink.list_changed", "stream.list_changed"]
    );
}

#[test]
fn a_failing_reconcile_still_starts_the_loops() {
    let h = harness("resilient");
    h.audio.set_unavailable(true);

    // The failure is reported, but the loops are what recover from it.
    assert!(h.state.start().is_err());
    assert!(h.state.chatmix.is_running());
}

#[test]
fn a_headset_plugged_in_after_start_is_noticed() {
    let h = harness("latedevice");
    h.state.start().unwrap();
    assert_eq!(h.state.snapshot().unwrap().selected_device, None);

    h.hid.add(NOVA7, vec![]);
    // The poll loop calls the same path; drive it directly so the test does
    // not wait on a timer.
    h.state.refresh_devices().unwrap();

    assert_eq!(h.state.snapshot().unwrap().selected_device, Some(NOVA7));
}

const HEADSET_SINK: &str = "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo";

/// Built on an empty graph rather than the captured fixture: that one has the
/// EQ output fanned out to two devices, so losing one leaves the route
/// legitimately intact and there is nothing to restore.
#[test]
fn a_route_survives_the_device_disappearing_and_coming_back() {
    let dir = TempDir::new("reroute");
    let audio = Arc::new(MockBackend::empty());
    audio.add_sink(HEADSET_SINK);
    // Without these the EQ start waits out its node timeout on every run.
    audio.add_node("penguinwave_eq_game", 9001);
    audio.add_node("penguinwave_eq_chat", 9002);
    let state = CoreState::new(
        audio.clone(),
        Arc::new(MockHid::new()),
        ConfigStore::new(&dir.0),
    );
    state.start().unwrap();

    // The user picks an output while the headset is on.
    dispatch(
        &state,
        Request::SinkRouteToDevice {
            sink: "game_sink".into(),
            device: HEADSET_SINK.into(),
        },
    )
    .unwrap();
    assert_eq!(
        state.routes().get("game_sink").map(String::as_str),
        Some(HEADSET_SINK)
    );
    assert!(
        state.reconcile_routes().unwrap().is_empty(),
        "already linked"
    );

    // Powered off: the node goes, and every link to it with it.
    audio.remove_sink(HEADSET_SINK);
    audio.drop_links_to(HEADSET_SINK);
    assert!(
        state.reconcile_routes().unwrap().is_empty(),
        "nothing to route to yet"
    );

    // Powered back on: a fresh node carrying no links.
    audio.add_sink(HEADSET_SINK);
    assert_eq!(
        state.reconcile_routes().unwrap(),
        vec!["game_sink".to_string()]
    );
    assert!(
        state.reconcile_routes().unwrap().is_empty(),
        "restored once"
    );
}

#[test]
fn reconcile_leaves_a_sink_that_already_points_somewhere() {
    let h = harness("noreroute");
    h.state.start().unwrap();
    dispatch(
        &h.state,
        Request::SinkRouteToDevice {
            sink: "game_sink".into(),
            device: HEADSET_SINK.into(),
        },
    )
    .unwrap();

    // Still linked, so there is nothing to restore and nothing to fight over.
    assert!(h.state.reconcile_routes().unwrap().is_empty());
}

#[test]
fn a_route_to_an_absent_device_is_kept_but_not_applied() {
    let dir = TempDir::new("absentroute");
    let audio = Arc::new(MockBackend::empty());
    audio.add_sink(HEADSET_SINK);
    // Without these the EQ start waits out its node timeout on every run.
    audio.add_node("penguinwave_eq_game", 9001);
    audio.add_node("penguinwave_eq_chat", 9002);
    let state = CoreState::new(
        audio.clone(),
        Arc::new(MockHid::new()),
        ConfigStore::new(&dir.0),
    );
    state.start().unwrap();
    dispatch(
        &state,
        Request::SinkRouteToDevice {
            sink: "game_sink".into(),
            device: HEADSET_SINK.into(),
        },
    )
    .unwrap();

    audio.remove_sink(HEADSET_SINK);
    audio.drop_links_to(HEADSET_SINK);

    // The user's choice outlives the hardware; it is simply not actionable.
    assert!(state.reconcile_routes().unwrap().is_empty());
    assert_eq!(
        state.routes().get("game_sink").map(String::as_str),
        Some(HEADSET_SINK)
    );
}

#[test]
fn routes_are_reloaded_after_a_restart() {
    let dir = TempDir::new("routepersist");
    let audio = Arc::new(MockBackend::new());
    let hid = Arc::new(MockHid::new());

    let first = CoreState::new(audio.clone(), hid.clone(), ConfigStore::new(&dir.0));
    first.set_route("game_sink", HEADSET_SINK).unwrap();

    // Managed sinks outlive the daemon, so the routing for them must too.
    let second = CoreState::new(audio, hid, ConfigStore::new(&dir.0));
    assert_eq!(
        second.routes().get("game_sink").map(String::as_str),
        Some(HEADSET_SINK)
    );
}

/// Routing a managed sink used to tear down the link feeding the EQ, leaving
/// the monitor wired straight to the device: audio kept playing, the chain
/// received nothing, and every band edit was silent.
#[test]
fn routing_a_sink_keeps_the_eq_in_the_path() {
    let h = harness("routekeepseq");
    h.state.start().unwrap();
    assert!(h.state.eq.is_active(), "test needs the EQ running");

    dispatch(
        &h.state,
        Request::SinkRouteToDevice {
            sink: "game_sink".into(),
            device: HEADSET_SINK.into(),
        },
    )
    .unwrap();

    let monitor_targets: Vec<String> = h
        .audio
        .list_links()
        .unwrap()
        .iter()
        .filter(|l| l.source.node_name == "game_sink" && l.source.port_name.starts_with("monitor"))
        .map(|l| l.target.node_name.clone())
        .collect();

    assert!(
        monitor_targets.iter().any(|n| n == "penguinwave_eq_game"),
        "the EQ receives nothing: {monitor_targets:?}"
    );
    assert!(
        !monitor_targets.iter().any(|n| n == HEADSET_SINK),
        "the monitor feeds the device directly, bypassing the EQ: {monitor_targets:?}"
    );
}
