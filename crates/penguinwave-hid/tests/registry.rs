use penguinwave_hid::models::steelseries::{ArctisNova7, DATA_REQUEST};
use penguinwave_hid::{DeviceChange, DeviceRegistry, Headset, HidError, MockBackend};
use penguinwave_proto::{DeviceId, DevicePresence};
use std::sync::Arc;

fn nova7() -> DeviceId {
    ArctisNova7::new().id()
}

/// Status report: byte 2 battery level, bytes 4 and 5 the ChatMix wheel.
fn report(battery: u8, game: u8, chat: u8) -> Vec<u8> {
    let mut r = vec![0u8; 64];
    r[2] = battery;
    r[4] = game;
    r[5] = chat;
    r
}

fn registry(backend: Arc<MockBackend>) -> DeviceRegistry {
    DeviceRegistry::new(backend)
}

#[test]
fn attach_and_detach_are_reported_once() {
    let backend = Arc::new(MockBackend::new());
    let mut reg = registry(backend.clone());

    assert_eq!(reg.refresh().unwrap(), vec![]);

    backend.add(nova7(), vec![]);
    assert_eq!(
        reg.refresh().unwrap(),
        vec![DeviceChange::Attached(nova7())]
    );
    assert_eq!(reg.refresh().unwrap(), vec![]);

    backend.remove(nova7());
    assert_eq!(
        reg.refresh().unwrap(),
        vec![DeviceChange::Detached(nova7())]
    );
    assert_eq!(reg.refresh().unwrap(), vec![]);
}

#[test]
fn unsupported_devices_are_ignored() {
    let backend = Arc::new(MockBackend::new());
    backend.add(DeviceId::new(0x046d, 0xc52b), vec![]);
    let mut reg = registry(backend);

    assert_eq!(reg.refresh().unwrap(), vec![]);
    assert_eq!(reg.selected(), None);
}

#[test]
fn first_attach_selects_and_detach_does_not_clear() {
    let backend = Arc::new(MockBackend::new());
    backend.add(nova7(), vec![]);
    let mut reg = registry(backend.clone());

    reg.refresh().unwrap();
    assert_eq!(reg.selected(), Some(nova7()));

    backend.remove(nova7());
    reg.refresh().unwrap();
    assert_eq!(reg.selected(), Some(nova7()));
    assert!(!reg.is_present(nova7()));
}

#[test]
fn selecting_an_unknown_device_fails() {
    let backend = Arc::new(MockBackend::new());
    let mut reg = registry(backend);
    let err = reg.select(DeviceId::new(0x0000, 0x0001)).unwrap_err();
    assert!(matches!(err, HidError::Unknown(..)));
}

#[test]
fn descriptors_report_presence_without_dropping_known_models() {
    let backend = Arc::new(MockBackend::new());
    let mut reg = registry(backend.clone());
    reg.refresh().unwrap();

    let d = reg.descriptors();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].presence, DevicePresence::Absent);
    assert!(!d[0].capabilities.is_empty());

    backend.add(nova7(), vec![]);
    reg.refresh().unwrap();
    assert_eq!(reg.descriptors()[0].presence, DevicePresence::Connected);
}

#[test]
fn absent_device_is_not_opened() {
    let backend = Arc::new(MockBackend::new());
    let reg = registry(backend);
    let err = reg.read_chatmix(nova7()).unwrap_err();
    assert!(matches!(err, HidError::Absent(..)));
    assert!(err.is_absent());
}

#[test]
fn chatmix_read_writes_the_request_prefix() {
    let backend = Arc::new(MockBackend::new());
    let state = backend.add(nova7(), vec![report(4, 100, 0)]);
    let mut reg = registry(backend);
    reg.refresh().unwrap();

    assert_eq!(reg.read_chatmix(nova7()).unwrap(), 100);
    assert_eq!(state.lock().unwrap().writes, vec![DATA_REQUEST.to_vec()]);
}

#[test]
fn sidetone_and_mic_volume_quantise() {
    let backend = Arc::new(MockBackend::new());
    let state = backend.add(nova7(), vec![]);
    let mut reg = registry(backend);
    reg.refresh().unwrap();

    reg.set_sidetone(nova7(), 0).unwrap();
    reg.set_sidetone(nova7(), 100).unwrap();
    reg.set_microphone_volume(nova7(), 255).unwrap();
    reg.set_microphone_volume(nova7(), 0).unwrap();

    assert_eq!(
        state.lock().unwrap().writes,
        vec![
            vec![0x00, 0x39, 0x00],
            vec![0x00, 0x39, 0x03],
            vec![0x00, 0x37, 0x07],
            vec![0x00, 0x37, 0x00],
        ]
    );
}

#[test]
fn battery_separates_absent_from_faulted() {
    let backend = Arc::new(MockBackend::new());
    let mut reg = registry(backend.clone());

    assert_eq!(reg.battery(nova7()).presence, DevicePresence::Absent);

    backend.add(nova7(), vec![report(2, 50, 50)]);
    reg.refresh().unwrap();
    let info = reg.battery(nova7());
    assert_eq!(info.presence, DevicePresence::Connected);
    assert_eq!(info.level, Some(50));

    // Replies exhausted: the device is present but the read fails.
    assert_eq!(reg.battery(nova7()).presence, DevicePresence::Faulted);
}

#[test]
fn unknown_battery_level_faults_instead_of_panicking() {
    let backend = Arc::new(MockBackend::new());
    backend.add(nova7(), vec![report(9, 0, 0)]);
    let mut reg = registry(backend);
    reg.refresh().unwrap();

    let info = reg.battery(nova7());
    assert_eq!(info.presence, DevicePresence::Faulted);
    assert_eq!(info.level, None);
}

#[test]
fn permission_denied_survives_as_its_own_kind() {
    let backend = Arc::new(MockBackend::new());
    backend.add(nova7(), vec![]);
    backend.deny(nova7());
    let mut reg = registry(backend);
    reg.refresh().unwrap();

    let err = reg.read_chatmix(nova7()).unwrap_err();
    assert!(matches!(err, HidError::PermissionDenied(..)));
    assert!(!err.is_absent());
}

#[test]
fn hidapi_failure_surfaces_from_refresh() {
    let backend = Arc::new(MockBackend::new());
    backend.set_unavailable(true);
    let mut reg = registry(backend);
    assert!(matches!(
        reg.refresh().unwrap_err(),
        HidError::ApiUnavailable(_)
    ));
}
