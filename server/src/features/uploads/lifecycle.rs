// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Per-original transitions serialize late poster writes with completion and cancellation.

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::{Mutex, OwnedMutexGuard};

/// Weak registrations retain only active or queued media operations, never upload history.
#[derive(Clone, Default)]
pub struct Lifecycle {
    /// Registry access ends before waiting for an individual media lock.
    locks: Arc<Mutex<HashMap<String, Weak<Mutex<()>>>>>,
}

impl Lifecycle {
    /// Serialize one original without delaying operations on other originals.
    pub async fn lock(&self, video: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks.retain(|_, lock| lock.strong_count() > 0);
            let lock = locks.get(video).and_then(Weak::upgrade).unwrap_or_default();
            locks.insert(video.to_owned(), Arc::downgrade(&lock));
            lock
        };
        lock.lock_owned().await
    }
}
