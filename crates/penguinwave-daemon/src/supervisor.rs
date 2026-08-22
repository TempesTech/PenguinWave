//! PipeWire liveness and recovery.

use penguinwave_core::CoreState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const PROBE_INTERVAL: Duration = Duration::from_secs(2);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Probe PipeWire and re-apply the desired state when it comes back.
///
/// Recovery is the ordinary startup path, not a special case: that is the only
/// version of it that stays tested.
pub fn spawn(state: Arc<CoreState>, running: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut was_down = false;
        let mut backoff = PROBE_INTERVAL;

        while running.load(Ordering::SeqCst) {
            std::thread::sleep(backoff);
            if !running.load(Ordering::SeqCst) {
                break;
            }

            match state.backend.probe() {
                Ok(_) => {
                    backoff = PROBE_INTERVAL;
                    if was_down {
                        eprintln!("[daemon] PipeWire is back; re-applying desired state");
                        if let Err(e) = state.resync() {
                            eprintln!("[daemon] resync failed: {e}");
                            continue;
                        }
                        was_down = false;
                    }
                }
                Err(e) => {
                    if !was_down {
                        eprintln!("[daemon] PipeWire unavailable: {e}");
                        was_down = true;
                    }
                    backoff = (backoff * 2).min(BACKOFF_MAX);
                }
            }
        }
    })
}
