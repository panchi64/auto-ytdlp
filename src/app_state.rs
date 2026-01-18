use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::errors::{AppError, Result};
use crate::utils::display::truncate_url_for_display;
use crate::utils::settings::Settings;

/// Progress information for a single download.
///
/// Tracks percentage, speed, ETA, and other metadata for displaying
/// per-download progress bars in the TUI.
#[derive(Clone, Debug)]
pub struct DownloadProgress {
    /// Truncated URL or video title for display
    pub display_name: String,
    /// Current phase: "downloading", "processing", "merging", "finished", "error"
    pub phase: String,
    /// Download percentage (0.0 - 100.0)
    pub percent: f64,
    /// Download speed (e.g., "1.5MiB/s")
    pub speed: Option<String>,
    /// Estimated time remaining (e.g., "00:05:23")
    pub eta: Option<String>,
    /// Bytes downloaded so far
    pub downloaded_bytes: Option<u64>,
    /// Total file size in bytes
    pub total_bytes: Option<u64>,
    /// Current fragment index (for HLS/DASH streams)
    pub fragment_index: Option<u32>,
    /// Total fragment count (for HLS/DASH streams)
    pub fragment_count: Option<u32>,
    /// Timestamp of last progress update (for staleness detection)
    pub last_update: Instant,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            phase: "downloading".to_string(),
            percent: 0.0,
            speed: None,
            eta: None,
            downloaded_bytes: None,
            total_bytes: None,
            fragment_index: None,
            fragment_count: None,
            last_update: Instant::now(),
        }
    }
}

