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
/// 3. Creates a controller thread to monitor the queue
/// 4. Creates worker threads only when downloads are ready to start
/// 5. Each worker thread pulls URLs from the queue and processes them
/// 6. Handles pausing, shutdown, and force quit conditions
/// 7. Waits for all worker threads to complete
/// 8. Updates application state and logs completion status
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

    // Create a single controller thread instead of immediately creating all worker threads
    let state_clone = state.clone();
    let args_clone = args.clone();

    let controller = thread::spawn(move || {
        let mut worker_handles = vec![];
        let mut workers_created = false;

        loop {
            if state_clone.is_force_quit() || state_clone.is_shutdown() {
                break;
            }

            if state_clone.is_paused() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            // Check if we need to start processing and haven't created workers yet
            if !workers_created && !state_clone.get_queue().is_empty() {
                // Create worker threads only when we're about to start processing
                let concurrent_count = state_clone.get_concurrent();
                workers_created = true;

                for _ in 0..concurrent_count {
                    let worker_state = state_clone.clone();
                    let worker_args = args_clone.clone();

                    let handle = thread::spawn(move || {
                        loop {
                            if worker_state.is_force_quit() || worker_state.is_shutdown() {
                                break;
                            }

                            if worker_state.is_paused() {
                                thread::sleep(Duration::from_millis(100));
                                continue;
                            }

                            // Get next URL from queue
                            if let Some(url) = worker_state.pop_queue() {
                                download_worker(url, worker_state.clone(), worker_args.clone());
                            } else {
                                thread::sleep(Duration::from_millis(100));

                                if worker_state.get_queue().is_empty()
                                    && worker_state.get_active_downloads().is_empty()
                                {
                                    // Only break if we're truly done and not just between tasks
                                    break;
                                }
                            }
                        }
                    });
                    worker_handles.push(handle);
                }
            }

            // Check if we're done
            if workers_created
                && state_clone.get_queue().is_empty()
                && state_clone.get_active_downloads().is_empty()
            {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }

        // Wait for all worker threads to complete
        for handle in worker_handles {
            handle.join().unwrap();
        }

        let queue_empty = state_clone.get_queue().is_empty();
        state_clone.send(StateMessage::SetCompleted(queue_empty));
        state_clone.send(StateMessage::SetStarted(false));

        if queue_empty {
            state_clone.add_log("All downloads completed".to_string());
        } else {
            state_clone.add_log("Download processing stopped".to_string());
        }

        // Clear logs after a short delay to allow the completion message to be seen
        let final_state = state_clone.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(2));
            final_state.clear_logs();
        });
    });

    controller.join().unwrap();
}
