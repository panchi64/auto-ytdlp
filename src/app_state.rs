use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::errors::{AppError, Result};
use crate::utils::settings::Settings;

/// Guard type for file operations lock
pub type FileLockGuard<'a> = std::sync::MutexGuard<'a, ()>;

#[derive(Default)]
struct DownloadStats {
    total_tasks: usize,
    completed_tasks: usize,
    progress: f64,
    initial_total_tasks: usize,
}

#[derive(Default)]
struct DownloadQueues {
    queue: VecDeque<String>,
    active_downloads: HashSet<String>,
}

#[derive(Default)]
struct AppFlags {
    paused: bool,
    shutdown: bool,
    started: bool,
    force_quit: bool,
    completed: bool,
    notification_sent: bool,
}

/// Messages used to update the application state.
///
/// This enum defines all possible state changes that can be applied to the
/// application state through the message passing system.
///
/// Note: Some variants may appear unused in the current implementation
/// but are retained for future extensibility. For example, the `UpdateSettings`
/// variant is referenced in the error handler of the message processor but
/// may not be directly constructed elsewhere in the codebase.
pub enum StateMessage {
    /// Adds a URL to the download queue.
    AddToQueue(String),

    /// Marks a URL as actively downloading.
    AddActiveDownload(String),

    /// Removes a URL from the active downloads.
    RemoveActiveDownload(String),

    /// Increments the completed downloads counter.
    IncrementCompleted,

    /// Sets the paused state.
    SetPaused(bool),

    /// Sets the started state.
    SetStarted(bool),

    /// Sets the shutdown state.
    SetShutdown(bool),

    /// Sets the force quit state.
    SetForceQuit(bool),

    /// Sets the completed state.
    SetCompleted(bool),

    /// Triggers a progress update calculation.
    UpdateProgress,

    /// Replaces the entire download queue with the provided list.
    LoadLinks(Vec<String>),

    /// Updates the application settings.
    #[allow(dead_code)]
    UpdateSettings(Settings),
}

/// A thread-safe application state manager for the script.
///
/// `AppState` manages download queues, active downloads, application flags,
/// and statistics through a "message-passing" architecture. It provides a central
/// point for managing the application's state across multiple threads.
#[derive(Clone)]
pub struct AppState {
    stats: Arc<Mutex<DownloadStats>>,
    queues: Arc<Mutex<DownloadQueues>>,
    flags: Arc<Mutex<AppFlags>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    concurrent: Arc<Mutex<usize>>,
    settings: Arc<Mutex<Settings>>,

    /// Lock for serializing file operations to prevent race conditions
    file_lock: Arc<Mutex<()>>,

    /// Toast notification (message, timestamp)
    toast: Arc<Mutex<Option<(String, Instant)>>>,

    /// Retry statistics counter
    total_retries: Arc<Mutex<usize>>,

    // Channel for state updates
    tx: Sender<StateMessage>,
    rx: Arc<Mutex<Receiver<StateMessage>>>,
}

impl AppState {
    /// Creates a new `AppState` instance with default values.
    ///
    /// Initializes the application state with empty queues, default statistics,
    /// and a welcome message in the logs. Also spawns a background thread to
    /// process state update messages.
    ///
    /// # Returns
    ///
    /// A new `AppState` instance ready for use.
    ///
    /// # Example
    ///
    /// ```
    /// let state = AppState::new();
    /// ```
    pub fn new() -> Self {
        let (tx, rx) = channel();

        // Load settings or use default if loading fails
        let settings = Settings::load().unwrap_or_default();

        let state = AppState {
            stats: Arc::new(Mutex::new(DownloadStats::default())),
            queues: Arc::new(Mutex::new(DownloadQueues::default())),
            flags: Arc::new(Mutex::new(AppFlags::default())),
            logs: Arc::new(Mutex::new(VecDeque::from([
                "Welcome! Press 'S' to start downloads".to_string(),
                "Press 'Q' to quit, 'Shift+Q' to force quit".to_string(),
            ]))),
            concurrent: Arc::new(Mutex::new(settings.concurrent_downloads)),
            settings: Arc::new(Mutex::new(settings)),
            file_lock: Arc::new(Mutex::new(())),
            toast: Arc::new(Mutex::new(None)),
            total_retries: Arc::new(Mutex::new(0)),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };

        // Start message processing thread
        let state_clone = state.clone();
        std::thread::spawn(move || {
            state_clone.process_messages();
        });

        state
    }

