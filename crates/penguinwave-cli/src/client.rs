//! Socket client.

use penguinwave_proto::{
    version_supported, ErrorKind, EventFrame, PwError, Request, RequestFrame, Response,
    ResponseFrame, ResponsePayload, MAX_FRAME_BYTES, PROTO_VERSION,
};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u32,
}

pub fn socket_path() -> Result<PathBuf, PwError> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        PwError::new(
            ErrorKind::Internal,
            "XDG_RUNTIME_DIR is not set; cannot locate the daemon socket",
        )
    })?;
    Ok(PathBuf::from(runtime)
        .join("penguinwave")
        .join("daemon.sock"))
}

/// What to print when there is nothing listening.
///
/// The daemon is a user unit, so the fix is a `--user` command; suggesting the
/// system one sends people to a unit that does not exist.
fn not_running(path: &std::path::Path) -> PwError {
    PwError::new(
        ErrorKind::PipeWireUnavailable,
        format!(
            "no daemon listening on {}\n\nstart it with:\n    systemctl --user enable --now penguinwave.service",
            path.display()
        ),
    )
}

impl Client {
    pub fn connect() -> Result<Self, PwError> {
        let path = socket_path()?;
        let stream = UnixStream::connect(&path).map_err(|_| not_running(&path))?;
        let reader = BufReader::new(stream.try_clone().map_err(io_err)?);
        let mut client = Client {
            reader,
            writer: stream,
            next_id: 1,
        };
        client.hello()?;
        Ok(client)
    }

    fn hello(&mut self) -> Result<(), PwError> {
        let response = self.call(Request::SessionHello {
            client: format!("penguinwave-cli/{}", env!("CARGO_PKG_VERSION")),
            proto: PROTO_VERSION,
        })?;
        if let Response::Hello(hello) = response {
            if !version_supported(hello.proto.0) && !version_supported(hello.proto.1) {
                return Err(PwError::version_mismatch(hello.proto));
            }
        }
        Ok(())
    }

    pub fn call(&mut self, request: Request) -> Result<Response, PwError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&RequestFrame::new(id, request))?;

        // Events can interleave with responses; skip anything not this reply.
        loop {
            let line = self.read_line()?;
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

    pub fn subscribe(&mut self, events: Vec<String>) -> Result<(), PwError> {
        self.call(Request::SessionSubscribe { events })?;
        Ok(())
    }

    /// Blocking iteration over pushed events.
    pub fn next_event(&mut self) -> Result<EventFrame, PwError> {
        loop {
            let line = self.read_line()?;
            if let Ok(frame) = serde_json::from_str::<EventFrame>(&line) {
                return Ok(frame);
            }
        }
    }

    fn send<T: serde::Serialize>(&mut self, value: &T) -> Result<(), PwError> {
        let mut line = serde_json::to_vec(value)
            .map_err(|e| PwError::new(ErrorKind::Internal, format!("serialize request: {e}")))?;
        line.push(b'\n');
        self.writer.write_all(&line).map_err(io_err)?;
        self.writer.flush().map_err(io_err)
    }

    fn read_line(&mut self) -> Result<String, PwError> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).map_err(io_err)?;
        if n == 0 {
            return Err(PwError::new(
                ErrorKind::PipeWireUnavailable,
                "daemon closed the connection",
            ));
        }
        if n > MAX_FRAME_BYTES {
            return Err(PwError::new(
                ErrorKind::BadRequest,
                "daemon sent a frame over the size cap",
            ));
        }
        Ok(line)
    }
}

fn io_err(e: std::io::Error) -> PwError {
    PwError::new(ErrorKind::Internal, e.to_string())
}
