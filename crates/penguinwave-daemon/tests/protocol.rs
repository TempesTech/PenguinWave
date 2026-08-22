//! End-to-end over a real unix socket, with mock backends behind the daemon.

use penguinwave_core::{ConfigStore, CoreState};
use penguinwave_hid::MockBackend as MockHid;
use penguinwave_pipewire::MockBackend;
use penguinwave_proto::{
    ErrorKind, EventFrame, Request, RequestFrame, Response, ResponseFrame, ResponsePayload,
    MAX_FRAME_BYTES,
};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;

#[path = "../src/framing.rs"]
mod framing;
#[path = "../src/ratelimit.rs"]
mod ratelimit;
#[path = "../src/session.rs"]
mod session;

struct Server {
    path: std::path::PathBuf,
    dir: std::path::PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start(tag: &str) -> Server {
    let dir = std::env::temp_dir().join(format!("penguinwave-daemon-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("daemon.sock");

    let listener = UnixListener::bind(&path).unwrap();
    let state = CoreState::new(
        Arc::new(MockBackend::new()),
        Arc::new(MockHid::new()),
        ConfigStore::new(dir.join("config")),
    );

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let state = state.clone();
            std::thread::spawn(move || session::serve(stream, state, "0.3.0-test"));
        }
    });

    Server { path, dir }
}

struct Conn {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

fn connect(server: &Server) -> Conn {
    let stream = UnixStream::connect(&server.path).unwrap();
    // Every read in these tests either expects data or expects nothing; a
    // timeout turns "nothing" into a result instead of a hung test run.
    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .unwrap();
    Conn {
        reader: BufReader::new(stream.try_clone().unwrap()),
        writer: stream,
    }
}

impl Conn {
    fn send_raw(&mut self, line: &str) {
        self.writer.write_all(line.as_bytes()).unwrap();
        self.writer.write_all(b"\n").unwrap();
        self.writer.flush().unwrap();
    }

    fn send(&mut self, id: u32, request: Request) {
        self.send_raw(&serde_json::to_string(&RequestFrame::new(id, request)).unwrap());
    }

    fn read_line(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line),
        }
    }

    fn response(&mut self) -> ResponseFrame {
        loop {
            let line = self.read_line().expect("connection closed");
            if let Ok(frame) = serde_json::from_str::<ResponseFrame>(&line) {
                return frame;
            }
        }
    }

    fn ok(&mut self, id: u32, request: Request) -> Response {
        self.send(id, request);
        match self.response().payload {
            ResponsePayload::Ok(r) => r,
            ResponsePayload::Err(e) => panic!("expected ok, got {e:?}"),
        }
    }

    fn err(&mut self, id: u32, request: Request) -> penguinwave_proto::PwError {
        self.send(id, request);
        match self.response().payload {
            ResponsePayload::Err(e) => e,
            ResponsePayload::Ok(r) => panic!("expected err, got {r:?}"),
        }
    }
}

fn hello() -> Request {
    Request::SessionHello {
        client: "test".into(),
        proto: 1,
    }
}

#[test]
fn hello_then_snapshot() {
    let server = start("hello");
    let mut conn = connect(&server);

    let Response::Hello(h) = conn.ok(1, hello()) else {
        panic!("wrong response kind");
    };
    assert_eq!(h.proto, (1, 1));
    assert!(matches!(
        conn.ok(2, Request::SessionSnapshot),
        Response::Snapshot(_)
    ));
}

#[test]
fn the_response_id_echoes_the_request_id() {
    let server = start("ids");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    conn.send(4242, Request::SinkDefault);
    assert_eq!(conn.response().id, 4242);
}

