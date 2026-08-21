//! NDJSON frames: one compact JSON value per line, `\n` terminated, UTF-8.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::PwError;
use crate::event::Event;
use crate::request::Request;
use crate::response::Response;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RequestFrame {
    /// Checked on every message, not just at handshake.
    pub v: u16,
    /// Client-chosen, echoed back. Responses may arrive out of order.
    ///
    /// `u32` not `u64`: ts-rs maps `u64` to `bigint`, which `JSON.stringify`
    /// rejects.
    pub id: u32,
    #[serde(flatten)]
    #[ts(flatten)]
    pub request: Request,
}

/// Reply to a [`RequestFrame`].
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

/// Unsolicited push.
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

/// Whether this build speaks the version on an incoming frame.
pub fn version_supported(v: u16) -> bool {
    let (lo, hi) = crate::PROTO_SUPPORTED;
    v >= lo && v <= hi
}
