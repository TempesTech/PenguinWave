//! Structured errors.
//!
//! Before the split every failure reached the UI as an opaque string, so a
//! headset being unplugged looked exactly like a broken `pactl` invocation.
//! The distinction drives UX: [`ErrorKind::DeviceAbsent`] is a state the UI
//! renders calmly, [`ErrorKind::PipeWireFailed`] is a bug report.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorKind {
    /// Client speaks a protocol version this daemon does not support.
    /// Terminal: the connection closes after this is sent.
    VersionMismatch,
    /// Malformed frame, unknown method, or parameters that do not validate.
    BadRequest,
    /// Named sink, stream, preset, port or device does not exist.
    NotFound,
    /// Headset not plugged in or powered off. Expected, not a failure.
    DeviceAbsent,
    /// The audio server is down or unreachable.
    PipeWireUnavailable,
    /// A backend command ran and failed.
    PipeWireFailed,
    /// Needs elevation the daemon deliberately does not have — typically a
    /// missing udev rule.
    PermissionDenied,
    /// State changed between a client's read and its write. With concurrent
    /// clients a stream can vanish mid-request; refetch and retry.
    Conflict,
    /// A bug in the daemon.
    Internal,
}

impl ErrorKind {
    /// Whether retrying the identical request could plausibly succeed.
    ///
    /// `Conflict` is retryable only after refetching state, which is why the
    /// client-facing advice differs from a blind retry.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            ErrorKind::PipeWireUnavailable | ErrorKind::PipeWireFailed | ErrorKind::Conflict
        )
    }

    /// Process exit code for `penguinwave-cli`, so scripts can branch on the
    /// failure class without parsing text.
    pub fn exit_code(&self) -> i32 {
        match self {
            ErrorKind::VersionMismatch => 3,
            ErrorKind::BadRequest => 4,
            ErrorKind::NotFound => 5,
            ErrorKind::DeviceAbsent => 6,
            ErrorKind::PipeWireUnavailable => 7,
            ErrorKind::PipeWireFailed => 8,
            ErrorKind::PermissionDenied => 9,
            ErrorKind::Conflict => 10,
            ErrorKind::Internal => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PwError {
    pub kind: ErrorKind,
    pub msg: String,
    pub retryable: bool,
    /// Set on [`ErrorKind::VersionMismatch`]: the range the daemon speaks, so
    /// the client can report something actionable instead of "connection lost".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub supported: Option<(u16, u16)>,
}

impl PwError {
    pub fn new(kind: ErrorKind, msg: impl Into<String>) -> Self {
        Self {
            kind,
            msg: msg.into(),
            retryable: kind.retryable(),
            supported: None,
        }
    }

    pub fn version_mismatch(supported: (u16, u16)) -> Self {
        Self {
            kind: ErrorKind::VersionMismatch,
            msg: format!(
                "unsupported protocol version; this daemon speaks {}..={}",
                supported.0, supported.1
            ),
            retryable: false,
            supported: Some(supported),
        }
    }

    pub fn not_found(what: impl std::fmt::Display) -> Self {
        Self::new(ErrorKind::NotFound, format!("{what} not found"))
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(ErrorKind::BadRequest, msg)
    }
}

impl std::fmt::Display for PwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.msg)
    }
}

impl std::error::Error for PwError {}
