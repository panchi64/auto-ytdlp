use std::collections::VecDeque;

use crate::errors::{AppError, Result};

use super::{AppState, DownloadProgress, StateMessage};

impl AppState {
    pub(super) fn process_messages(&self) {
        loop {
            let rx = match self.rx.lock() {
                Ok(rx) => rx,
                Err(err) => {
                    // Mutex poisoned - another thread panicked while holding the lock
                    // Exit the processor since state is potentially inconsistent
                    eprintln!("Message processor: mutex poisoned, exiting: {}", err);
                    break;
                }
            };

            let message = match rx.recv() {
                Ok(msg) => msg,
                Err(_) => {
                    // Channel closed (all senders dropped), exit gracefully
                    break;
                }
            };

            drop(rx); // Release lock before processing

            match message {
                StateMessage::AddToQueue(url) => {
                    if let Err(err) = self.handle_add_to_queue(url) {
                        eprintln!("Error adding to queue: {}", err);
                    }
                }
                StateMessage::AddActiveDownload(url) => {
                    if let Err(err) = self.handle_add_active_download(url) {
                        eprintln!("Error adding active download: {}", err);
                    }
                }
                StateMessage::RemoveActiveDownload(url) => {
                    if let Err(err) = self.handle_remove_active_download(&url) {
                        eprintln!("Error removing active download: {}", err);
                    }
                }
                StateMessage::UpdateDownloadProgress { url, progress } => {
                    if let Err(err) = self.handle_update_download_progress(&url, progress) {
                        eprintln!("Error updating download progress: {}", err);
                    }
                }
                StateMessage::IncrementCompleted => {
                    if let Err(err) = self.handle_increment_completed() {
                        eprintln!("Error incrementing completed: {}", err);
                    }
                }
                StateMessage::UpdateProgress => {
                    if let Err(err) = self.update_progress() {
                        eprintln!("Error updating progress: {}", err);
                    }
                }
                StateMessage::SetPaused(value) => {
                    if let Err(err) = self.handle_set_paused(value) {
                        eprintln!("Error setting paused: {}", err);
                    }
                }
                StateMessage::SetStarted(value) => {
                    if let Err(err) = self.handle_set_started(value) {
                        eprintln!("Error setting started: {}", err);
                    }
                }
                StateMessage::SetShutdown(value) => {
                    if let Err(err) = self.handle_set_shutdown(value) {
                        eprintln!("Error setting shutdown: {}", err);
                    }
                }
                StateMessage::SetForceQuit(value) => {
                    if let Err(err) = self.handle_set_force_quit(value) {
                        eprintln!("Error setting force quit: {}", err);
                    }
                }
                StateMessage::SetCompleted(value) => {
                    if let Err(err) = self.handle_set_completed(value) {
                        eprintln!("Error setting completed: {}", err);
                    }
                }
                StateMessage::LoadLinks(links) => {
                    if let Err(err) = self.handle_load_links(links) {
                        eprintln!("Error loading links: {}", err);
                    }
                }
                StateMessage::AddFailedDownload(url) => {
                    if let Err(err) = self.handle_add_failed_download(url) {
                        eprintln!("Error adding failed download: {}", err);
                    }
                }
            }
        }
    }

    fn handle_add_to_queue(&self, url: String) -> Result<()> {
        let mut queues = self.queues.lock()?;
        queues.queue.push_back(url);

        // Update stats
        let mut stats = self.stats.lock()?;
        stats.total_tasks += 1;
        stats.initial_total_tasks += 1;
        Ok(())
    }

    fn handle_add_active_download(&self, url: String) -> Result<()> {
        let mut queues = self.queues.lock()?;
        let progress = DownloadProgress::new(&url);
        queues.active_downloads.insert(url, progress);
        Ok(())
    }

    fn handle_remove_active_download(&self, url: &str) -> Result<()> {
        let mut queues = self.queues.lock()?;
        queues.active_downloads.remove(url);
        Ok(())
    }

    fn handle_update_download_progress(&self, url: &str, progress: DownloadProgress) -> Result<()> {
        let mut queues = self.queues.lock()?;
        if let Some(existing) = queues.active_downloads.get_mut(url) {
            *existing = progress;
        }
        Ok(())
    }

    fn handle_increment_completed(&self) -> Result<()> {
        let mut stats = self.stats.lock()?;
        stats.completed_tasks += 1;
        // Auto-update progress
        self.tx
            .send(StateMessage::UpdateProgress)
            .map_err(|e| AppError::Channel(e.to_string()))?;
        Ok(())
    }

    fn handle_set_paused(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.paused = value;
        Ok(())
    }

    fn handle_set_started(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.started = value;
        Ok(())
    }

    fn handle_set_shutdown(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.shutdown = value;
        Ok(())
    }

    fn handle_set_force_quit(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.force_quit = value;
        Ok(())
    }

    fn handle_set_completed(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.completed = value;
        Ok(())
    }

    fn handle_load_links(&self, links: Vec<String>) -> Result<()> {
        let mut queues = self.queues.lock()?;
        queues.queue = VecDeque::from(links);

        let queue_len = queues.queue.len();
        drop(queues);

        let mut stats = self.stats.lock()?;
        stats.total_tasks = queue_len;
        stats.initial_total_tasks = queue_len;
        Ok(())
    }

    fn handle_add_failed_download(&self, url: String) -> Result<()> {
        let mut failed = self.failed_downloads.lock()?;
        if !failed.contains(&url) {
            failed.push(url);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
