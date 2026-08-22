//! The UI's socket client, against a stub daemon.
//!
//! The handshake previously deadlocked: it waited on the read loop that had
//! not started yet, so the UI never reported itself connected.

// Compiled standalone here, so the parts only the app uses look unused.
#[path = "../src/daemon.rs"]
#[allow(dead_code)]
mod daemon;

use daemon::{DaemonClient, EventRelay};
use penguinwave_proto::{
    Event, EventFrame, Hello, Request, RequestFrame, Response, ResponseFrame, PROTO_SUPPORTED,
};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);

struct Stub {
    dir: std::path::PathBuf,
    path: std::path::PathBuf,
}

impl Drop for Stub {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A daemon that answers the handshake and echoes one event on demand.
fn stub(tag: &str) -> (Stub, Receiver<String>) {
    let dir =
        std::env::temp_dir().join(format!("penguinwave-uiclient-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("penguinwave")).unwrap();
    let path = dir.join("penguinwave").join("daemon.sock");

    let listener = UnixListener::bind(&path).unwrap();
    let (seen_tx, seen_rx) = channel();

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let seen = seen_tx.clone();
            std::thread::spawn(move || serve(stream, seen));
        }
    });

    (Stub { dir, path }, seen_rx)
}

fn serve(stream: UnixStream, seen: Sender<String>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let Ok(frame) = serde_json::from_str::<RequestFrame>(&line) else {
            continue;
        };
        let response = match &frame.request {
            Request::SessionHello { .. } => Response::Hello(Hello {
                daemon: "0.3.0-stub".into(),
                proto: PROTO_SUPPORTED,
                caps: vec![],
            }),
            Request::SessionSubscribe { .. } => Response::Empty,
            Request::SinkDefault => Response::SinkName("game_sink".into()),
            _ => Response::Empty,
        };
        let _ = seen.send(method_of(&frame.request).to_string());

        let reply = serde_json::to_string(&ResponseFrame::ok(frame.id, response)).unwrap();
        if writeln!(writer, "{reply}").is_err() {
            return;
        }
        let _ = writer.flush();

        // Push an event after the subscription, to prove the relay works.
        if matches!(frame.request, Request::SessionSubscribe { .. }) {
            let event = EventFrame::new(Event::GraphChanged);
            let _ = writeln!(writer, "{}", serde_json::to_string(&event).unwrap());
            let _ = writer.flush();
        }
    }
}

fn method_of(request: &Request) -> &'static str {
    match request {
        Request::SessionHello { .. } => "session.hello",
        Request::SessionSubscribe { .. } => "session.subscribe",
        Request::SinkDefault => "sink.default",
        _ => "other",
    }
}

#[derive(Clone, Default)]
struct Recorder {
    events: Arc<Mutex<Vec<String>>>,
    connections: Arc<Mutex<Vec<bool>>>,
    tx: Option<Arc<Mutex<Sender<bool>>>>,
}

impl EventRelay for Recorder {
    fn event(&self, frame: &EventFrame) {
        self.events
            .lock()
            .unwrap()
            .push(frame.event.name().to_string());
    }

    fn connection(&self, connected: bool) {
        self.connections.lock().unwrap().push(connected);
        if let Some(tx) = &self.tx {
            let _ = tx.lock().unwrap().send(connected);
        }
    }
}

fn recorder() -> (Recorder, Receiver<bool>) {
    let (tx, rx) = channel();
    (
        Recorder {
            tx: Some(Arc::new(Mutex::new(tx))),
            ..Default::default()
        },
        rx,
    )
}

#[test]
fn the_handshake_completes_and_reports_connected() {
    let (stub, seen) = stub("handshake");
    let (recorder, connections) = recorder();
    let client = DaemonClient::with_path(Some(stub.path.clone()));
    client.spawn(recorder.clone());

    assert!(
        connections.recv_timeout(TIMEOUT).unwrap(),
        "client never reported itself connected"
    );
    assert!(client.is_connected());

    // Handshake order matters: subscribe must land before the UI is told it
    // is connected, or a snapshot taken on that signal races it.
    assert_eq!(seen.recv_timeout(TIMEOUT).unwrap(), "session.hello");
    assert_eq!(seen.recv_timeout(TIMEOUT).unwrap(), "session.subscribe");
}

#[test]
fn requests_work_after_the_handshake() {
    let (stub, _seen) = stub("request");
    let (recorder, connections) = recorder();
    let client = DaemonClient::with_path(Some(stub.path.clone()));
    client.spawn(recorder);
    connections.recv_timeout(TIMEOUT).unwrap();

    let response = client.call(Request::SinkDefault).unwrap();
    assert!(matches!(response, Response::SinkName(name) if name == "game_sink"));
}

#[test]
fn pushed_events_reach_the_relay() {
    let (stub, _seen) = stub("events");
    let (recorder, connections) = recorder();
    let client = DaemonClient::with_path(Some(stub.path.clone()));
    client.spawn(recorder.clone());
    connections.recv_timeout(TIMEOUT).unwrap();

    // The stub pushes one event right after the subscription.
    let deadline = std::time::Instant::now() + TIMEOUT;
    while recorder.events.lock().unwrap().is_empty() {
        assert!(std::time::Instant::now() < deadline, "no event arrived");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(recorder.events.lock().unwrap()[0], "graph.changed");
}

#[test]
fn a_call_without_a_daemon_fails_instead_of_hanging() {
    let nowhere = std::env::temp_dir().join("penguinwave-uiclient-none/daemon.sock");
    let client = DaemonClient::with_path(Some(nowhere));

    assert!(!client.is_connected());
    assert!(client.call(Request::SinkDefault).is_err());
}
