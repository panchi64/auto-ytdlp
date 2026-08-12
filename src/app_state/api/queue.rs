use std::collections::VecDeque;
use std::time::Instant;

use crate::app_state::AppState;
use crate::errors::Result;

impl AppState {
    pub fn pop_queue(&self) -> Result<Option<String>> {
        let mut queues = self.queues.lock()?;
        Ok(queues.queue.pop_front())
    }

    pub fn get_queue(&self) -> Result<VecDeque<String>> {
        let queues = self.queues.lock()?;
        Ok(queues.queue.clone())
    }

    /// Number of URLs waiting in the queue, without cloning it.
    pub fn queue_len(&self) -> Result<usize> {
        let queues = self.queues.lock()?;
        Ok(queues.queue.len())
    }

    /// Number of downloads in progress, without cloning the set.
    pub fn active_download_count(&self) -> Result<usize> {
        let queues = self.queues.lock()?;
        Ok(queues.active_downloads.len())
    }

    /// Remove a URL from the queue at a specific index
    pub fn remove_from_queue(&self, index: usize) -> Result<Option<String>> {
        let mut queues = self.queues.lock()?;
        if index < queues.queue.len() {
            let removed = queues.queue.remove(index);
            // Update stats
            if removed.is_some() {
                let mut stats = self.stats.lock()?;
                if stats.total_tasks > 0 {
                    stats.total_tasks -= 1;
                }
                if stats.initial_total_tasks > 0 {
                    stats.initial_total_tasks -= 1;
                }
            }
            Ok(removed)
        } else {
            Ok(None)
        }
    }

    /// Swap two items in the queue
    ///
    /// Returns true if the swap was successful, false if indices were invalid.
    pub fn swap_queue_items(&self, index_a: usize, index_b: usize) -> Result<bool> {
        let mut queues = self.queues.lock()?;
        if index_a < queues.queue.len() && index_b < queues.queue.len() && index_a != index_b {
            queues.queue.swap(index_a, index_b);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Returns just the URLs of active downloads (for compatibility checks)
    /// Test-only: production reads the count via `active_download_count`, but
    /// tests assert *which* URLs are active.
    #[cfg(test)]
    pub fn get_active_downloads(&self) -> Result<std::collections::HashSet<String>> {
        let queues = self.queues.lock()?;
        Ok(queues.active_downloads.keys().cloned().collect())
    }

    /// Refresh the download timestamp for all active downloads to dismiss stale indicators
    pub fn refresh_all_download_timestamps(&self) -> Result<()> {
        let mut queues = self.queues.lock()?;
        for progress in queues.active_downloads.values_mut() {
            progress.last_update = Instant::now();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
