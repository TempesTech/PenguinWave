//! Per-model HID implementations.

pub mod steelseries;

use crate::headset::Headset;
use std::sync::Arc;

/// Every model this build knows how to drive.
pub fn all() -> Vec<Arc<dyn Headset>> {
    vec![Arc::new(steelseries::ArctisNova7::new())]
}
