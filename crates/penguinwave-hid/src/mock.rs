//! In-memory [`HidBackend`] for tests.

use crate::error::{HidError, Result};
use crate::transport::{HidBackend, HidTransport};
use penguinwave_proto::DeviceId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct MockState {
    /// Every buffer written, in order, across all opens.
    pub writes: Vec<Vec<u8>>,
    /// Replies handed to successive reads, oldest first.
    pub replies: Vec<Vec<u8>>,
}

/// A backend serving a fixed device list from canned replies.
#[derive(Default)]
pub struct MockBackend {
    devices: Mutex<Vec<DeviceId>>,
    state: Mutex<HashMap<DeviceId, Arc<Mutex<MockState>>>>,
    unavailable: Mutex<bool>,
    denied: Mutex<Vec<DeviceId>>,
    opens: Mutex<usize>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self, id: DeviceId, replies: Vec<Vec<u8>>) -> Arc<Mutex<MockState>> {
        let state = Arc::new(Mutex::new(MockState {
            writes: Vec::new(),
            replies,
        }));
        self.devices.lock().unwrap().push(id);
        self.state.lock().unwrap().insert(id, state.clone());
        state
    }

    /// How many times a device was opened, for asserting handle reuse.
    pub fn opens(&self) -> usize {
        *self.opens.lock().unwrap()
    }

    /// Queue more replies on an existing device.
    pub fn add_replies(&self, id: DeviceId, replies: Vec<Vec<u8>>) {
        if let Some(state) = self.state.lock().unwrap().get(&id) {
            state.lock().unwrap().replies.extend(replies);
        }
    }

    pub fn remove(&self, id: DeviceId) {
        self.devices.lock().unwrap().retain(|d| *d != id);
        self.state.lock().unwrap().remove(&id);
    }

    /// Make `id` enumerate but refuse to open, as a missing udev rule does.
    pub fn deny(&self, id: DeviceId) {
        self.denied.lock().unwrap().push(id);
    }

    pub fn set_unavailable(&self, value: bool) {
        *self.unavailable.lock().unwrap() = value;
    }
}

impl HidBackend for MockBackend {
    fn list(&self) -> Result<Vec<DeviceId>> {
        if *self.unavailable.lock().unwrap() {
            return Err(HidError::ApiUnavailable("mock".into()));
        }
        Ok(self.devices.lock().unwrap().clone())
    }

    fn open(&self, id: DeviceId) -> Result<Box<dyn HidTransport>> {
        if *self.unavailable.lock().unwrap() {
            return Err(HidError::ApiUnavailable("mock".into()));
        }
        if self.denied.lock().unwrap().contains(&id) {
            return Err(HidError::PermissionDenied(id.vendor_id, id.product_id));
        }
        match self.state.lock().unwrap().get(&id) {
            Some(state) => {
                *self.opens.lock().unwrap() += 1;
                Ok(Box::new(MockTransport {
                    state: state.clone(),
                }))
            }
            None => Err(HidError::Absent(id.vendor_id, id.product_id)),
        }
    }

    fn available(&self) -> bool {
        !*self.unavailable.lock().unwrap()
    }
}

struct MockTransport {
    state: Arc<Mutex<MockState>>,
}

impl HidTransport for MockTransport {
    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.state.lock().unwrap().writes.push(data.to_vec());
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
        let mut state = self.state.lock().unwrap();
        if state.replies.is_empty() {
            return Err(HidError::Timeout);
        }
        let reply = state.replies.remove(0);
        let n = reply.len().min(buf.len());
        buf[..n].copy_from_slice(&reply[..n]);
        Ok(n)
    }
}
