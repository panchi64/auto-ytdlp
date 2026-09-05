use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::utils::settings::Settings;

mod api;
mod dispatch;
mod message;
mod progress;
mod snapshot;

pub use message::StateMessage;
pub use progress::DownloadProgress;
pub use snapshot::UiSnapshot;

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
    /// Whether the download controller thread is alive.
    ///
    /// `started` is cleared by the stop keypress while yt-dlp subprocesses are
    /// still finishing, so it cannot answer "is anything still running?". This
    /// is set before the controller spawns and cleared as it exits.
    controller_active: bool,
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

    /// Serializes file operations to prevent race conditions
    file_lock: Arc<Mutex<()>>,

    toast: Arc<Mutex<Option<(String, Instant)>>>,
    total_retries: Arc<Mutex<usize>>,

    /// URLs that failed to download, for retry with the 'T' key
    failed_downloads: Arc<Mutex<Vec<String>>>,

    tx: Sender<StateMessage>,
    rx: Arc<Mutex<Receiver<StateMessage>>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = channel();

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
            failed_downloads: Arc::new(Mutex::new(Vec::new())),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };

        let state_clone = state.clone();
        std::thread::spawn(move || {
            state_clone.process_messages();
        });

        state
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
