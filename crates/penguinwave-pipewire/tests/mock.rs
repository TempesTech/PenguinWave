//! Tests for the mock itself.
//!
//! The mock is what P3 will test against, so its behaviour has to match
//! `PactlBackend` where it matters. Anything it gets wrong becomes a false
//! green in every test above it.

use penguinwave_pipewire::{MockBackend, PipeWireBackend};
use penguinwave_proto::{ErrorKind, SinkConfig, StreamRef};

#[test]
fn seeded_from_fixtures() {
    let m = MockBackend::new();
    assert_eq!(m.list_sinks().unwrap().len(), 6);
    assert_eq!(m.list_application_streams().unwrap().len(), 5);
    assert!(!m.list_links().unwrap().is_empty());
    assert!(!m.default_sink().unwrap().is_empty());
}

#[test]
fn empty_session_is_representable() {
    let m = MockBackend::empty();
    assert!(m.list_sinks().unwrap().is_empty());
    assert!(m.list_application_streams().unwrap().is_empty());
    assert!(m.list_links().unwrap().is_empty());
}

#[test]
fn mutations_are_visible_in_later_reads() {
    let m = MockBackend::new();
    m.create_virtual_sink(&SinkConfig {
        name: "test_sink".into(),
        display_name: "Test".into(),
    })
    .unwrap();

    let sinks = m.list_sinks().unwrap();
    let created = sinks
        .iter()
        .find(|s| s.name == "test_sink")
        .expect("created");
    assert!(created.managed);

    m.set_sink_volume("test_sink", 40).unwrap();
    let sinks = m.list_sinks().unwrap();
    assert_eq!(
        sinks.iter().find(|s| s.name == "test_sink").unwrap().volume,
        40
    );

    m.delete_virtual_sink("test_sink").unwrap();
    assert!(!m
        .list_sinks()
        .unwrap()
        .iter()
        .any(|s| s.name == "test_sink"));
}

/// Same rule as the real backend: hardware sinks are not ours to remove.
#[test]
fn hardware_sinks_cannot_be_deleted() {
    let m = MockBackend::new();
    let hardware = m
        .list_sinks()
        .unwrap()
        .into_iter()
        .find(|s| !s.managed)
        .expect("a hardware sink in the fixture");

    let err = m.delete_virtual_sink(&hardware.name).unwrap_err();
    assert_eq!(err.kind, ErrorKind::BadRequest);
}

#[test]
fn stale_stream_index_is_a_conflict() {
    let m = MockBackend::new();
    let stale = StreamRef {
        index: 999_999,
        app_name: "Gone".into(),
        pid: None,
    };
    let err = m.set_stream_volume(&stale, 50).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Conflict);
}

/// The case revalidation exists for: the index is live, but PulseAudio has
/// reused it for a different application.
#[test]
fn recycled_index_belonging_to_another_app_is_a_conflict() {
    let m = MockBackend::new();
    let live = m.list_application_streams().unwrap()[0].stream.clone();

    let impostor = StreamRef {
        app_name: "Something Else".into(),
        ..live
    };
    let err = m.set_stream_volume(&impostor, 50).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Conflict);
}

#[test]
fn moving_to_an_unknown_sink_is_not_found() {
    let m = MockBackend::new();
    let stream = m.list_application_streams().unwrap()[0].stream.clone();
    let err = m.move_stream_to_sink(&stream, "no_such_sink").unwrap_err();
    assert_eq!(err.kind, ErrorKind::NotFound);
}

#[test]
fn routing_replaces_previous_monitor_links() {
    let m = MockBackend::new();
    m.route_sink_to_device("game_sink", "device_a").unwrap();
    m.route_sink_to_device("game_sink", "device_b").unwrap();

    let monitors: Vec<_> = m
        .list_links()
        .unwrap()
        .into_iter()
        .filter(|l| l.source.node_name == "game_sink" && l.source.port_name.starts_with("monitor_"))
        .collect();

    assert_eq!(monitors.len(), 2, "one per channel, not accumulating");
    assert!(monitors.iter().all(|l| l.target.node_name == "device_b"));
}

#[test]
fn unavailable_server_fails_every_call() {
    let m = MockBackend::new();
    m.set_unavailable(true);

    assert_eq!(
        m.list_sinks().unwrap_err().kind,
        ErrorKind::PipeWireUnavailable
    );
    assert_eq!(m.probe().unwrap_err().kind, ErrorKind::PipeWireUnavailable);

    m.set_unavailable(false);
    assert!(m.list_sinks().is_ok(), "recovers when the server returns");
}

/// Callers can assert on what they actually asked the backend to do, which is
/// how P3 will verify that startup adopts sinks instead of recreating them.
#[test]
fn calls_are_recorded_in_order() {
    let m = MockBackend::new();
    m.list_sinks().unwrap();
    m.set_sink_volume("game_sink", 30).unwrap();

    assert_eq!(
        m.calls(),
        vec!["list_sinks", "set_sink_volume game_sink 30"]
    );
}

#[test]
fn ports_and_node_ids_resolve() {
    let m = MockBackend::new();
    assert_eq!(m.list_node_ports("game_sink").unwrap().len(), 4);
    assert!(m.resolve_node_id("game_sink").is_ok());
    assert_eq!(
        m.list_node_ports("no_such_node").unwrap_err().kind,
        ErrorKind::NotFound
    );
}
