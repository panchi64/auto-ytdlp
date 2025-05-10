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
                // If force_quit is set, we want to exit the controller loop immediately.
                // Worker threads also check this flag and should start terminating.
                // The download_worker itself is modified to exit quickly on force_quit.
                if state_clone.is_force_quit() {
                    state_clone
                        .add_log("Controller: Force quit detected, exiting main loop.".to_string());
                }
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

        // After controller loop exits (due to completion, shutdown, or force_quit)

        if state_clone.is_force_quit() {
            state_clone.add_log(
                "Controller: Force quit active. Not waiting for worker threads to join."
                    .to_string(),
            );
            // Worker threads are expected to terminate themselves upon detecting is_force_quit().
            // The download_worker function is also modified to not block on cmd.wait() during a force quit.
            // Thus, we don't join worker_handles here to ensure a fast exit.
        } else {
            // If not a force quit (i.e., normal completion or graceful shutdown), wait for workers.
            state_clone.add_log("Controller: Waiting for worker threads to complete.".to_string());
            for handle in worker_handles {
                if let Err(e) = handle.join() {
                    state_clone.add_log(format!("Controller: Worker thread panicked: {:?}", e));
                }
            }
            state_clone.add_log("Controller: All worker threads completed.".to_string());
        }

        let queue_empty = state_clone.get_queue().is_empty();
        let active_downloads_empty = state_clone.get_active_downloads().is_empty();

        // Update final status based on whether it was a force quit or not
        if state_clone.is_force_quit() {
            state_clone.add_log("Download processing forcefully stopped.".to_string());
            // Do not set SetCompleted(true) on force quit, even if queue became empty by chance.
            // The state should reflect an interruption.
        } else if queue_empty && active_downloads_empty {
            state_clone.send(StateMessage::SetCompleted(true));
            state_clone.add_log("All downloads completed or queue is empty.".to_string());
        } else {
            // This case covers normal stop (shutdown flag) where queue might not be empty.
            state_clone.add_log("Download processing stopped.".to_string());
        }

        state_clone.send(StateMessage::SetStarted(false)); // Always mark as not started

        // Clear logs after a short delay, but only if not a force quit.
        // For force quit, we want to preserve the logs detailing the forceful termination.
        let mut log_clear_handle: Option<thread::JoinHandle<()>> = None;

        if !state_clone.is_force_quit() {
            let final_state_clone = state_clone.clone();
            log_clear_handle = Some(thread::spawn(move || {
                thread::sleep(Duration::from_secs(2));
                // Check again in case state changed, though unlikely for a detached thread task like this.
                if !final_state_clone.is_completed() && !final_state_clone.is_shutdown() {
                    // If not completed and not a normal shutdown, maybe don't clear logs?
                    // For now, let's stick to original logic: clear logs if not force_quit.
                    // The original logic was to clear logs anyway after a delay.
                }
                final_state_clone.add_log("Clearing logs after completion/stop.".to_string()); // Log before clear
                final_state_clone.clear_logs();
            }));
        }

        if let Some(handle) = log_clear_handle {
            if let Err(e) = handle.join() {
                state_clone.add_log(format!(
                    "Log clearing thread panicked: {:?}. Logs may not be cleared.",
                    e
                ));
            }
        }
    });

    // This join is for the controller thread itself.
    // If force_quit is true, the controller thread should now exit quickly because it
    // doesn't .join() its own worker_handles.
    if let Err(e) = controller.join() {
        // Log controller panic, this might be important especially in --auto mode.
        // Using eprintln as AppState might not be available or reliable if controller panicked badly.
        eprintln!("FATAL: Controller thread panicked: {:?}", e);
        // Optionally, could try to use state.add_log if it's a soft panic.
    }
}
