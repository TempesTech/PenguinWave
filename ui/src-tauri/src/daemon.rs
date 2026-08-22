//! Connection to `penguinwave-daemon`.
//!
//! The UI holds no audio logic: this forwards requests, relays events into the
//! webview, and reconnects when the daemon restarts.

use penguinwave_proto::{
    version_supported, ErrorKind, EventFrame, PwError, Request, RequestFrame, Response,
    ResponseFrame, ResponsePayload, PROTO_SUPPORTED, PROTO_VERSION,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BACKOFF_MIN: Duration = Duration::from_millis(100);
const BACKOFF_MAX: Duration = Duration::from_secs(5);

/// Tauri channel carrying every daemon event, the frame itself as payload.
///
/// One channel rather than one per event name: Tauri rejects event names
/// containing a dot, and every protocol name has one. The webview dispatches
/// on the frame's own `event` field.
pub const DAEMON_EVENT: &str = "daemon:event";

/// Tauri channel for socket up/down.
pub const CONNECTION_EVENT: &str = "daemon:connection";

pub fn socket_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|r| PathBuf::from(r).join("penguinwave").join("daemon.sock"))
}

type Pending = Arc<Mutex<HashMap<u32, Sender<ResponsePayload>>>>;

/// Relays daemon events to the webview.
pub trait EventRelay: Send + Sync + 'static {
    fn event(&self, frame: &EventFrame);
    fn connection(&self, connected: bool);
}

pub struct DaemonClient {
    /// Resolved once at construction: the socket does not move under a
    /// running client, and reading it from the environment on every reconnect
    /// makes the client untestable.
    path: Option<PathBuf>,
    writer: Mutex<Option<UnixStream>>,
    pending: Pending,
    next_id: AtomicU32,
    connected: Arc<AtomicBool>,
}

impl DaemonClient {
    pub fn new() -> Arc<Self> {
        Self::with_path(socket_path())
    }

    pub fn with_path(path: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            path,
            writer: Mutex::new(None),
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU32::new(1),
            connected: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Connect, and keep reconnecting for as long as the process runs.
    pub fn spawn(self: &Arc<Self>, relay: impl EventRelay) {
        let client = Arc::clone(self);
        let relay = Arc::new(relay);
        std::thread::spawn(move || {
            let mut backoff = BACKOFF_MIN;
            loop {
                match client.connect_once(relay.as_ref()) {
                    Ok(()) => backoff = BACKOFF_MIN,
                    Err(_) => {
                        backoff = (backoff * 2).min(BACKOFF_MAX);
                    }
                }
                std::thread::sleep(backoff);
            }
        });
    }

    /// One connection's lifetime. Returns when it drops.
    fn connect_once(&self, relay: &dyn EventRelay) -> Result<(), PwError> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| PwError::new(ErrorKind::Internal, "XDG_RUNTIME_DIR is not set"))?;
        let stream = UnixStream::connect(path)
            .map_err(|e| PwError::new(ErrorKind::PipeWireUnavailable, e.to_string()))?;
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .map_err(|e| PwError::new(ErrorKind::Internal, e.to_string()))?,
        );
        *self.writer.lock().unwrap() = Some(stream);

        // The handshake reads its own replies. It cannot go through `call`,
        // which waits on the read loop that has not started yet.
        let result = self.handshake(&mut reader);
        if let Err(e) = result {
            *self.writer.lock().unwrap() = None;
            return Err(e);
        }

        self.connected.store(true, Ordering::SeqCst);
        relay.connection(true);

        self.read_loop(reader, relay);

        self.connected.store(false, Ordering::SeqCst);
        relay.connection(false);
        *self.writer.lock().unwrap() = None;
        // Waiting callers must fail rather than hang until the app closes.
        self.pending.lock().unwrap().clear();
        Ok(())
    }

    /// Negotiate the version and subscribe, before the UI is told it is
    /// connected, so a snapshot taken on that signal cannot race them.
    fn handshake(&self, reader: &mut BufReader<UnixStream>) -> Result<(), PwError> {
        let hello = self.request_inline(
            reader,
            Request::SessionHello {
                client: format!("penguinwave-ui/{}", env!("CARGO_PKG_VERSION")),
                proto: PROTO_VERSION,
            },
        )?;
        if let Response::Hello(hello) = hello {
            if !version_supported(hello.proto.0) && !version_supported(hello.proto.1) {
                return Err(PwError::version_mismatch(PROTO_SUPPORTED));
            }
        }
        self.request_inline(
            reader,
            Request::SessionSubscribe {
                events: vec!["*".into()],
            },
        )?;
        Ok(())
    }

    /// Send and read the reply directly, for use before the read loop runs.
    fn request_inline(
        &self,
        reader: &mut BufReader<UnixStream>,
        request: Request,
    ) -> Result<Response, PwError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.write(&RequestFrame::new(id, request))?;

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => {
                    return Err(PwError::new(
                        ErrorKind::PipeWireUnavailable,
                        "the daemon closed the connection during the handshake",
                    ))
                }
                Ok(_) => {}
            }
            if let Ok(frame) = serde_json::from_str::<ResponseFrame>(&line) {
                if frame.id != id {
                    continue;
                }
                return match frame.payload {
                    ResponsePayload::Ok(r) => Ok(r),
                    ResponsePayload::Err(e) => Err(e),
                };
            }
        }
    }

    fn read_loop(&self, mut reader: BufReader<UnixStream>, relay: &dyn EventRelay) {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            if let Ok(frame) = serde_json::from_str::<ResponseFrame>(&line) {
                if let Some(tx) = self.pending.lock().unwrap().remove(&frame.id) {
                    let _ = tx.send(frame.payload);
                }
                continue;
            }
            if let Ok(frame) = serde_json::from_str::<EventFrame>(&line) {
                relay.event(&frame);
            }
        }
    }

    /// Send a request and wait for its reply.
    pub fn call(&self, request: Request) -> Result<Response, PwError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = channel();
        self.pending.lock().unwrap().insert(id, tx);

        let send = self.write(&RequestFrame::new(id, request));
        if let Err(e) = send {
            self.pending.lock().unwrap().remove(&id);
            return Err(e);
        }

        match rx.recv() {
            Ok(ResponsePayload::Ok(r)) => Ok(r),
            Ok(ResponsePayload::Err(e)) => Err(e),
            Err(_) => Err(PwError::new(
                ErrorKind::PipeWireUnavailable,
                "the daemon closed the connection",
            )),
        }
    }

    fn write<T: serde::Serialize>(&self, value: &T) -> Result<(), PwError> {
        let mut guard = self.writer.lock().unwrap();
        let stream = guard.as_mut().ok_or_else(|| {
            PwError::new(
                ErrorKind::PipeWireUnavailable,
                "not connected to the daemon",
            )
        })?;
        let mut line = serde_json::to_vec(value)
            .map_err(|e| PwError::new(ErrorKind::Internal, e.to_string()))?;
        line.push(b'\n');
        stream
            .write_all(&line)
            .and_then(|_| stream.flush())
            .map_err(|e| PwError::new(ErrorKind::PipeWireUnavailable, e.to_string()))
    }
}