#[test]
fn a_wrong_protocol_version_is_refused_and_closes_the_session() {
    let server = start("version");
    let mut conn = connect(&server);

    conn.send_raw(r#"{"v":99,"id":1,"method":"sink.default"}"#);
    let frame = conn.response();
    let ResponsePayload::Err(e) = frame.payload else {
        panic!("expected error");
    };
    assert_eq!(e.kind, ErrorKind::VersionMismatch);
    assert_eq!(e.supported, Some((1, 1)));
    assert!(conn.read_line().is_none(), "session should have closed");
}

#[test]
fn malformed_frames_are_answered_and_the_session_survives() {
    let server = start("malformed");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    for bad in [
        "{",
        "not json at all",
        "[]",
        "null",
        r#"{"v":1,"id":7}"#,
        r#"{"v":1,"id":8,"method":"no.such.method"}"#,
        r#"{"v":1,"id":9,"method":"sink.set_volume","params":{"sink":1,"pct":"loud"}}"#,
    ] {
        conn.send_raw(bad);
        let frame = conn.response();
        assert!(
            matches!(frame.payload, ResponsePayload::Err(_)),
            "accepted {bad:?}"
        );
    }

    // Still usable afterwards.
    assert!(matches!(
        conn.ok(10, Request::SinkDefault),
        Response::SinkName(_)
    ));
}

#[test]
fn a_recoverable_id_is_echoed_on_a_malformed_frame() {
    let server = start("recoverid");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    conn.send_raw(r#"{"v":1,"id":77,"method":"bogus"}"#);
    assert_eq!(conn.response().id, 77);
}

#[test]
fn an_empty_line_is_ignored() {
    let server = start("empty");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    conn.send_raw("");
    conn.send_raw("");
    assert!(matches!(
        conn.ok(2, Request::SinkDefault),
        Response::SinkName(_)
    ));
}

#[test]
fn an_oversized_frame_closes_the_session_without_killing_the_server() {
    let server = start("oversize");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    let huge = "x".repeat(MAX_FRAME_BYTES + 1024);
    let _ = conn.writer.write_all(huge.as_bytes());
    let _ = conn.writer.write_all(b"\n");
    let _ = conn.writer.flush();
    assert!(conn.read_line().is_none());

    // The server still accepts new connections.
    let mut fresh = connect(&server);
    assert!(matches!(fresh.ok(1, hello()), Response::Hello(_)));
}

#[test]
fn events_reach_only_subscribed_sessions() {
    let server = start("subs");
    let mut subscriber = connect(&server);
    let mut silent = connect(&server);
    subscriber.ok(1, hello());
    silent.ok(1, hello());

    subscriber.ok(
        2,
        Request::SessionSubscribe {
            events: vec!["*".into()],
        },
    );

    let mix = Request::ChatmixSetManual { value: Some(70) };
    subscriber.send(3, mix);
    // Response and event both arrive; find the event.
    let mut saw_event = false;
    for _ in 0..4 {
        let Some(line) = subscriber.read_line() else {
            break;
        };
        if let Ok(frame) = serde_json::from_str::<EventFrame>(&line) {
            if frame.event.name() == "chatmix.changed" {
                saw_event = true;
                break;
            }
        }
    }
    assert!(saw_event, "subscriber never received the event");

    // The unsubscribed session sees only its own responses.
    silent.send(2, Request::SinkDefault);
    let line = silent.read_line().unwrap();
    assert!(serde_json::from_str::<EventFrame>(&line).is_err());
}

#[test]
fn a_mutation_echoes_back_to_the_client_that_made_it() {
    let server = start("selfecho");
    let mut conn = connect(&server);
    conn.ok(1, hello());
    conn.ok(
        2,
        Request::SessionSubscribe {
            events: vec!["chatmix.changed".into()],
        },
    );

    conn.send(3, Request::ChatmixSetManual { value: Some(25) });

    let mut saw_event = false;
    for _ in 0..4 {
        let Some(line) = conn.read_line() else { break };
        if serde_json::from_str::<EventFrame>(&line).is_ok() {
            saw_event = true;
            break;
        }
    }
    assert!(saw_event, "the caller must receive its own event");
}

#[test]
fn a_filtered_subscription_drops_other_events() {
    let server = start("filter");
    let mut conn = connect(&server);
    conn.ok(1, hello());
    conn.ok(
        2,
        Request::SessionSubscribe {
            events: vec!["eq.state_changed".into()],
        },
    );

    conn.send(3, Request::ChatmixSetManual { value: Some(15) });
    let line = conn.read_line().unwrap();
    assert!(
        serde_json::from_str::<EventFrame>(&line).is_err(),
        "received an unsubscribed event"
    );
}

#[test]
fn one_session_closing_does_not_disturb_another() {
    let server = start("isolation");
    let mut first = connect(&server);
    let mut second = connect(&server);
    first.ok(1, hello());
    second.ok(1, hello());

    drop(first);

    assert!(matches!(
        second.ok(2, Request::SinkDefault),
        Response::SinkName(_)
    ));
}

#[test]
fn requests_may_be_pipelined() {
    let server = start("pipeline");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    for id in 2..=6 {
        conn.send(id, Request::SinkDefault);
    }
    let mut seen = Vec::new();
    for _ in 2..=6 {
        seen.push(conn.response().id);
    }
    seen.sort_unstable();
    assert_eq!(seen, vec![2, 3, 4, 5, 6]);
}

#[test]
fn session_methods_are_answered_by_the_daemon_not_core() {
    let server = start("sessionmethods");
    let mut conn = connect(&server);

    // hello works before anything else, and subscribe is accepted.
    assert!(matches!(conn.ok(1, hello()), Response::Hello(_)));
    assert!(matches!(
        conn.ok(2, Request::SessionSubscribe { events: vec![] }),
        Response::Empty
    ));
}

#[test]
fn an_unsupported_hello_version_is_refused() {
    let server = start("hellover");
    let mut conn = connect(&server);

    let e = conn.err(
        1,
        Request::SessionHello {
            client: "test".into(),
            proto: 42,
        },
    );
    assert_eq!(e.kind, ErrorKind::VersionMismatch);
}

#[test]
fn errors_carry_a_retryable_flag_and_an_exit_code() {
    let server = start("errcodes");
    let mut conn = connect(&server);
    conn.ok(1, hello());

    let e = conn.err(
        2,
        Request::EqDeletePreset {
            name: "Flat".into(),
        },
    );
    assert_eq!(e.kind, ErrorKind::BadRequest);
    assert!(!e.retryable);
    assert_eq!(e.kind.exit_code(), 4);
}
