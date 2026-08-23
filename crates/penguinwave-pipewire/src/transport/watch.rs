//! Graph-change notification.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// What a backend calls when the graph changes. Already debounced.
pub type EventSink = Arc<dyn Fn() + Send + Sync>;

/// Dropping this stops the watch.
pub struct WatchHandle {
    stop: Arc<AtomicBool>,
}

impl WatchHandle {
    pub fn new(stop: Arc<AtomicBool>) -> Self {
        Self { stop }
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
