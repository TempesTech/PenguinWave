mod common;
use common::TempDir;

use penguinwave_core::{sinks, ConfigStore, CoreState};
use penguinwave_hid::MockBackend as MockHid;
use penguinwave_pipewire::{MockBackend, PipeWireBackend};
use penguinwave_proto::{DeviceId, SinkConfig};
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
