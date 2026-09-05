use std::collections::VecDeque;

use crate::errors::Result;

use super::{AppState, DownloadProgress};

/// A snapshot of UI-relevant state, captured with minimal locking.
///
/// This struct is created once per frame to avoid multiple lock acquisitions
/// during rendering. All fields are owned values to avoid lifetime issues.
#[derive(Clone)]
pub struct UiSnapshot {
    pub progress: f64,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub initial_total_tasks: usize,
    pub started: bool,
    pub paused: bool,
    pub completed: bool,
    pub queue: VecDeque<String>,
    pub active_downloads: Vec<DownloadProgress>,
    pub logs: Vec<String>,
    pub concurrent: usize,
    pub toast: Option<String>,
    pub use_ascii_indicators: bool,
    pub total_retries: usize,
    pub failed_count: usize,
}

impl AppState {
    /// Creates a snapshot of all UI-relevant state with minimal locking.
    ///
    /// Each lock is acquired once per frame rather than once per read. Not
    /// read-only: an expired toast is cleared as a side effect.
    pub fn get_ui_snapshot(&self) -> Result<UiSnapshot> {
        let stats = self.stats.lock()?;
        let progress = stats.progress;
        let completed_tasks = stats.completed_tasks;
        let total_tasks = stats.total_tasks;
        let initial_total_tasks = stats.initial_total_tasks;
        drop(stats);

        let flags = self.flags.lock()?;
        let started = flags.started;
        let paused = flags.paused;
        let completed = flags.completed;
        drop(flags);

        let queues = self.queues.lock()?;
        let queue = queues.queue.clone();
        let active_downloads: Vec<DownloadProgress> =
            queues.active_downloads.values().cloned().collect();
        drop(queues);

        let logs = self.logs.lock()?;
        let logs_vec: Vec<String> = logs.iter().cloned().collect();
        drop(logs);

        let concurrent = *self.concurrent.lock()?;

        // Get toast with expiry check
        let toast = {
            let mut toast_guard = self.toast.lock()?;
            if let Some((msg, time)) = toast_guard.as_ref() {
                if time.elapsed().as_secs() < 3 {
                    Some(msg.clone())
                } else {
                    *toast_guard = None;
                    None
                }
            } else {
                None
            }
        };

        let settings = self.settings.lock()?;
        let use_ascii_indicators = settings.use_ascii_indicators;
        drop(settings);

        let total_retries = *self.total_retries.lock()?;

        let failed_count = self.failed_downloads.lock()?.len();

        Ok(UiSnapshot {
            progress,
            completed_tasks,
            total_tasks,
            initial_total_tasks,
            started,
            paused,
            completed,
            queue,
            active_downloads,
            logs: logs_vec,
            concurrent,
            toast,
            use_ascii_indicators,
            total_retries,
            failed_count,
        })
    }
}

#[cfg(test)]
mod tests;