    // Process incoming state update messages
    fn process_messages(&self) {
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
                        if let Err(err) = self.handle_remove_active_download(url) {
                            eprintln!("Error removing active download: {}", err);
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
                    StateMessage::UpdateSettings(settings) => {
                        if let Err(err) = self.update_settings(settings) {
                            eprintln!("Error updating settings: {}", err);
                        }
                    }
                }
        }
    }

    // Helper methods for handling individual state messages
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
        queues.active_downloads.insert(url);
        Ok(())
    }

    fn handle_remove_active_download(&self, url: String) -> Result<()> {
        let mut queues = self.queues.lock()?;
        queues.active_downloads.remove(&url);
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

    // Public API methods
    pub fn send(&self, message: StateMessage) -> Result<()> {
        self.tx
            .send(message)
            .map_err(|e| AppError::Channel(e.to_string()))?;
        Ok(())
    }

    pub fn add_log(&self, message: String) -> Result<()> {
        let mut logs = self.logs.lock()?;
        logs.push_back(message);
        // Keep only the latest 1000 log messages (O(1) operation with VecDeque)
        while logs.len() > 1000 {
            logs.pop_front();
        }
        Ok(())
    }

    /// Logs an error message to the TUI logs with context.
    ///
    /// This method formats the error with context and adds it to the visible logs,
    /// making errors visible in the TUI rather than just printing to stderr.
    pub fn log_error(&self, context: &str, error: impl std::fmt::Display) -> Result<()> {
        self.add_log(format!("[ERROR] {}: {}", context, error))
    }

    pub fn get_logs(&self) -> Result<Vec<String>> {
        let logs = self.logs.lock()?;
        Ok(logs.iter().cloned().collect())
    }

    pub fn update_progress(&self) -> Result<()> {
        let mut stats = self.stats.lock()?;
        if stats.initial_total_tasks > 0 {
            stats.progress = stats.completed_tasks as f64 / stats.initial_total_tasks as f64;
        } else {
            stats.progress = 0.0;
        }

        // Check completion
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

    pub fn pop_queue(&self) -> Result<Option<String>> {
        let mut queues = self.queues.lock()?;
        Ok(queues.queue.pop_front())
    }

    pub fn get_queue(&self) -> Result<VecDeque<String>> {
        let queues = self.queues.lock()?;
        Ok(queues.queue.clone())
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

    pub fn get_active_downloads(&self) -> Result<HashSet<String>> {
        let queues = self.queues.lock()?;
        Ok(queues.active_downloads.clone())
    }

    pub fn is_paused(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.paused)
    }

    pub fn is_started(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.started)
    }

    pub fn is_completed(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.completed)
    }

    pub fn is_shutdown(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.shutdown)
    }

    pub fn is_force_quit(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.force_quit)
    }

    pub fn get_progress(&self) -> Result<f64> {
        let stats = self.stats.lock()?;
        Ok(stats.progress)
    }

    pub fn get_completed_tasks(&self) -> Result<usize> {
        let stats = self.stats.lock()?;
        Ok(stats.completed_tasks)
    }

    pub fn get_total_tasks(&self) -> Result<usize> {
        let stats = self.stats.lock()?;
        Ok(stats.total_tasks)
    }

    pub fn get_initial_total_tasks(&self) -> Result<usize> {
        let stats = self.stats.lock()?;
        Ok(stats.initial_total_tasks)
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
        let mut flags = self.flags.lock()?;
        flags.paused = false;
        flags.started = false;
        flags.completed = false;
        flags.notification_sent = false;
        flags.shutdown = false;
        flags.force_quit = false;
        drop(flags);

        let mut stats = self.stats.lock()?;
        stats.completed_tasks = 0;
        stats.progress = 0.0;
        drop(stats);

        // Reset retry counter and clear toast using the public API
        self.reset_retries()?;
        self.clear_toast()?;

        Ok(())
    }

    pub fn clear_logs(&self) -> Result<()> {
        let mut logs = self.logs.lock()?;
        logs.clear();
        logs.push_back("Logs cleared".to_string());
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
    ///
    /// Accepts any type that can be converted into a String (e.g., `&str`, `String`)
    pub fn show_toast(&self, message: impl Into<String>) -> Result<()> {
        let mut toast = self.toast.lock()?;
        *toast = Some((message.into(), Instant::now()));
        Ok(())
    }

    /// Get the current toast message if it hasn't expired (3 seconds)
    pub fn get_toast(&self) -> Result<Option<String>> {
        let mut toast = self.toast.lock()?;
        if let Some((msg, time)) = toast.as_ref() {
            if time.elapsed().as_secs() < 3 {
                return Ok(Some(msg.clone()));
            } else {
                // Toast expired, clear it
                *toast = None;
            }
        }
        Ok(None)
    }

    /// Clear any active toast notification
    pub fn clear_toast(&self) -> Result<()> {
        let mut toast = self.toast.lock()?;
        *toast = None;
        Ok(())
    }

    /// Increment the retry counter
    pub fn increment_retries(&self) -> Result<()> {
        let mut retries = self.total_retries.lock()?;
        *retries += 1;
        Ok(())
    }

    /// Get the total retry count
    pub fn get_total_retries(&self) -> Result<usize> {
        let retries = self.total_retries.lock()?;
        Ok(*retries)
    }

    /// Reset the retry counter (called when starting a new download session)
    pub fn reset_retries(&self) -> Result<()> {
        let mut retries = self.total_retries.lock()?;
        *retries = 0;
        Ok(())
    }
}
