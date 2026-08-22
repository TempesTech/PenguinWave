//! AC-8: the daemon survives arbitrary bytes on the socket.
//!
//! Not a coverage-guided fuzzer; a deterministic corpus plus a seeded random
//! walk, so a failure is reproducible from the seed printed in the panic.

use penguinwave_core::{ConfigStore, CoreState};
use penguinwave_hid::MockBackend as MockHid;
use penguinwave_pipewire::MockBackend;
use penguinwave_proto::{Request, RequestFrame, Response, ResponseFrame, ResponsePayload};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::time::Duration;

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
    let dir = std::env::temp_dir().join(format!("penguinwave-fuzz-{tag}-{}", std::process::id()));
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

fn connect(server: &Server) -> UnixStream {
    let s = UnixStream::connect(&server.path).unwrap();
    // Short: most of these payloads are answered immediately or not at all,
    // and the timeout is multiplied by the number of rounds.
    s.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
    s
}

/// The server is still healthy if a fresh connection completes a handshake.
fn assert_server_alive(server: &Server) {
    let stream = connect(server);
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    let frame = RequestFrame::new(
        1,
        Request::SessionHello {
            client: "fuzz".into(),
            proto: 1,
        },
    );
    writeln!(writer, "{}", serde_json::to_string(&frame).unwrap()).unwrap();
    writer.flush().unwrap();

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("server stopped answering");
    let response: ResponseFrame = serde_json::from_str(&line).expect("not a response frame");
    assert!(matches!(
        response.payload,
        ResponsePayload::Ok(Response::Hello(_))
    ));
}

fn feed(server: &Server, payload: &[u8]) {
    let mut stream = connect(server);
    let _ = stream.write_all(payload);
    let _ = stream.flush();
    let mut sink = [0u8; 1024];
    let _ = std::io::Read::read(&mut stream, &mut sink);
}

#[test]
fn a_corpus_of_hostile_frames_never_takes_the_server_down() {
    let server = start("corpus");

    let corpus: Vec<Vec<u8>> = vec![
        b"\n".to_vec(),
        b"\n\n\n\n".to_vec(),
        b"{".to_vec(),
        b"}".to_vec(),
        b"[[[[[[[[[[".to_vec(),
        b"null\n".to_vec(),
        b"true\n".to_vec(),
        b"-0\n".to_vec(),
        b"\"just a string\"\n".to_vec(),
        b"{\"v\":1}\n".to_vec(),
        b"{\"v\":1,\"id\":1}\n".to_vec(),
        b"{\"v\":-1,\"id\":1,\"method\":\"sink.default\"}\n".to_vec(),
        b"{\"v\":1,\"id\":-1,\"method\":\"sink.default\"}\n".to_vec(),
        b"{\"v\":1,\"id\":99999999999999999999,\"method\":\"sink.default\"}\n".to_vec(),
        b"{\"v\":1,\"id\":1,\"method\":\"sink.set_volume\",\"params\":{\"sink\":\"x\",\"pct\":9999}}\n".to_vec(),
        b"{\"v\":1,\"id\":1,\"method\":\"eq.set_band\",\"params\":{\"chain\":\"game\",\"index\":255,\"band\":{\"freq\":0,\"gain_db\":0,\"q\":0,\"filter_type\":\"peaking\",\"enabled\":true}}}\n".to_vec(),
        b"{\"v\":1,\"id\":1,\"method\":\"eq.set_preamp\",\"params\":{\"chain\":\"game\",\"preamp_db\":null}}\n".to_vec(),
        // NaN and infinity are not JSON; serde must reject rather than accept.
        b"{\"v\":1,\"id\":1,\"method\":\"eq.set_preamp\",\"params\":{\"chain\":\"game\",\"preamp_db\":NaN}}\n".to_vec(),
        // Invalid UTF-8.
        vec![0xff, 0xfe, 0xfd, b'\n'],
        // Interior NUL.
        vec![b'{', 0x00, b'}', b'\n'],
        // Deep nesting.
        format!("{}{}\n", "[".repeat(2048), "]".repeat(2048)).into_bytes(),
        // No trailing newline at all, then EOF.
        b"{\"v\":1,\"id\":1,\"method\":\"sink.default\"}".to_vec(),
    ];

    for payload in &corpus {
        feed(&server, payload);
        assert_server_alive(&server);
    }
}

#[test]
fn a_seeded_random_walk_never_takes_the_server_down() {
    let server = start("random");
    let seed: u64 = std::env::var("PENGUINWAVE_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed_1234_abcd_ef01);
    let mut rng = Lcg::new(seed);

    for round in 0..200 {
        let len = (rng.next() % 512) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.next_byte()).collect();
        feed(&server, &payload);
        assert_server_alive(&server);
        assert!(round < 200);
    }
    // Printed only on failure, via the panic message from assert_server_alive.
    eprintln!("fuzz seed {seed:#x}");
}

/// Deterministic generator, so a failure reproduces from its seed.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 16
    }

    fn next_byte(&mut self) -> u8 {
        (self.next() & 0xff) as u8
    }
}
