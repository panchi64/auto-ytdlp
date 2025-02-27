use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

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

pub enum StateMessage {
    AddToQueue(String),
    RemoveFromQueue(String),
    AddActiveDownload(String),
    RemoveActiveDownload(String),
    IncrementCompleted,
    SetPaused(bool),
    SetStarted(bool),
    SetShutdown(bool),
    SetForceQuit(bool),
    UpdateProgress,
    LoadLinks(Vec<String>),
}

#[derive(Clone)]
pub struct AppState {
    stats: Arc<Mutex<DownloadStats>>,
    queues: Arc<Mutex<DownloadQueues>>,
    flags: Arc<Mutex<AppFlags>>,
    logs: Arc<Mutex<Vec<String>>>,
    concurrent: Arc<Mutex<usize>>,

    // Channel for state updates
    tx: Sender<StateMessage>,
    rx: Arc<Mutex<Receiver<StateMessage>>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        let state = AppState {
            stats: Arc::new(Mutex::new(DownloadStats::default())),
            queues: Arc::new(Mutex::new(DownloadQueues::default())),
            flags: Arc::new(Mutex::new(AppFlags::default())),
            logs: Arc::new(Mutex::new(vec![
                "Welcome! Press 'S' to start downloads".to_string(),
                "Press 'Q' to quit, 'Shift+Q' to force quit".to_string(),
            ])),
            concurrent: Arc::new(Mutex::new(4)), // Default value
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
                    StateMessage::RemoveFromQueue(url) => {
                        let mut queues = self.queues.lock().unwrap();
                        queues.queue.retain(|u| u != &url);
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
                    StateMessage::LoadLinks(links) => {
                        let mut queues = self.queues.lock().unwrap();
                        queues.queue = VecDeque::from(links);

                        let queue_len = queues.queue.len();
                        drop(queues);

                        let mut stats = self.stats.lock().unwrap();
                        stats.total_tasks = queue_len;
                        stats.initial_total_tasks = queue_len;
                    }
                }
            } else {
                // Channel closed
                break;
            }
        }
    }

    // Send a message to update state
    pub fn send(&self, message: StateMessage) {
        self.tx.send(message).unwrap_or_else(|_| {
            // Handle send error (channel closed)
            self.add_log("Error: State channel closed".to_string());
        });
    }

    pub fn add_log(&self, message: String) {
        let mut logs = self.logs.lock().unwrap();
        logs.push(message);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }

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

    pub fn pop_queue(&self) -> Option<String> {
        self.queues.lock().unwrap().queue.pop_front()
    }

    pub fn get_queue(&self) -> VecDeque<String> {
        self.queues.lock().unwrap().queue.clone()
    }

    pub fn get_active_downloads(&self) -> HashSet<String> {
        self.queues.lock().unwrap().active_downloads.clone()
    }

    // Getter methods (mainly to abstract away the Mutex complexity)
    pub fn is_paused(&self) -> bool {
        self.flags.lock().unwrap().paused
    }

    pub fn is_started(&self) -> bool {
        self.flags.lock().unwrap().started
    }

    pub fn is_completed(&self) -> bool {
        self.flags.lock().unwrap().completed
    }

    pub fn is_shutdown(&self) -> bool {
        self.flags.lock().unwrap().shutdown
    }

    pub fn is_force_quit(&self) -> bool {
        self.flags.lock().unwrap().force_quit
    }

    pub fn get_progress(&self) -> f64 {
        self.stats.lock().unwrap().progress
    }

    pub fn get_completed_tasks(&self) -> usize {
        self.stats.lock().unwrap().completed_tasks
    }

    pub fn get_total_tasks(&self) -> usize {
        self.stats.lock().unwrap().total_tasks
    }

    pub fn get_initial_total_tasks(&self) -> usize {
        self.stats.lock().unwrap().initial_total_tasks
    }

    pub fn get_concurrent(&self) -> usize {
        *self.concurrent.lock().unwrap()
    }

    pub fn set_concurrent(&self, value: usize) {
        *self.concurrent.lock().unwrap() = value;
    }

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
}
