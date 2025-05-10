use std::{thread, time::Duration};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
};

use super::worker::download_worker;

/// Processes the download queue using multiple worker threads.
///
/// This function is the main orchestrator of the download process. It:
/// 1. Checks if the queue is empty and marks as completed if so
/// 2. Resets application state for a new download run
/// 3. Spawns multiple worker threads (based on configured concurrent downloads)
/// 4. Each worker thread pulls URLs from the queue and processes them
/// 5. Handles pausing, shutdown, and force quit conditions
/// 6. Waits for all worker threads to complete
/// 7. Updates application state and logs completion status
///
/// # Parameters
///
/// * `state` - The application state containing the download queue
/// * `args` - Command line arguments with download configuration
///
/// # Example
///
/// ```
/// // Start processing the download queue in a separate thread
/// let state_clone = state.clone();
/// let args_clone = args.clone();
/// thread::spawn(move || process_queue(state_clone, args_clone));
/// ```
///
/// # Notes
///
/// Each worker thread will continue running until one of these conditions is met:
/// - The queue is empty AND there are no active downloads
/// - The application is shutting down
/// - A force quit is requested
///
/// Workers will pause processing (but not exit) when the pause flag is set.
pub fn process_queue(state: AppState, args: Args) {
    if state.get_queue().is_empty() {
        state.send(StateMessage::SetCompleted(true));
        return;
    }

    state.reset_for_new_run();

    let mut handles = vec![];
    let concurrent_count = state.get_concurrent();

    // Create worker threads
    for _ in 0..concurrent_count {
        let state_clone = state.clone();
        let args_clone = args.clone();

        let handle = thread::spawn(move || {
            loop {
                if state_clone.is_force_quit() || state_clone.is_shutdown() {
                    break;
                }

                if state_clone.is_paused() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Get next URL from queue
                if let Some(url) = state_clone.pop_queue() {
                    download_worker(url, state_clone.clone(), args_clone.clone());
                } else {
                    thread::sleep(Duration::from_millis(100));

                    if state_clone.get_queue().is_empty()
                        && state_clone.get_active_downloads().is_empty()
                    {
                        // Only break if we're truly done and not just between tasks
                        break;
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let queue_empty = state.get_queue().is_empty();
    state.send(StateMessage::SetCompleted(queue_empty));
    state.send(StateMessage::SetStarted(false));

    if queue_empty {
        state.add_log("All downloads completed".to_string());
    } else {
        state.add_log("Download processing stopped".to_string());
    }

    // Clear logs after a short delay to allow the completion message to be seen
    let state_clone = state.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        state_clone.clear_logs();
    });
}
