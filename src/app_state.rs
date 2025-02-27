use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub queue: Arc<Mutex<VecDeque<String>>>,
    pub active_downloads: Arc<Mutex<HashSet<String>>>,
    pub progress: Arc<Mutex<f64>>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub paused: Arc<Mutex<bool>>,
    pub shutdown: Arc<Mutex<bool>>,
    pub started: Arc<Mutex<bool>>,
    pub force_quit: Arc<Mutex<bool>>,
    pub completed: Arc<Mutex<bool>>,
    pub total_tasks: Arc<Mutex<usize>>,
    pub completed_tasks: Arc<Mutex<usize>>,
    pub notification_sent: Arc<Mutex<bool>>,
    pub initial_total_tasks: Arc<Mutex<usize>>,
    pub concurrent: Arc<Mutex<usize>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_downloads: Arc::new(Mutex::new(HashSet::new())),
            progress: Arc::new(Mutex::new(0.0)),
            logs: Arc::new(Mutex::new(vec![
                "Welcome! Press 'S' to start downloads".to_string(),
                "Press 'Q' to quit, 'Shift+Q' to force quit".to_string(),
            ])),
            paused: Arc::new(Mutex::new(false)),
            shutdown: Arc::new(Mutex::new(false)),
            started: Arc::new(Mutex::new(false)),
            force_quit: Arc::new(Mutex::new(false)),
            completed: Arc::new(Mutex::new(false)),
            total_tasks: Arc::new(Mutex::new(0)),
            completed_tasks: Arc::new(Mutex::new(0)),
            notification_sent: Arc::new(Mutex::new(false)),
            initial_total_tasks: Arc::new(Mutex::new(0)),
            concurrent: Arc::new(Mutex::new(0)),
        }
    }
}

pub fn update_progress(state: &AppState) {
    let total = *state.total_tasks.lock().unwrap();
    let completed = *state.completed_tasks.lock().unwrap();

    let progress = if total > 0 {
        let p = (completed as f64 / total as f64) * 100.0;
        // Clamp between 0-100 to prevent gauge panic
        p.clamp(0.0, 100.0)
    } else {
        0.0
    };

    *state.progress.lock().unwrap() = progress;

    // Update completion state
    let mut completed_state = state.completed.lock().unwrap();
    *completed_state = total > 0 && completed == total;
}