impl DownloadProgress {
    /// Creates a new DownloadProgress for the given URL
    pub fn new(url: &str) -> Self {
        Self {
            display_name: truncate_url_for_display(url),
            ..Default::default()
        }
    }
}

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
    /// Per-download progress information for active downloads
    pub active_downloads: Vec<DownloadProgress>,
    pub logs: Vec<String>,
    pub concurrent: usize,
    pub toast: Option<String>,
    pub use_ascii_indicators: bool,
    pub total_retries: usize,
}

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
    /// Active downloads with their progress information
    active_downloads: HashMap<String, DownloadProgress>,
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

    /// Updates progress information for an active download.
    UpdateDownloadProgress {
        url: String,
        progress: DownloadProgress,
    },
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

    /// Refresh the download timestamp for all active downloads to dismiss stale indicators
    pub fn refresh_all_download_timestamps(&self) -> Result<()> {
        let mut queues = self.queues.lock()?;
        for progress in queues.active_downloads.values_mut() {
            progress.last_update = Instant::now();
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
    pub fn get_active_downloads(&self) -> Result<HashSet<String>> {
        let queues = self.queues.lock()?;
        Ok(queues.active_downloads.keys().cloned().collect())
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

    pub fn is_notification_sent(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.notification_sent)
    }

    pub fn set_notification_sent(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.notification_sent = value;
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

    /// Reset the retry counter (called when starting a new download session)
    pub fn reset_retries(&self) -> Result<()> {
        let mut retries = self.total_retries.lock()?;
        *retries = 0;
        Ok(())
    }

    /// Creates a snapshot of all UI-relevant state with minimal locking.
    ///
    /// This method acquires each lock once and captures all necessary state
    /// for UI rendering in a single pass, avoiding the overhead of multiple
    /// lock acquisitions per frame.
    ///
    /// # Returns
    ///
    /// A `UiSnapshot` containing all state needed for UI rendering.
    pub fn get_ui_snapshot(&self) -> Result<UiSnapshot> {
        // Acquire each lock once and extract all needed values
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
        // Convert HashMap values to Vec for UI rendering
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // Helper to wait for message processing
    fn wait_for_processing() {
        thread::sleep(Duration::from_millis(50));
    }

    // ========== Initialization Tests ==========

    #[test]
    fn test_new_creates_default_state() {
        let state = AppState::new();

        // Verify default flags
        assert!(!state.is_paused().unwrap());
        assert!(!state.is_started().unwrap());
        assert!(!state.is_shutdown().unwrap());
        assert!(!state.is_force_quit().unwrap());
        assert!(!state.is_completed().unwrap());
    }

    #[test]
    fn test_new_creates_empty_queue() {
        let state = AppState::new();
        let queue = state.get_queue().unwrap();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_new_has_welcome_logs() {
        let state = AppState::new();
        let snapshot = state.get_ui_snapshot().unwrap();

        assert!(snapshot.logs.len() >= 2);
        assert!(snapshot.logs[0].contains("Welcome"));
        assert!(snapshot.logs[1].contains("quit"));
    }

    #[test]
    fn test_new_loads_settings() {
        let state = AppState::new();
        let settings = state.get_settings().unwrap();
        // Settings should load (either from file or default)
        assert!(settings.concurrent_downloads > 0);
    }

    // ========== Queue Operations Tests ==========

    #[test]
    fn test_pop_queue_empty() {
        let state = AppState::new();
        let result = state.pop_queue().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_pop_queue_returns_front() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
                "url3".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let first = state.pop_queue().unwrap();
        assert_eq!(first, Some("url1".to_string()));

        let second = state.pop_queue().unwrap();
        assert_eq!(second, Some("url2".to_string()));
    }

    #[test]
    fn test_get_queue_returns_copy() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let queue = state.get_queue().unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0], "url1");
        assert_eq!(queue[1], "url2");
    }

    #[test]
    fn test_remove_from_queue_valid_index() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
                "url3".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let removed = state.remove_from_queue(1).unwrap();
        assert_eq!(removed, Some("url2".to_string()));

        let queue = state.get_queue().unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0], "url1");
        assert_eq!(queue[1], "url3");
    }

    #[test]
    fn test_remove_from_queue_invalid_index() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec!["url1".to_string()]))
            .unwrap();
        wait_for_processing();

        let removed = state.remove_from_queue(5).unwrap();
        assert!(removed.is_none());
    }

    #[test]
    fn test_swap_queue_items_valid() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
                "url3".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let success = state.swap_queue_items(0, 2).unwrap();
        assert!(success);

        let queue = state.get_queue().unwrap();
        assert_eq!(queue[0], "url3");
        assert_eq!(queue[2], "url1");
    }

    #[test]
    fn test_swap_queue_items_same_index() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let success = state.swap_queue_items(0, 0).unwrap();
        assert!(!success);
    }

    #[test]
    fn test_swap_queue_items_invalid_index() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec!["url1".to_string()]))
            .unwrap();
        wait_for_processing();

        let success = state.swap_queue_items(0, 5).unwrap();
        assert!(!success);
    }

    // ========== Flag Mutations Tests ==========

    #[test]
    fn test_set_paused() {
        let state = AppState::new();
        state.send(StateMessage::SetPaused(true)).unwrap();
        wait_for_processing();
        assert!(state.is_paused().unwrap());

        state.send(StateMessage::SetPaused(false)).unwrap();
        wait_for_processing();
        assert!(!state.is_paused().unwrap());
    }

    #[test]
    fn test_set_started() {
        let state = AppState::new();
        state.send(StateMessage::SetStarted(true)).unwrap();
        wait_for_processing();
        assert!(state.is_started().unwrap());

        state.send(StateMessage::SetStarted(false)).unwrap();
        wait_for_processing();
        assert!(!state.is_started().unwrap());
    }

    #[test]
    fn test_set_shutdown() {
        let state = AppState::new();
        state.send(StateMessage::SetShutdown(true)).unwrap();
        wait_for_processing();
        assert!(state.is_shutdown().unwrap());
    }

    #[test]
    fn test_set_force_quit() {
        let state = AppState::new();
        state.send(StateMessage::SetForceQuit(true)).unwrap();
        wait_for_processing();
        assert!(state.is_force_quit().unwrap());
    }

    #[test]
    fn test_set_completed() {
        let state = AppState::new();
        state.send(StateMessage::SetCompleted(true)).unwrap();
        wait_for_processing();
        assert!(state.is_completed().unwrap());
    }

    // ========== Progress Tracking Tests ==========

    #[test]
    fn test_increment_completed() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
            ]))
            .unwrap();
        state.send(StateMessage::SetStarted(true)).unwrap();
        wait_for_processing();

        state.send(StateMessage::IncrementCompleted).unwrap();
        wait_for_processing();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.completed_tasks, 1);
        assert!((snapshot.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_update_progress_calculation() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
                "url3".to_string(),
                "url4".to_string(),
            ]))
            .unwrap();
        state.send(StateMessage::SetStarted(true)).unwrap();
        wait_for_processing();

        // Complete 2 out of 4 tasks
        state.send(StateMessage::IncrementCompleted).unwrap();
        state.send(StateMessage::IncrementCompleted).unwrap();
        wait_for_processing();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!((snapshot.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_auto_completion_detection() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
            ]))
            .unwrap();
        state.send(StateMessage::SetStarted(true)).unwrap();
        wait_for_processing();

        // Complete all tasks
        state.send(StateMessage::IncrementCompleted).unwrap();
        state.send(StateMessage::IncrementCompleted).unwrap();
        wait_for_processing();

        // Give extra time for auto-completion to be detected
        thread::sleep(Duration::from_millis(100));

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!(snapshot.completed);
        assert!((snapshot.progress - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_progress_zero_when_no_tasks() {
        let state = AppState::new();
        state.send(StateMessage::UpdateProgress).unwrap();
        wait_for_processing();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!((snapshot.progress - 0.0).abs() < 0.01);
    }

    // ========== Active Downloads Tests ==========

    #[test]
    fn test_add_active_download() {
        let state = AppState::new();
        state
            .send(StateMessage::AddActiveDownload(
                "https://example.com/video".to_string(),
            ))
            .unwrap();
        wait_for_processing();

        let active = state.get_active_downloads().unwrap();
        assert!(active.contains("https://example.com/video"));
    }

    #[test]
    fn test_remove_active_download() {
        let state = AppState::new();
        state
            .send(StateMessage::AddActiveDownload(
                "https://example.com/video".to_string(),
            ))
            .unwrap();
        wait_for_processing();

        state
            .send(StateMessage::RemoveActiveDownload(
                "https://example.com/video".to_string(),
            ))
            .unwrap();
        wait_for_processing();

        let active = state.get_active_downloads().unwrap();
        assert!(!active.contains("https://example.com/video"));
    }

    #[test]
    fn test_update_download_progress() {
        let state = AppState::new();
        let url = "https://youtube.com/watch?v=abc123".to_string();
        state
            .send(StateMessage::AddActiveDownload(url.clone()))
            .unwrap();
        wait_for_processing();

        let progress = DownloadProgress {
            display_name: "Test Video".to_string(),
            phase: "downloading".to_string(),
            percent: 50.0,
            speed: Some("1.5MiB/s".to_string()),
            eta: Some("00:02:30".to_string()),
            downloaded_bytes: Some(1024 * 1024),
            total_bytes: Some(2 * 1024 * 1024),
            fragment_index: None,
            fragment_count: None,
            last_update: Instant::now(),
        };

        state
            .send(StateMessage::UpdateDownloadProgress {
                url: url.clone(),
                progress,
            })
            .unwrap();
        wait_for_processing();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.active_downloads.len(), 1);

        let download = &snapshot.active_downloads[0];
        assert_eq!(download.display_name, "Test Video");
        assert!((download.percent - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_multiple_active_downloads() {
        let state = AppState::new();
        state
            .send(StateMessage::AddActiveDownload("url1".to_string()))
            .unwrap();
        state
            .send(StateMessage::AddActiveDownload("url2".to_string()))
            .unwrap();
        state
            .send(StateMessage::AddActiveDownload("url3".to_string()))
            .unwrap();
        wait_for_processing();

        let active = state.get_active_downloads().unwrap();
        assert_eq!(active.len(), 3);
    }

    // ========== Log Management Tests ==========

    #[test]
    fn test_add_log() {
        let state = AppState::new();
        state.add_log("Test log message".to_string()).unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!(
            snapshot
                .logs
                .iter()
                .any(|log| log.contains("Test log message"))
        );
    }

    #[test]
    fn test_log_limit_1000() {
        let state = AppState::new();

        // Add 1100 logs
        for i in 0..1100 {
            state.add_log(format!("Log message {}", i)).unwrap();
        }

        let snapshot = state.get_ui_snapshot().unwrap();
        // Should have at most 1000 logs
        assert!(snapshot.logs.len() <= 1000);

        // Oldest logs should be removed (logs 0-99 and welcome messages gone)
        assert!(
            !snapshot
                .logs
                .iter()
                .any(|log| log.contains("Log message 0"))
        );

        // Recent logs should still be there
        assert!(
            snapshot
                .logs
                .iter()
                .any(|log| log.contains("Log message 1099"))
        );
    }

    #[test]
    fn test_clear_logs() {
        let state = AppState::new();
        state.add_log("Test log 1".to_string()).unwrap();
        state.add_log("Test log 2".to_string()).unwrap();
        state.clear_logs().unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.logs.len(), 1);
        assert!(snapshot.logs[0].contains("Logs cleared"));
    }

    #[test]
    fn test_log_error() {
        let state = AppState::new();
        state.log_error("Download", "Connection timeout").unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!(snapshot.logs.iter().any(|log| {
            log.contains("[ERROR]")
                && log.contains("Download")
                && log.contains("Connection timeout")
        }));
    }

    // ========== Toast Notification Tests ==========

    #[test]
    fn test_show_toast() {
        let state = AppState::new();
        state.show_toast("Test notification").unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.toast, Some("Test notification".to_string()));
    }

    #[test]
    fn test_show_toast_accepts_string() {
        let state = AppState::new();
        state.show_toast(String::from("String toast")).unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.toast, Some("String toast".to_string()));
    }

    #[test]
    fn test_clear_toast() {
        let state = AppState::new();
        state.show_toast("Test notification").unwrap();
        state.clear_toast().unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!(snapshot.toast.is_none());
    }

    #[test]
    fn test_toast_auto_expiry() {
        let state = AppState::new();

        // Directly set toast with old timestamp
        {
            let mut toast = state.toast.lock().unwrap();
            *toast = Some((
                "Old toast".to_string(),
                Instant::now() - Duration::from_secs(5),
            ));
        }

        // get_ui_snapshot should clear expired toast
        let snapshot = state.get_ui_snapshot().unwrap();
        assert!(snapshot.toast.is_none());
    }

    // ========== Settings Tests ==========

    #[test]
    fn test_get_settings() {
        let state = AppState::new();
        let settings = state.get_settings().unwrap();
        // Should have valid default settings
        assert!(settings.concurrent_downloads > 0);
    }

    #[test]
    fn test_update_settings() {
        let state = AppState::new();
        let mut new_settings = state.get_settings().unwrap();
        new_settings.concurrent_downloads = 8;
        new_settings.write_subtitles = true;

        state.update_settings(new_settings).unwrap();

        let updated = state.get_settings().unwrap();
        assert_eq!(updated.concurrent_downloads, 8);
        assert!(updated.write_subtitles);
    }

    // ========== UI Snapshot Tests ==========

    #[test]
    fn test_ui_snapshot_captures_all_state() {
        let state = AppState::new();

        // Set up various state
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
            ]))
            .unwrap();
        state.send(StateMessage::SetStarted(true)).unwrap();
        state.send(StateMessage::SetPaused(true)).unwrap();
        state
            .send(StateMessage::AddActiveDownload("url1".to_string()))
            .unwrap();
        wait_for_processing();

        state.add_log("Test log".to_string()).unwrap();
        state.show_toast("Test toast").unwrap();
        state.set_concurrent(6).unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();

        assert!(snapshot.started);
        assert!(snapshot.paused);
        assert!(!snapshot.completed);
        assert_eq!(snapshot.queue.len(), 2);
        assert_eq!(snapshot.active_downloads.len(), 1);
        assert!(snapshot.logs.iter().any(|l| l.contains("Test log")));
        assert_eq!(snapshot.toast, Some("Test toast".to_string()));
        assert_eq!(snapshot.concurrent, 6);
        assert_eq!(snapshot.initial_total_tasks, 2);
    }

    #[test]
    fn test_ui_snapshot_includes_retry_count() {
        let state = AppState::new();
        state.increment_retries().unwrap();
        state.increment_retries().unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.total_retries, 2);
    }

    // ========== Concurrent Access Tests ==========

    #[test]
    fn test_concurrent_get_settings() {
        let state = AppState::new();
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let state_clone = state.clone();
                thread::spawn(move || {
                    for _ in 0..100 {
                        let _ = state_clone.get_settings().unwrap();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_flag_access() {
        let state = AppState::new();
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let state_clone = state.clone();
                thread::spawn(move || {
                    for _ in 0..50 {
                        state_clone
                            .send(StateMessage::SetPaused(i % 2 == 0))
                            .unwrap();
                        let _ = state_clone.is_paused().unwrap();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_queue_access() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
                "url3".to_string(),
                "url4".to_string(),
                "url5".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let handles: Vec<_> = (0..3)
            .map(|_| {
                let state_clone = state.clone();
                thread::spawn(move || {
                    let _ = state_clone.pop_queue().unwrap();
                    let _ = state_clone.get_queue().unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_log_writes() {
        let state = AppState::new();
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let state_clone = state.clone();
                thread::spawn(move || {
                    for j in 0..20 {
                        state_clone
                            .add_log(format!("Thread {} log {}", i, j))
                            .unwrap();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = state.get_ui_snapshot().unwrap();
        // All 100 logs should be present (5 threads x 20 logs + 2 welcome logs)
        assert!(snapshot.logs.len() >= 100);
    }

    // ========== Reset and File Lock Tests ==========

    #[test]
    fn test_reset_for_new_run() {
        let state = AppState::new();

        // Set up various state
        state.send(StateMessage::SetStarted(true)).unwrap();
        state.send(StateMessage::SetPaused(true)).unwrap();
        state.send(StateMessage::SetCompleted(true)).unwrap();
        wait_for_processing();

        state.increment_retries().unwrap();
        state.show_toast("Test").unwrap();

        state.reset_for_new_run().unwrap();

        assert!(!state.is_paused().unwrap());
        assert!(!state.is_started().unwrap());
        assert!(!state.is_completed().unwrap());
        assert!(!state.is_shutdown().unwrap());
        assert!(!state.is_force_quit().unwrap());

        let snapshot = state.get_ui_snapshot().unwrap();
        assert!(snapshot.toast.is_none());
        assert_eq!(snapshot.total_retries, 0);
    }

    #[test]
    fn test_acquire_file_lock() {
        let state = AppState::new();
        let _lock = state.acquire_file_lock().unwrap();
        // Lock is acquired, will be released when _lock drops
    }

    #[test]
    fn test_set_concurrent() {
        let state = AppState::new();
        state.set_concurrent(12).unwrap();
        assert_eq!(state.get_concurrent().unwrap(), 12);
    }

    #[test]
    fn test_increment_and_reset_retries() {
        let state = AppState::new();
        state.increment_retries().unwrap();
        state.increment_retries().unwrap();
        state.increment_retries().unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.total_retries, 3);

        state.reset_retries().unwrap();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.total_retries, 0);
    }

    #[test]
    fn test_refresh_all_download_timestamps() {
        let state = AppState::new();
        state
            .send(StateMessage::AddActiveDownload("url1".to_string()))
            .unwrap();
        state
            .send(StateMessage::AddActiveDownload("url2".to_string()))
            .unwrap();
        wait_for_processing();

        // This should not panic
        state.refresh_all_download_timestamps().unwrap();
    }

    // ========== DownloadProgress Tests ==========

    #[test]
    fn test_download_progress_default() {
        let progress = DownloadProgress::default();
        assert!(progress.display_name.is_empty());
        assert_eq!(progress.phase, "downloading");
        assert!((progress.percent - 0.0).abs() < 0.01);
        assert!(progress.speed.is_none());
        assert!(progress.eta.is_none());
    }

    #[test]
    fn test_download_progress_new_youtube() {
        let progress = DownloadProgress::new("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert!(progress.display_name.contains("dQw4w9WgXcQ"));
    }

    #[test]
    fn test_download_progress_new_other_url() {
        let progress = DownloadProgress::new("https://example.com/video.mp4");
        assert!(progress.display_name.contains("video.mp4"));
    }

    // ========== Load Links Tests ==========

    #[test]
    fn test_load_links_replaces_queue() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec!["old_url".to_string()]))
            .unwrap();
        wait_for_processing();

        state
            .send(StateMessage::LoadLinks(vec![
                "new1".to_string(),
                "new2".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let queue = state.get_queue().unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0], "new1");
        assert_eq!(queue[1], "new2");
    }

    #[test]
    fn test_load_links_updates_stats() {
        let state = AppState::new();
        state
            .send(StateMessage::LoadLinks(vec![
                "url1".to_string(),
                "url2".to_string(),
                "url3".to_string(),
            ]))
            .unwrap();
        wait_for_processing();

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.total_tasks, 3);
        assert_eq!(snapshot.initial_total_tasks, 3);
    }

    // ========== AddToQueue Tests ==========

    #[test]
    fn test_add_to_queue() {
        let state = AppState::new();
        state
            .send(StateMessage::AddToQueue("url1".to_string()))
            .unwrap();
        state
            .send(StateMessage::AddToQueue("url2".to_string()))
            .unwrap();
        wait_for_processing();

        let queue = state.get_queue().unwrap();
        assert_eq!(queue.len(), 2);

        let snapshot = state.get_ui_snapshot().unwrap();
        assert_eq!(snapshot.initial_total_tasks, 2);
    }

    // ========== Notification Flag Tests ==========

    #[test]
    fn test_notification_not_sent_initially() {
        let state = AppState::new();
        assert!(!state.is_notification_sent().unwrap());
    }

    #[test]
    fn test_notification_can_be_marked_as_sent() {
        let state = AppState::new();
        state.set_notification_sent(true).unwrap();
        assert!(state.is_notification_sent().unwrap());
    }

    #[test]
    fn test_notification_flag_resets_on_new_run() {
        let state = AppState::new();

        // Mark notification as sent
        state.set_notification_sent(true).unwrap();
        assert!(state.is_notification_sent().unwrap());

        // Reset for new run
        state.reset_for_new_run().unwrap();

        // Notification flag should be reset
        assert!(!state.is_notification_sent().unwrap());
    }

    #[test]
    fn test_notification_lifecycle_across_multiple_runs() {
        let state = AppState::new();

        // First run: notification starts unsent
        assert!(!state.is_notification_sent().unwrap());

        // Simulate completion notification being sent
        state.set_notification_sent(true).unwrap();
        assert!(state.is_notification_sent().unwrap());

        // Subsequent checks should still show sent (idempotent)
        assert!(state.is_notification_sent().unwrap());

        // User restarts downloads
        state.reset_for_new_run().unwrap();

        // Second run: notification should be unsent again
        assert!(!state.is_notification_sent().unwrap());

        // Second completion
        state.set_notification_sent(true).unwrap();
        assert!(state.is_notification_sent().unwrap());
    }

    #[test]
    fn test_notification_flag_independent_of_completion_state() {
        let state = AppState::new();

        // Set completed without setting notification
        state.send(StateMessage::SetCompleted(true)).unwrap();
        wait_for_processing();

        // Notification flag should still be false
        assert!(state.is_completed().unwrap());
        assert!(!state.is_notification_sent().unwrap());

        // Set notification independently
        state.set_notification_sent(true).unwrap();
        assert!(state.is_notification_sent().unwrap());
    }
}
