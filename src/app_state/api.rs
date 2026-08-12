use std::time::Instant;

use crate::errors::{AppError, Result};
use crate::utils::settings::Settings;

use super::{AppState, FileLockGuard, StateMessage};

mod flags;
mod logs;
mod queue;

impl AppState {
    pub fn send(&self, message: StateMessage) -> Result<()> {
        self.tx
            .send(message)
            .map_err(|e| AppError::Channel(e.to_string()))?;
        Ok(())
    }

    pub fn update_progress(&self) -> Result<()> {
        let mut stats = self.stats.lock()?;
        if stats.initial_total_tasks > 0 {
            stats.progress = stats.completed_tasks as f64 / stats.initial_total_tasks as f64;
        } else {
            stats.progress = 0.0;
        }

        let flags = self.flags.lock()?;
        let is_completed = stats.completed_tasks == stats.initial_total_tasks
            && stats.initial_total_tasks > 0
            && flags.started
            && !flags.completed;
        drop(flags);

        if is_completed {
            self.send(StateMessage::SetCompleted(true))?;
        }

        Ok(())
    }

    pub fn get_concurrent(&self) -> Result<usize> {
        let concurrent = self.concurrent.lock()?;
        Ok(*concurrent)
    }

    pub fn set_concurrent(&self, value: usize) -> Result<()> {
        let mut concurrent = self.concurrent.lock()?;
        *concurrent = value;
        Ok(())
    }

    pub fn reset_for_new_run(&self) -> Result<()> {
        // Check setting before acquiring stats lock
        let reset_stats = self.settings.lock()?.reset_stats_on_new_batch;

        let mut flags = self.flags.lock()?;
        flags.paused = false;
        flags.started = false;
        flags.completed = false;
        flags.notification_sent = false;
        flags.shutdown = false;
        flags.force_quit = false;
        drop(flags);

        // Get current queue length before stats lock (consistent lock ordering: queues → stats)
        let queue_len = if reset_stats {
            self.queues.lock()?.queue.len()
        } else {
            0 // unused when not resetting
        };

        // Reset the counters as a unit: zeroing the completed count while the
        // denominator keeps accumulating would leave progress unable to reach
        // 100%. When stats are cumulative, both sides carry over instead.
        if reset_stats {
            let mut stats = self.stats.lock()?;
            stats.completed_tasks = 0;
            stats.progress = 0.0;
            stats.total_tasks = queue_len;
            stats.initial_total_tasks = queue_len;
        }

        self.reset_retries()?;
        self.clear_toast()?;
        {
            let mut failed = self.failed_downloads.lock()?;
            failed.clear();
        }

        Ok(())
    }

    /// Acquires the file operations lock to ensure exclusive access to links.txt.
    ///
    /// Multiple worker threads may try to modify links.txt concurrently. This lock
    /// serializes those operations to prevent race conditions where concurrent
    /// read-modify-write operations would lose data.
    ///
    /// # Returns
    ///
    /// A `MutexGuard` that releases the lock when dropped.
    pub fn acquire_file_lock(&self) -> Result<FileLockGuard<'_>> {
        self.file_lock.lock().map_err(AppError::from)
    }

    pub fn get_settings(&self) -> Result<Settings> {
        let settings = self.settings.lock()?;
        Ok(settings.clone())
    }

    pub fn update_settings(&self, new_settings: Settings) -> Result<()> {
        let mut settings = self.settings.lock()?;
        *settings = new_settings;
        Ok(())
    }

    /// Show a toast notification (auto-clears after 3 seconds)
    pub fn show_toast(&self, message: impl Into<String>) -> Result<()> {
        let mut toast = self.toast.lock()?;
        *toast = Some((message.into(), Instant::now()));
        Ok(())
    }

    pub fn clear_toast(&self) -> Result<()> {
        let mut toast = self.toast.lock()?;
        *toast = None;
        Ok(())
    }

    pub fn increment_retries(&self) -> Result<()> {
        let mut retries = self.total_retries.lock()?;
        *retries += 1;
        Ok(())
    }

    /// Reset the retry counter (called when starting a new download session)
    pub fn reset_retries(&self) -> Result<()> {
        let mut retries = self.total_retries.lock()?;
        *retries = 0;
        Ok(())
    }

    /// Drain and return all failed download URLs (for retry)
    pub fn take_failed_downloads(&self) -> Result<Vec<String>> {
        let mut failed = self.failed_downloads.lock()?;
        Ok(failed.drain(..).collect())
    }
}
