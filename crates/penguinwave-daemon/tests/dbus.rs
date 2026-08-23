//! End-to-end over a real session bus, with mock backends behind the daemon.
//!
//! Ignored by default: this needs an actual D-Bus session bus, which is not
//! guaranteed in every CI environment. Run with `cargo test -- --ignored`.

use penguinwave_core::{ConfigStore, CoreState};
use penguinwave_hid::MockBackend as MockHid;
use penguinwave_pipewire::MockBackend;
use std::sync::Arc;
use zbus::blocking::Connection;

#[path = "../src/dbus.rs"]
mod dbus;

fn tagged(tag: &str) -> (String, String) {
    let pid = std::process::id();
    (
        format!("com.penguinwave.test.{tag}{pid}"),
        format!("/com/penguinwave/test/{tag}{pid}"),
    )
}

#[test]
#[ignore = "needs a real D-Bus session bus"]
fn properties_reflect_live_state() {
    let dir = std::env::temp_dir().join(format!("penguinwave-dbus-props-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = CoreState::new(
        Arc::new(MockBackend::new()),
        Arc::new(MockHid::new()),
        ConfigStore::new(&dir),
    );
    state.start().unwrap();

    let (bus_name, object_path) = tagged("props");
    let _server = dbus::serve_at(&bus_name, &object_path, state).unwrap();

    let client = Connection::session().unwrap();
    let proxy =
        zbus::blocking::Proxy::new(&client, bus_name, object_path, "com.penguinwave.Daemon1")
            .unwrap();

    let eq_active: bool = proxy.get_property("EqActive").unwrap();
    assert!(eq_active, "the fixture starts with the EQ active");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs a real D-Bus session bus"]
fn call_mirrors_the_socket_protocol() {
    let dir = std::env::temp_dir().join(format!("penguinwave-dbus-call-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = CoreState::new(
        Arc::new(MockBackend::new()),
        Arc::new(MockHid::new()),
        ConfigStore::new(&dir),
    );
    state.start().unwrap();

    let (bus_name, object_path) = tagged("call");
    let _server = dbus::serve_at(&bus_name, &object_path, state).unwrap();

    let client = Connection::session().unwrap();
    let proxy =
        zbus::blocking::Proxy::new(&client, bus_name, object_path, "com.penguinwave.Daemon1")
            .unwrap();

    let response: String = proxy
        .call(
            "Call",
            &(r#"{"method":"chatmix.set_manual","params":{"value":42}}"#,),
        )
        .unwrap();
    assert!(
        response.contains(r#""value":42"#),
        "unexpected response: {response}"
    );
    assert!(
        response.contains(r#""manual":true"#),
        "unexpected response: {response}"
    );

    let bad: Result<String, zbus::Error> = proxy.call("Call", &("not json",));
    assert!(
        bad.is_err(),
        "malformed JSON should be rejected, not silently ignored"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
