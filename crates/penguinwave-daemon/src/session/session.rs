//! One client connection.

use crate::transport::framing::{read_frame, write_frame, FrameError};
use crate::session::ratelimit::RateLimiter;
use penguinwave_core::{dispatch, CoreState};
use penguinwave_proto::{
    version_supported, ErrorKind, Event, EventFrame, Hello, PwError, Request, RequestFrame,
    Response, ResponseFrame, PROTO_SUPPORTED,
};
use std::io::{BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Which events a session asked for.
#[derive(Debug, Default)]
struct Subscription(Option<Vec<String>>);

impl Subscription {
    fn wants(&self, event: &Event) -> bool {
        match &self.0 {
            None => false,
            Some(names) => names.iter().any(|n| n == "*" || n == event.name()),
        }
    }
}

pub fn serve(stream: UnixStream, state: Arc<CoreState>, version: &str) {
    let Ok(write_half) = stream.try_clone() else {
        return;
    };
    let writer = Arc::new(Mutex::new(write_half));
    let subscription = Arc::new(Mutex::new(Subscription::default()));
    let open = Arc::new(AtomicBool::new(true));

    let events = state.events.subscribe();
    let pump = spawn_event_pump(events, writer.clone(), subscription.clone(), open.clone());

    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let mut limiter = RateLimiter::new();

    loop {
        match read_frame(&mut reader, &mut buf) {
            Ok(()) => {}
            Err(FrameError::TooLong) => {
                // No id is known, so this cannot be answered as a response.
                eprintln!("[daemon] frame over the size cap; closing session");
                break;
            }
            Err(_) => break,
        }
        if buf.is_empty() {
            continue;
        }
        if !limiter.allow() {
            let _ = send(
                &writer,
                &ResponseFrame::err(
                    0,
                    PwError::new(ErrorKind::BadRequest, "request rate limit exceeded"),
                ),
            );
            break;
        }

        let frame: RequestFrame = match serde_json::from_slice(&buf) {
            Ok(f) => f,
            Err(e) => {
                // Recover the id when the frame is only partly malformed, so
                // the client can still match the error to its request.
                let id = serde_json::from_slice::<serde_json::Value>(&buf)
                    .ok()
                    .and_then(|v| v.get("id").and_then(|i| i.as_u64()))
                    .unwrap_or(0) as u32;
                if send(
                    &writer,
                    &ResponseFrame::err(id, PwError::new(ErrorKind::BadRequest, e.to_string())),
                )
                .is_err()
                {
                    break;
                }
                continue;
            }
        };

        if !version_supported(frame.v) {
            let _ = send(
                &writer,
                &ResponseFrame::err(frame.id, PwError::version_mismatch(PROTO_SUPPORTED)),
            );
            break;
        }

        let reply = handle(&state, &subscription, version, frame.request);
        let frame = match reply {
            Ok(response) => ResponseFrame::ok(frame.id, response),
            Err(e) => ResponseFrame::err(frame.id, e),
        };
        if send(&writer, &frame).is_err() {
            break;
        }
    }

    open.store(false, Ordering::SeqCst);
    let _ = pump.join();
}

fn handle(
    state: &Arc<CoreState>,
    subscription: &Mutex<Subscription>,
    version: &str,
    request: Request,
) -> Result<Response, PwError> {
    match request {
        Request::SessionHello { proto, .. } => {
            if !version_supported(proto) {
                return Err(PwError::version_mismatch(PROTO_SUPPORTED));
            }
            Ok(Response::Hello(Hello {
                daemon: version.to_string(),
                proto: PROTO_SUPPORTED,
                caps: vec!["events".into(), "eq".into(), "chatmix".into()],
            }))
        }
        Request::SessionSubscribe { events } => {
            subscription.lock().unwrap().0 = Some(events);
            Ok(Response::Empty)
        }
        other => dispatch(state, other).map_err(Into::into),
    }
}

/// Forward published events to this session while it is subscribed.
fn spawn_event_pump(
    events: std::sync::mpsc::Receiver<Event>,
    writer: Arc<Mutex<UnixStream>>,
    subscription: Arc<Mutex<Subscription>>,
    open: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for event in events {
            if !open.load(Ordering::SeqCst) {
                break;
            }
            if !subscription.lock().unwrap().wants(&event) {
                continue;
            }
            if send(&writer, &EventFrame::new(event)).is_err() {
                break;
            }
        }
    })
}

fn send<T: serde::Serialize>(writer: &Mutex<UnixStream>, value: &T) -> std::io::Result<()> {
    let mut guard = writer.lock().unwrap();
    write_frame(&mut *guard, value)?;
    guard.flush()
}
