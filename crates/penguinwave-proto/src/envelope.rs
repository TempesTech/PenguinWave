//! NDJSON frames.
//!
//! One compact JSON value per line, `\n` terminated, UTF-8. `serde_json`
//! compact output never contains a bare newline, so the framing is unambiguous
//! and a session is readable with `socat` when something goes wrong.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::PwError;
use crate::event::Event;
use crate::request::Request;
use crate::response::Response;

/// Client -> daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RequestFrame {
    /// Protocol version. Checked on every message, not just at handshake: a
    /// client that reconnects to an upgraded daemon must fail loudly on its
    /// first real request rather than partway through a routing change.
    pub v: u16,
    /// Client-chosen, echoed back. Requests may be pipelined and responses may
    /// return out of order, so clients must correlate on this rather than
    /// assuming FIFO.
    ///
    /// `u32`, not `u64`, on purpose: ts-rs maps `u64` to `bigint`, and
    /// `JSON.stringify` throws on a `BigInt`. A 64-bit id would break every
    /// request the browser client sends. Four billion requests per session is
    /// not a limit anyone will reach.
    pub id: u32,
    #[serde(flatten)]
    #[ts(flatten)]
    pub request: Request,
}

/// Daemon -> client, in reply to a [`RequestFrame`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResponseFrame {
    pub v: u16,
    pub id: u32,
    #[serde(flatten)]
    #[ts(flatten)]
    pub payload: ResponsePayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ResponsePayload {
    Ok(Response),
    Err(PwError),
}

/// Daemon -> client, unsolicited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EventFrame {
    pub v: u16,
    #[serde(flatten)]
    #[ts(flatten)]
    pub event: Event,
}

impl RequestFrame {
    pub fn new(id: u32, request: Request) -> Self {
        Self {
            v: crate::PROTO_VERSION,
            id,
            request,
        }
    }
}

impl ResponseFrame {
    pub fn ok(id: u32, response: Response) -> Self {
        Self {
            v: crate::PROTO_VERSION,
            id,
            payload: ResponsePayload::Ok(response),
        }
    }

    pub fn err(id: u32, error: PwError) -> Self {
        Self {
            v: crate::PROTO_VERSION,
            id,
            payload: ResponsePayload::Err(error),
        }
    }
}

impl EventFrame {
    pub fn new(event: Event) -> Self {
        Self {
            v: crate::PROTO_VERSION,
            event,
        }
    }
}

/// Whether this build can speak the version on an incoming frame.
pub fn version_supported(v: u16) -> bool {
    let (lo, hi) = crate::PROTO_SUPPORTED;
    v >= lo && v <= hi
}
