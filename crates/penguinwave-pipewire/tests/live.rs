//! Integration tests against a real PipeWire session.
//!
//! Ignored by default: they need a running audio server.
//! Run with `cargo test -p penguinwave-pipewire --test live -- --ignored`.
//!
//! Read-only. Nothing here creates, deletes, moves or changes volume, so the
//! tests are safe to run against a session someone is using.

use penguinwave_pipewire::{PactlBackend, PipeWireBackend};

fn backend() -> PactlBackend {
    PactlBackend::new()
}

#[test]
#[ignore]
fn probe_reaches_the_server() {
    let info = backend().probe().expect("probe");
    assert!(!info.server_name.is_empty());
    assert!(!info.default_sink.is_empty());
}

#[test]
#[ignore]
fn sinks_are_listed_and_classified() {
    let sinks = backend().list_sinks().expect("list_sinks");
    assert!(!sinks.is_empty());
    for s in &sinks {
        assert!(!s.name.is_empty());
        assert!(s.volume <= 100);
    }
}

#[test]
#[ignore]
fn streams_resolve_a_display_name() {
    for s in backend().list_application_streams().expect("streams") {
        assert!(
            !s.name.is_empty(),
            "stream {:?} has no display name",
            s.stream
        );
        assert!(s.volume <= 100);
    }
}

#[test]
#[ignore]
fn links_carry_port_ids() {
    for l in backend().list_links().expect("list_links") {
        assert!(l.source.id.is_some());
        assert!(l.target.id.is_some());
    }
}

/// The path that was silently broken before: `pw-cli dump` does not exist, and
/// its error was parsed as an empty port list.
#[test]
#[ignore]
fn node_ports_resolve_or_fail_loudly() {
    let b = backend();
    let default = b.default_sink().expect("default_sink");

    let ports = b.list_node_ports(&default).expect("ports for default sink");
    assert!(!ports.is_empty());

    let err = b.list_node_ports("definitely_not_a_node_xyz").unwrap_err();
    assert_eq!(err.kind, penguinwave_proto::ErrorKind::NotFound);
}

#[test]
#[ignore]
fn missing_sink_reports_not_found() {
    let err = backend()
        .sink_current_route("definitely_not_a_sink_xyz")
        .expect("route lookup should not error");
    assert!(err.is_none());
}

/// Measures the icon lookup, which decides whether the daemon needs an on-disk
/// icon cache or per-session client caching is enough.
#[test]
#[ignore]
fn icon_lookup_latency() {
    use penguinwave_pipewire::icons;
    use std::time::Instant;

    let keys = [
        "firefox",
        "brave-browser",
        "steam",
        "code",
        "definitely-missing-xyz",
    ];

    for key in keys {
        let start = Instant::now();
        let found = icons::icon_path(key).is_some();
        let elapsed = start.elapsed();
        println!("{key:<24} {found:<6} {:>8.2?}", elapsed);
    }

    let start = Instant::now();
    for _ in 0..20 {
        let _ = icons::icon_path("definitely-missing-xyz");
    }
    println!("20x worst case (miss):   {:>8.2?}", start.elapsed());
}
