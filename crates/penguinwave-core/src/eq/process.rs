//! Filter-chain supervision: node readiness and crash-loop detection.

use crate::eq::nodes::sink_node_name;
use crate::error::{CoreError, Result};
use penguinwave_pipewire::PipeWireBackend;
use penguinwave_proto::EqChainId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Exits within [`CRASH_WINDOW`] that trigger safe mode.
pub const CRASH_LIMIT: usize = 3;
pub const CRASH_WINDOW: Duration = Duration::from_secs(30);

const NODE_WAIT_TIMEOUT: Duration = Duration::from_secs(8);
const NODE_WAIT_POLL: Duration = Duration::from_millis(200);

/// Block until every chain node is visible in the graph, returning their ids.
pub fn wait_for_nodes(backend: &dyn PipeWireBackend) -> Result<HashMap<EqChainId, u32>> {
    wait_for_nodes_until(backend, Instant::now() + NODE_WAIT_TIMEOUT, NODE_WAIT_POLL)
}

pub fn wait_for_nodes_until(
    backend: &dyn PipeWireBackend,
    deadline: Instant,
    poll: Duration,
) -> Result<HashMap<EqChainId, u32>> {
    loop {
        let resolved: HashMap<EqChainId, u32> = EqChainId::ALL
            .iter()
            .filter_map(|&chain| {
                backend
                    .resolve_node_id(sink_node_name(chain))
                    .ok()
                    .map(|id| (chain, id))
            })
            .collect();

        if resolved.len() == EqChainId::ALL.len() {
            return Ok(resolved);
        }
        if Instant::now() >= deadline {
            return Err(CoreError::NodeNotFound(
                "timed out waiting for EQ filter-chain nodes".into(),
            ));
        }
        std::thread::sleep(poll);
    }
}

/// Sliding-window crash counter.
#[derive(Default, Debug)]
pub struct CrashTracker {
    exits: Vec<Instant>,
}

impl CrashTracker {
    /// Record an exit; true once the crash-loop limit is reached.
    pub fn record_exit(&mut self) -> bool {
        let now = Instant::now();
        self.exits.push(now);
        self.exits.retain(|t| now.duration_since(*t) < CRASH_WINDOW);
        self.exits.len() >= CRASH_LIMIT
    }

    /// Backoff before the next respawn: 1s, 2s, then 4s.
    pub fn backoff(&self) -> Duration {
        Duration::from_secs(1 << self.exits.len().saturating_sub(1).min(2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_tracker_trips_at_limit() {
        let mut tracker = CrashTracker::default();
        assert!(!tracker.record_exit());
        assert!(!tracker.record_exit());
        assert!(tracker.record_exit());
    }

    #[test]
    fn backoff_grows_then_caps() {
        let mut tracker = CrashTracker::default();
        let expected = [1, 2, 4, 4];
        for want in expected {
            tracker.record_exit();
            assert_eq!(tracker.backoff(), Duration::from_secs(want));
        }
    }
}
