//! Structured errors.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorKind {
    /// Unsupported protocol version. Terminal: the connection closes.
    VersionMismatch,
    /// Malformed frame, unknown method, or invalid parameters.
    BadRequest,
    NotFound,
    /// Headset unplugged or powered off. Expected, not a failure.
    DeviceAbsent,
    PipeWireUnavailable,
    /// A backend command ran and failed.
    PipeWireFailed,
    /// Needs elevation the daemon does not have, typically a missing udev rule.
    PermissionDenied,
    /// State changed between read and write; refetch and retry.
    Conflict,
    Internal,
}

impl ErrorKind {
    /// Whether retrying could succeed. `Conflict` requires a refetch first.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            ErrorKind::PipeWireUnavailable | ErrorKind::PipeWireFailed | ErrorKind::Conflict
        )
    }

    /// Exit code for `penguinwave-cli`. Scripting contract; do not renumber.
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
    /// Set on [`ErrorKind::VersionMismatch`]: the range the daemon speaks.
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
