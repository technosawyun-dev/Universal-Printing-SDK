use crate::auth::TokenStore;
use crate::jobs::{JobRegistry, JobState};
use crate::printers::DiscoveredRegistry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, OwnedMutexGuard};

pub struct AppState {
    pub allowed_origins: Vec<String>,
    pub tokens: TokenStore,
    pub jobs: JobRegistry,
    pub discovered: DiscoveredRegistry,
    pub events_tx: broadcast::Sender<String>,
    printer_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl AppState {
    /// Real entrypoint: persists pairing tokens to disk (see `TokenStore::load`).
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self::with_tokens(allowed_origins, TokenStore::load())
    }

    /// For tests: an in-memory `TokenStore` so concurrent test runs never race
    /// on the same real `tokens.json` on disk.
    pub fn with_tokens(allowed_origins: Vec<String>, tokens: TokenStore) -> Self {
        let (events_tx, _rx) = broadcast::channel(100);
        Self {
            allowed_origins,
            tokens,
            jobs: JobRegistry::default(),
            discovered: DiscoveredRegistry::default(),
            events_tx,
            printer_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Serializes print jobs per printer -- concurrent writes to the same
    /// thermal printer can interleave mid-stream and corrupt output (the
    /// existing POS_system frontend works around exactly this with its own
    /// client-side job queue; this closes the same gap on the agent side,
    /// so it holds even for two different, uncoordinated callers). Jobs
    /// targeting *different* printers are unaffected and run concurrently.
    pub async fn lock_printer(&self, printer_id: &str) -> OwnedMutexGuard<()> {
        let mutex = {
            let mut locks = self.printer_locks.lock().unwrap();
            locks.entry(printer_id.to_string()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
        };
        mutex.lock_owned().await
    }

    pub fn broadcast_job_status(&self, job_id: &str, status: JobState) {
        let event = serde_json::json!({ "type": "job.status", "jobId": job_id, "status": status }).to_string();
        let _ = self.events_tx.send(event);
    }

    pub fn broadcast_printers_changed(&self) {
        let _ = self.events_tx.send(serde_json::json!({ "type": "printers.changed" }).to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::TokenStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn lock_printer_serializes_access_to_the_same_printer() {
        let state = Arc::new(AppState::with_tokens(vec![], TokenStore::in_memory()));
        let concurrent = Arc::new(AtomicU32::new(0));
        let max_concurrent = Arc::new(AtomicU32::new(0));

        let mut tasks = Vec::new();
        for _ in 0..5 {
            let state = state.clone();
            let concurrent = concurrent.clone();
            let max_concurrent = max_concurrent.clone();
            tasks.push(tokio::spawn(async move {
                let _guard = state.lock_printer("tcp:1.2.3.4:9100").await;
                let now = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                max_concurrent.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                concurrent.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(max_concurrent.load(Ordering::SeqCst), 1, "two jobs for the same printer ran concurrently");
    }

    #[tokio::test]
    async fn lock_printer_does_not_serialize_different_printers() {
        let state = Arc::new(AppState::with_tokens(vec![], TokenStore::in_memory()));
        let concurrent = Arc::new(AtomicU32::new(0));
        let max_concurrent = Arc::new(AtomicU32::new(0));

        let mut tasks = Vec::new();
        for i in 0..5 {
            let state = state.clone();
            let concurrent = concurrent.clone();
            let max_concurrent = max_concurrent.clone();
            tasks.push(tokio::spawn(async move {
                let _guard = state.lock_printer(&format!("tcp:1.2.3.4:{}", 9100 + i)).await;
                let now = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                max_concurrent.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                concurrent.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        assert!(max_concurrent.load(Ordering::SeqCst) > 1, "different printers should be able to print concurrently");
    }
}
