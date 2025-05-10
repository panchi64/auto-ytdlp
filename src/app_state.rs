use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::utils::settings::Settings;

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
    logs: Arc<Mutex<Vec<String>>>,
    concurrent: Arc<Mutex<usize>>,
    settings: Arc<Mutex<Settings>>,

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
            logs: Arc::new(Mutex::new(vec![
                "Welcome! Press 'S' to start downloads".to_string(),
                "Press 'Q' to quit, 'Shift+Q' to force quit".to_string(),
            ])),
            concurrent: Arc::new(Mutex::new(settings.concurrent_downloads)),
            settings: Arc::new(Mutex::new(settings)),
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
            let rx = self.rx.lock().unwrap();
            if let Ok(message) = rx.recv() {
                drop(rx); // Release lock before processing

                match message {
                    StateMessage::AddToQueue(url) => {
                        let mut queues = self.queues.lock().unwrap();
                        queues.queue.push_back(url);

                        // Update stats
                        let mut stats = self.stats.lock().unwrap();
                        stats.total_tasks += 1;
                        stats.initial_total_tasks += 1;
                    }
                    StateMessage::AddActiveDownload(url) => {
                        let mut queues = self.queues.lock().unwrap();
                        queues.active_downloads.insert(url);
                    }
                    StateMessage::RemoveActiveDownload(url) => {
                        let mut queues = self.queues.lock().unwrap();
                        queues.active_downloads.remove(&url);
                    }
                    StateMessage::IncrementCompleted => {
                        let mut stats = self.stats.lock().unwrap();
                        stats.completed_tasks += 1;
                        // Auto-update progress
                        self.tx.send(StateMessage::UpdateProgress).unwrap();
                    }
                    StateMessage::UpdateProgress => {
                        self.update_progress();
                    }
                    StateMessage::SetPaused(value) => {
                        let mut flags = self.flags.lock().unwrap();
                        flags.paused = value;
                    }
                    StateMessage::SetStarted(value) => {
                        let mut flags = self.flags.lock().unwrap();
                        flags.started = value;
                    }
                    StateMessage::SetShutdown(value) => {
                        let mut flags = self.flags.lock().unwrap();
                        flags.shutdown = value;
                    }
                    StateMessage::SetForceQuit(value) => {
                        let mut flags = self.flags.lock().unwrap();
                        flags.force_quit = value;
                    }
                    StateMessage::SetCompleted(value) => {
                        let mut flags = self.flags.lock().unwrap();
                        flags.completed = value;
                    }
                    StateMessage::LoadLinks(links) => {
                        let mut queues = self.queues.lock().unwrap();
                        queues.queue = VecDeque::from(links);

                        let queue_len = queues.queue.len();
                        drop(queues);

                        let mut stats = self.stats.lock().unwrap();
                        stats.total_tasks = queue_len;
                        stats.initial_total_tasks = queue_len;
                    }
                    StateMessage::UpdateSettings(new_settings) => {
                        // Update settings in memory
                        let mut settings = self.settings.lock().unwrap();
                        *settings = new_settings.clone();
                        drop(settings);

                        // Update concurrent downloads
                        let mut concurrent = self.concurrent.lock().unwrap();
                        *concurrent = new_settings.concurrent_downloads;

                        // Save settings to disk
                        if let Err(err) = new_settings.save() {
                            self.add_log(format!("Error saving settings: {}", err));
                        } else {
                            self.add_log("Settings saved successfully".to_string());
                        }
                    }
                }
            } else {
                // Channel closed
                break;
            }
        }
    }

    /// Sends a state update message to the background message processing thread.
    ///
    /// This is the primary method for modifying the application state in a
    /// thread-safe manner.
    ///
    /// # Parameters
    ///
    /// * `message` - The `StateMessage` indicating what state should be updated
    ///
    /// # Example
    ///
    /// ```
    /// state.send(StateMessage::SetPaused(true));
    /// ```
    pub fn send(&self, message: StateMessage) {
        self.tx.send(message).unwrap_or_else(|_| {
            // Handle send error (channel closed)
            self.add_log("Error: State channel closed".to_string());
        });
    }

    /// Adds a log message to the application logs.
    ///
    /// # Parameters
    ///
    /// * `message` - The log message to add
    ///
    /// # Example
    ///
    /// ```
    /// state.add_log("Download started".to_string());
    /// ```
    pub fn add_log(&self, message: String) {
        let mut logs = self.logs.lock().unwrap();
        logs.push(message);
    }

    /// Retrieves all log messages as a vector of strings.
    ///
    /// # Returns
    ///
    /// A clone of the current log messages.
    pub fn get_logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }

    /// Updates the download progress based on completed and total tasks.
    ///
    /// Calculates the percentage of completed downloads and updates the
    /// `completed` flag if all downloads are finished.
    pub fn update_progress(&self) {
        let mut stats = self.stats.lock().unwrap();
        if stats.total_tasks > 0 {
            let progress = (stats.completed_tasks as f64 / stats.total_tasks as f64) * 100.0;
            stats.progress = progress.clamp(0.0, 100.0);

            let is_complete = stats.total_tasks > 0 && stats.completed_tasks == stats.total_tasks;
            drop(stats);

            let mut flags = self.flags.lock().unwrap();
            flags.completed = is_complete;
        }
    }

    /// Removes and returns the next URL from the download queue.
    ///
    /// # Returns
    ///
    /// `Some(String)` containing the next URL to download, or `None` if the queue is empty.
    pub fn pop_queue(&self) -> Option<String> {
        self.queues.lock().unwrap().queue.pop_front()
    }

    /// Returns a copy of the current download queue.
    ///
    /// # Returns
    ///
    /// A clone of the current download queue.
    pub fn get_queue(&self) -> VecDeque<String> {
        self.queues.lock().unwrap().queue.clone()
    }

    /// Returns a copy of the active downloads set.
    ///
    /// # Returns
    ///
    /// A clone of the set of URLs currently being downloaded.
    pub fn get_active_downloads(&self) -> HashSet<String> {
        self.queues.lock().unwrap().active_downloads.clone()
    }

    // Getter methods (mainly to abstract away the Mutex complexity)

    /// Checks if the application is in paused state.
    ///
    /// # Returns
    ///
    /// `true` if downloads are paused, `false` otherwise.
    pub fn is_paused(&self) -> bool {
        self.flags.lock().unwrap().paused
    }

    /// Checks if downloads have been started.
    ///
    /// # Returns
    ///
    /// `true` if the download process has been started, `false` otherwise.
    pub fn is_started(&self) -> bool {
        self.flags.lock().unwrap().started
    }

    /// Checks if all downloads are completed.
    ///
    /// # Returns
    ///
    /// `true` if all downloads are completed, `false` otherwise.
    pub fn is_completed(&self) -> bool {
        self.flags.lock().unwrap().completed
    }

    /// Checks if the application is shutting down.
    ///
    /// # Returns
    ///
    /// `true` if a shutdown has been requested, `false` otherwise.
    pub fn is_shutdown(&self) -> bool {
        self.flags.lock().unwrap().shutdown
    }

    /// Checks if a force quit has been requested.
    ///
    /// # Returns
    ///
    /// `true` if a force quit has been requested, `false` otherwise.
    pub fn is_force_quit(&self) -> bool {
        self.flags.lock().unwrap().force_quit
    }

    /// Gets the current download progress as a percentage.
    ///
    /// # Returns
    ///
    /// A percentage value between 0.0 and 100.0 indicating download progress.
    pub fn get_progress(&self) -> f64 {
        self.stats.lock().unwrap().progress
    }

    /// Gets the number of completed download tasks.
    ///
    /// # Returns
    ///
    /// The count of completed download tasks.
    pub fn get_completed_tasks(&self) -> usize {
        self.stats.lock().unwrap().completed_tasks
    }

    /// Gets the total number of download tasks.
    ///
    /// # Returns
    ///
    /// The total count of download tasks.
    pub fn get_total_tasks(&self) -> usize {
        self.stats.lock().unwrap().total_tasks
    }

    /// Gets the initial total number of tasks from when downloads began.
    ///
    /// # Returns
    ///
    /// The initial count of download tasks.
    pub fn get_initial_total_tasks(&self) -> usize {
        self.stats.lock().unwrap().initial_total_tasks
    }

    /// Gets the maximum number of concurrent downloads.
    ///
    /// # Returns
    ///
    /// The current limit on concurrent downloads.
    pub fn get_concurrent(&self) -> usize {
        *self.concurrent.lock().unwrap()
    }

    /// Sets the maximum number of concurrent downloads.
    ///
    /// # Parameters
    ///
    /// * `value` - The maximum number of concurrent downloads to allow.
    pub fn set_concurrent(&self, value: usize) {
        *self.concurrent.lock().unwrap() = value;
    }

    /// Resets the application state for a new download run.
    ///
    /// Resets progress, flags, and counters while preserving the download queue.
    pub fn reset_for_new_run(&self) {
        let mut flags = self.flags.lock().unwrap();
        flags.shutdown = false;
        flags.paused = false;
        flags.started = true;
        flags.completed = false;
        flags.notification_sent = false;
        drop(flags);

        let mut stats = self.stats.lock().unwrap();
        stats.progress = 0.0;
        stats.completed_tasks = 0;
        drop(stats);

        // Queue length stays the same
    }

    /// Clears all logs except for welcome messages.
    ///
    /// This function resets the log history but keeps the welcome messages
    /// to ensure the user always has basic instructions visible.
    pub fn clear_logs(&self) {
        let mut logs = self.logs.lock().unwrap();
        logs.clear();
        logs.push("Welcome! Press 'S' to start downloads".to_string());
        logs.push("Press 'Q' to quit, 'Shift+Q' to force quit".to_string());
    }

    /// Get the current settings
    pub fn get_settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Update the settings
    pub fn update_settings(&self, new_settings: Settings) {
        self.send(StateMessage::UpdateSettings(new_settings));
    }
}
