/* use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

#[derive(Debug, Clone)]
pub struct Signal {
    pub device_id: String,
    pub remote_offer: String,
}

impl Signal {
    pub fn new(device_id: String, remote_offer: String) -> Self {
        Signal {
            device_id,
            remote_offer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Signaling {
    pub list: Arc<Mutex<HashMap<String, Signal>>>,
    pub notify: Arc<Notify>,
}

impl Signaling {
    pub fn new() -> Self {
        Signaling {
            list: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn add_offer(&self, device_id: String, offer: String) {
        let mut list = self.list.lock().await;
        let signal = Signal::new(device_id.clone(), offer.clone());
        list.insert(device_id, signal);
        drop(list);
        self.notify.notify_waiters();
    }

    pub async fn wait_answer(&self, device_id: &str) -> anyhow::Result<String> {
        loop {
            let guard = self.list.lock().await;
            if let Some(signal) = guard.get(device_id) {
                return Ok(signal.remote_offer.clone());
            }
            drop(guard);
            self.notify.notified().await;
        }
    }
} */